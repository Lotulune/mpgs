import { defineConfig } from "vitest/config";
import { loadEnv } from "vite";
import react from "@vitejs/plugin-react";

const PACKAGED_API_BASES = new Set([
  "http://127.0.0.1:8080",
  "http://localhost:8080",
]);

// Browser dev server proxies API calls to the local mpgs-server so the web app
// can be developed without CORS friction. The packaged Tauri client talks to the
// server directly (the server keeps an explicit CORS allowlist for that origin).
export default defineConfig(({ command, mode }) => {
  const configuredApiBase = loadEnv(mode, process.cwd(), "VITE_").VITE_MPGS_API_BASE?.replace(
    /\/$/,
    "",
  );
  if (command === "build" && configuredApiBase && !PACKAGED_API_BASES.has(configuredApiBase)) {
    throw new Error(
      `VITE_MPGS_API_BASE=${configuredApiBase} is not allowed by the desktop CSP; ` +
        `use ${[...PACKAGED_API_BASES].join(" or ")}`,
    );
  }

  return {
    plugins: [react()],
    server: {
      port: 5173,
      strictPort: true,
      proxy: {
        "/v1": { target: "http://127.0.0.1:8080", changeOrigin: true },
        "/health": { target: "http://127.0.0.1:8080", changeOrigin: true },
        "/openapi.json": { target: "http://127.0.0.1:8080", changeOrigin: true },
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
