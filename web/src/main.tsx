import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { loadSavedTheme } from "./theme/registry";
import "./styles/base.css";
import "./styles/themes.css";

// Apply the saved theme attribute before first paint to avoid a flash of the
// default skin. ThemeProvider re-applies it (plus procedural textures/FX) on mount.
document.documentElement.dataset.theme = loadSavedTheme() ?? "steam";

const container = document.getElementById("root");
if (!container) {
  throw new Error("missing #root element");
}

createRoot(container).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
