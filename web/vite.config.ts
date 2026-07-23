import { defineConfig } from "vitest/config";
import { loadEnv } from "vite";
import react from "@vitejs/plugin-react";

const PACKAGED_API_BASES = new Set([
  "http://127.0.0.1:8080",
  "http://localhost:8080",
  "http://127.0.0.1:18080",
]);

// Browser dev server proxies API calls to the local mpgs-server so the web app
// can be developed without CORS friction. The packaged Tauri client talks to the
// server directly (the server keeps an explicit CORS allowlist for that origin).
export default defineConfig(({ command, mode }) => {
  const env = loadEnv(mode, process.cwd(), "VITE_");
  const configuredApiBase =
    env.VITE_MPGS_API_BASE?.replace(/\/$/, "") ??
    (mode === "e2e" ? "http://127.0.0.1:18080" : undefined);
  const devApiProxyTarget =
    env.VITE_MPGS_DEV_PROXY_TARGET?.replace(/\/$/, "") ?? "http://127.0.0.1:8080";
  if (command === "build" && configuredApiBase && !PACKAGED_API_BASES.has(configuredApiBase)) {
    throw new Error(
      `VITE_MPGS_API_BASE=${configuredApiBase} is not allowed by the desktop CSP; ` +
        `use ${[...PACKAGED_API_BASES].join(" or ")}`,
    );
  }

  return {
    plugins: [react()],
    define: configuredApiBase
      ? { "import.meta.env.VITE_MPGS_API_BASE": JSON.stringify(configuredApiBase) }
      : undefined,
    server: {
      port: 5173,
      strictPort: true,
      proxy: {
        "/v1": { target: devApiProxyTarget, changeOrigin: true },
        "/health": { target: devApiProxyTarget, changeOrigin: true },
        "/openapi.json": { target: devApiProxyTarget, changeOrigin: true },
      },
    },
    build: {
      target: "es2022",
      sourcemap: false,
    },
    test: {
      environment: "jsdom",
      globals: false,
      include: ["tests/**/*.test.ts", "tests/**/*.test.tsx"],
    },
  };
});
