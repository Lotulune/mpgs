import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { initializeClientStorage } from "./api/storage";
import "./styles/base.css";
import "./styles/themes.css";

const container = document.getElementById("root");
if (!container) {
  throw new Error("missing #root element");
}

async function bootstrap(): Promise<void> {
  await initializeClientStorage();
  const [{ App }, { loadSavedTheme }] = await Promise.all([
    import("./App"),
    import("./theme/registry"),
  ]);

  // Apply the saved theme before mounting. ThemeProvider then installs FX.
  document.documentElement.dataset.theme = loadSavedTheme() ?? "steam";
  createRoot(container as HTMLElement).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}

void bootstrap().catch((error: unknown) => {
  container.textContent = `客户端存储初始化失败：${error instanceof Error ? error.message : String(error)}`;
});
