import { useState } from "react";
import { isOnboarded } from "./app/runtime";
import { ThemeProvider } from "./app/ThemeProvider";
import { ToastProvider } from "./app/ToastProvider";
import { OnboardingScreen } from "./screens/OnboardingScreen";
import { Shell } from "./screens/Shell";

export function App() {
  const [onboarded, setOnboarded] = useState(isOnboarded);

  return (
    <ThemeProvider>
      <ToastProvider>
        {onboarded ? <Shell /> : <OnboardingScreen onDone={() => setOnboarded(true)} />}
      </ToastProvider>
    </ThemeProvider>
  );
}
