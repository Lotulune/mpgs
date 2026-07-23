import { act } from "react";
import { createRoot } from "react-dom/client";
import { describe, expect, it, vi } from "vitest";
import { Modal } from "../src/components/Modal";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("Modal", () => {
  it("closes on Escape without forwarding the key to the underlying screen", () => {
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const onClose = vi.fn();
    const underlyingKeyHandler = vi.fn();
    window.addEventListener("keydown", underlyingKeyHandler);

    act(() => {
      root.render(
        <Modal title="登录" titleId="test-modal-title" onClose={onClose}>
          <button type="button">继续</button>
        </Modal>,
      );
    });

    const dialog = host.querySelector<HTMLElement>("[role='dialog']")!;
    act(() => {
      dialog.dispatchEvent(
        new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }),
      );
    });

    expect(onClose).toHaveBeenCalledOnce();
    expect(underlyingKeyHandler).not.toHaveBeenCalled();

    window.removeEventListener("keydown", underlyingKeyHandler);
    act(() => root.unmount());
    host.remove();
  });
});
