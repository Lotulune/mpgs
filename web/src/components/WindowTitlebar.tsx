import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

function runWindowAction(action: () => Promise<void>): void {
  void action().catch((error: unknown) => {
    console.error("window action failed", error);
  });
}

export function WindowControls({
  floating = false,
  elevated = false,
}: {
  /** Fixed top-right controls + full-width drag strip (onboarding). */
  floating?: boolean;
  /** Fixed top-right controls only, above modals (shell topbar). */
  elevated?: boolean;
}) {
  if (!isTauri()) return null;

  const appWindow = getCurrentWindow();
  const className = [
    "window-controls",
    floating ? "floating" : "",
    elevated && !floating ? "elevated" : "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <>
      {floating && <div className="window-drag-overlay" data-tauri-drag-region aria-hidden="true" />}
      <div className={className} aria-label="窗口控制">
        <button
          type="button"
          className="window-control"
          aria-label="最小化窗口"
          title="最小化"
          onClick={() => runWindowAction(() => appWindow.minimize())}
        >
          <span aria-hidden="true">−</span>
        </button>
        <button
          type="button"
          className="window-control"
          aria-label="最大化或还原窗口"
          title="最大化或还原"
          onClick={() => runWindowAction(() => appWindow.toggleMaximize())}
        >
          <span aria-hidden="true">□</span>
        </button>
        <button
          type="button"
          className="window-control close"
          aria-label="关闭窗口"
          title="关闭"
          onClick={() => runWindowAction(() => appWindow.close())}
        >
          <span aria-hidden="true">×</span>
        </button>
      </div>
    </>
  );
}
