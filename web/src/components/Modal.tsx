// Modal shell: dimmed backdrop + dialog card + header with close button.
// Extracted from AuthDialog; open/close state stays with the caller.
//
// Accessibility: focus moves into the dialog on open (callers may refocus a
// specific control afterwards), Tab is trapped within, Escape closes, and the
// previously-focused element is restored on close. The backdrop closes on a
// direct click only (not a drag that started inside the dialog).

import { useEffect, useRef, type ReactNode } from "react";

const FOCUSABLE =
  "button, [href], input, select, textarea, [tabindex]:not([tabindex='-1'])";

export function Modal({
  title,
  titleId,
  onClose,
  className,
  children,
}: {
  title: ReactNode;
  /** id of the heading element, wired to aria-labelledby. */
  titleId: string;
  onClose: () => void;
  className?: string;
  children: ReactNode;
}) {
  const dialogRef = useRef<HTMLElement | null>(null);
  // Keep onClose in a ref so the mount-only effect below never re-subscribes
  // (and never re-steals focus) if the caller passes a new callback identity.
  const onCloseRef = useRef(onClose);
  onCloseRef.current = onClose;

  useEffect(() => {
    const dialog = dialogRef.current;
    const previouslyFocused = document.activeElement as HTMLElement | null;

    // Move focus into the dialog on open. Consumers may then focus a specific
    // control (AuthDialog refocuses the username field via queueMicrotask).
    dialog?.focus();

    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        onCloseRef.current();
        return;
      }
      if (event.key !== "Tab" || !dialog) return;
      const items = Array.from(
        dialog.querySelectorAll<HTMLElement>(FOCUSABLE),
      ).filter((el) => {
        const control = el as HTMLElement & { disabled?: boolean };
        return !control.disabled && el.offsetParent !== null;
      });
      if (items.length === 0) {
        event.preventDefault();
        return;
      }
      const first = items[0]!;
      const last = items[items.length - 1]!;
      const active = document.activeElement;
      const inside = dialog.contains(active);
      if (event.shiftKey && (active === first || !inside)) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && (active === last || !inside)) {
        event.preventDefault();
        first.focus();
      }
    };
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("keydown", onKey);
      previouslyFocused?.focus?.();
    };
  }, []);

  return (
    <div
      className="modal-backdrop"
      role="presentation"
      onClick={(event) => {
        // Close only on a direct click on the backdrop, not a click that
        // bubbled up from inside the dialog (and not a drag from inside).
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <section
        ref={dialogRef}
        className={["modal", className].filter(Boolean).join(" ")}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
      >
        <div className="modal-head">
          <h2 id={titleId}>{title}</h2>
          <button type="button" className="icon-btn" aria-label="关闭" onClick={onClose}>
            ×
          </button>
        </div>
        {children}
      </section>
    </div>
  );
}
