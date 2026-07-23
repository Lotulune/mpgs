import { spawn } from "node:child_process";
import { closeSync, existsSync, mkdirSync, mkdtempSync, openSync, rmSync, writeFileSync } from "node:fs";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const packageDir = path.resolve(fileURLToPath(new URL(".", import.meta.url)));
const repoRoot = path.resolve(packageDir, "..");
const runtimeRoot = path.join(packageDir, ".runtime");
const runtimeFile = path.join(runtimeRoot, "current.json");
const artifactDir = path.resolve(process.env.MPGS_E2E_ARTIFACT_DIR ?? path.join(packageDir, "artifacts"));
const executableSuffix = process.platform === "win32" ? ".exe" : "";
const seedServerHost = process.env.MPGS_E2E_SERVER_HOST ?? "127.0.0.1";
const seedServerPort = 18080;
const seedServerUrlHost = seedServerHost.includes(":") ? `[${seedServerHost}]` : seedServerHost;
const seedServerOrigin = `http://${seedServerUrlHost}:${seedServerPort}`;

function configuredPath(envName, fallback) {
  const configured = process.env[envName];
  return path.resolve(repoRoot, configured || fallback);
}

const application = configuredPath(
  "MPGS_E2E_APP",
  `apps/desktop/src-tauri/target/debug/lobbytally-desktop${executableSuffix}`,
);
const serverBinary = configuredPath(
  "MPGS_E2E_SERVER",
  `target/debug/mpgs-server${executableSuffix}`,
);
const tauriDriverBinary = process.env.TAURI_DRIVER_BINARY
  ? path.resolve(process.env.TAURI_DRIVER_BINARY)
  : path.join(
      process.env.CARGO_HOME ?? path.join(os.homedir(), ".cargo"),
      "bin",
      `tauri-driver${executableSuffix}`,
    );

let serverProcess;
let serverLogFd;
let driverProcess;
let driverLogFd;
let runtimeDir;

function requireFile(file, label) {
  if (!existsSync(file)) {
    throw new Error(`${label} was not found at ${file}`);
  }
}

async function requirePortAvailable(host, port) {
  await new Promise((resolve, reject) => {
    const probe = net.createServer();
    probe.unref();
    probe.once("error", (error) => {
      reject(new Error(`required E2E port ${port} is unavailable: ${error.message}`));
    });
    probe.listen({ host, port, exclusive: true }, () => {
      probe.close(resolve);
    });
  });
}

function stopProcess(child) {
  if (!child || child.exitCode !== null || child.killed) return;
  try {
    child.kill("SIGTERM");
  } catch (error) {
    if (error?.code !== "ESRCH") throw error;
  }
}

async function waitForHttp(url, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  let lastError;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(url, { signal: AbortSignal.timeout(750) });
      if (response.ok) return;
      lastError = new Error(`HTTP ${response.status}`);
    } catch (error) {
      lastError = error;
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`timed out waiting for ${url}: ${lastError?.message ?? "unknown error"}`);
}

async function waitForProcessHttp(child, url, timeoutMs) {
  let onExit;
  const earlyExit = new Promise((_, reject) => {
    onExit = (code, signal) => {
      reject(new Error(`mpgs-server exited before readiness (code=${code}, signal=${signal})`));
    };
    child.once("exit", onExit);
  });
  try {
    await Promise.race([waitForHttp(url, timeoutMs), earlyExit]);
    await new Promise((resolve, reject) => {
      if (child.exitCode !== null) {
        reject(new Error(`mpgs-server exited after readiness (code=${child.exitCode})`));
        return;
      }
      const onStabilityExit = (code, signal) => {
        clearTimeout(timer);
        reject(new Error(`mpgs-server exited after readiness (code=${code}, signal=${signal})`));
      };
      const timer = setTimeout(() => {
        child.off("exit", onStabilityExit);
        resolve();
      }, 250);
      child.once("exit", onStabilityExit);
    });
  } finally {
    child.off("exit", onExit);
  }
}

async function waitForPort(port, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const connected = await new Promise((resolve) => {
      const socket = net.createConnection({ host: "127.0.0.1", port });
      socket.setTimeout(500);
      socket.once("connect", () => {
        socket.destroy();
        resolve(true);
      });
      const failed = () => {
        socket.destroy();
        resolve(false);
      };
      socket.once("error", failed);
      socket.once("timeout", failed);
    });
    if (connected) return;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`timed out waiting for tauri-driver on port ${port}`);
}

function configureIsolatedDesktopData(dataDir) {
  process.env.MPGS_CLIENT_DATA_DIR = dataDir;
  process.env.WEBVIEW2_USER_DATA_FOLDER = path.join(dataDir, "webview2");
  process.env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=0";
  if (process.platform === "win32") {
    process.env.APPDATA = path.join(dataDir, "roaming");
    process.env.LOCALAPPDATA = path.join(dataDir, "local");
  } else {
    process.env.XDG_DATA_HOME = path.join(dataDir, "data");
    process.env.XDG_CONFIG_HOME = path.join(dataDir, "config");
    process.env.XDG_CACHE_HOME = path.join(dataDir, "cache");
    process.env.TAURI_WEBVIEW_AUTOMATION = "true";
  }
}

function safeName(value) {
  return value.replace(/[^a-z0-9_-]+/gi, "-").replace(/^-|-$/g, "").slice(0, 80) || "failure";
}

export const config = {
  runner: "local",
  host: "127.0.0.1",
  port: 4444,
  specs: ["./specs/**/*.e2e.mjs"],
  maxInstances: 1,
  capabilities: [
    {
      maxInstances: 1,
      "tauri:options": { application },
    },
  ],
  logLevel: "info",
  outputDir: path.join(artifactDir, "wdio"),
  bail: 0,
  waitforTimeout: 15_000,
  connectionRetryTimeout: 120_000,
  connectionRetryCount: 2,
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: {
    ui: "bdd",
    timeout: 180_000,
  },

  onPrepare: async () => {
    requireFile(application, "Tauri application");
    requireFile(serverBinary, "mpgs-server");
    requireFile(tauriDriverBinary, "tauri-driver");
    await requirePortAvailable(seedServerHost, seedServerPort);
    mkdirSync(runtimeRoot, { recursive: true });
    mkdirSync(artifactDir, { recursive: true });
    runtimeDir = mkdtempSync(path.join(os.tmpdir(), "mpgs-e2e-"));
    const desktopDataDir = path.join(runtimeDir, "desktop-data");
    configureIsolatedDesktopData(desktopDataDir);

    serverLogFd = openSync(path.join(artifactDir, "mpgs-server.log"), "w");
    serverProcess = spawn(serverBinary, [], {
      cwd: repoRoot,
      env: {
        ...process.env,
        MPGS_DATABASE_PATH: path.join(runtimeDir, "server.sqlite3"),
        MPGS_SEED_DEMO: "true",
        MPGS_RATE_LIMIT_ENABLED: "false",
        MPGS_BIND_ADDR: `${seedServerUrlHost}:${seedServerPort}`,
      },
      stdio: ["ignore", serverLogFd, serverLogFd],
      windowsHide: true,
    });
    writeFileSync(
      runtimeFile,
      JSON.stringify({
        runtimeDir,
        desktopDataDir,
        serverPid: serverProcess.pid,
        serverHealthUrl: `${seedServerOrigin}/health/live`,
        serverStopped: false,
      }),
      "utf8",
    );
    await waitForProcessHttp(serverProcess, `${seedServerOrigin}/health/ready`, 20_000);
  },

  beforeSession: async (_wdioConfig, capabilities) => {
    const runtime = JSON.parse(await import("node:fs/promises").then(({ readFile }) => readFile(runtimeFile, "utf8")));
    configureIsolatedDesktopData(runtime.desktopDataDir);
    const tauriOptions = capabilities["tauri:options"];
    if (!tauriOptions || typeof tauriOptions !== "object") {
      throw new Error("missing tauri:options capability");
    }
    tauriOptions.webviewOptions = {
      userDataFolder: path.join(runtime.desktopDataDir, "webview2"),
      additionalBrowserArguments: ["remote-debugging-port=0"],
    };
    driverLogFd = openSync(path.join(artifactDir, "tauri-driver.log"), "a");
    driverProcess = spawn(tauriDriverBinary, [], {
      cwd: repoRoot,
      env: process.env,
      stdio: ["ignore", driverLogFd, driverLogFd],
      windowsHide: true,
    });
    await waitForPort(4444, 15_000);
    if (driverProcess.exitCode !== null) {
      throw new Error(`tauri-driver exited early with code ${driverProcess.exitCode}`);
    }
  },

  afterTest: async (test, _context, result) => {
    if (result.passed || !globalThis.browser?.sessionId) return;
    const filename = `${Date.now()}-${safeName(test.title)}.png`;
    await globalThis.browser.saveScreenshot(path.join(artifactDir, filename));
  },

  afterSession: () => {
    stopProcess(driverProcess);
    if (driverLogFd !== undefined) closeSync(driverLogFd);
    driverLogFd = undefined;
  },

  onComplete: (exitCode) => {
    stopProcess(serverProcess);
    if (serverLogFd !== undefined) closeSync(serverLogFd);
    serverLogFd = undefined;
    if (exitCode === 0 && runtimeDir) {
      rmSync(runtimeDir, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
      rmSync(runtimeRoot, { recursive: true, force: true });
    }
  },
};
