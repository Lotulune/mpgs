import { isTauri } from "@tauri-apps/api/core";
import { useState } from "react";
import { isOnboarded } from "./app/runtime";
import { ThemeProvider } from "./app/ThemeProvider";
import { ToastProvider } from "./app/ToastProvider";
import { OnboardingScreen } from "./screens/OnboardingScreen";
import { Shell } from "./screens/Shell";
import { WindowControls } from "./components/WindowTitlebar";

export function App() {
  const [onboarded, setOnboarded] = useState(isOnboarded);
  const desktop = isTauri();

  return (
    <ThemeProvider>
      <ToastProvider>
        {desktop && <div className="window-frame" aria-hidden="true" />}
        {desktop && !onboarded && <WindowControls floating />}
        {onboarded ? <Shell /> : <OnboardingScreen onDone={() => setOnboarded(true)} />}
      </ToastProvider>
    </ThemeProvider>
  );
}
