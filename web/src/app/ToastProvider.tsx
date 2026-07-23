// Toast notifications with optional undo action.

import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { Button } from "../components/Button";

export interface Toast {
  id: number;
  text: string;
  actionLabel?: string;
  onAction?: () => void;
}

interface ToastContextValue {
  show: (text: string, action?: { label: string; run: () => void }) => void;
}

const ToastContext = createContext<ToastContextValue | null>(null);

export function useToast(): ToastContextValue {
  const value = useContext(ToastContext);
  if (!value) throw new Error("useToast outside ToastProvider");
  return value;
}

const TOAST_TTL_MS = 5000;

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const nextId = useRef(1);

  const dismiss = useCallback((id: number) => {
    setToasts((current) => current.filter((t) => t.id !== id));
  }, []);

  const show = useCallback(
    (text: string, action?: { label: string; run: () => void }) => {
      const id = nextId.current;
      nextId.current += 1;
      const toast: Toast = { id, text };
      if (action) {
        toast.actionLabel = action.label;
        toast.onAction = action.run;
      }
      setToasts((current) => [...current.slice(-2), toast]);
      window.setTimeout(() => dismiss(id), TOAST_TTL_MS);
    },
    [dismiss],
  );

  const value = useMemo(() => ({ show }), [show]);

  return (
    <ToastContext.Provider value={value}>
      {children}
      <div className="toast-stack" role="status" aria-live="polite">
        {toasts.map((toast) => (
          <div key={toast.id} className="toast">
            <span>{toast.text}</span>
            {toast.actionLabel && (
              <Button
                size="small"
                onClick={() => {
                  toast.onAction?.();
                  dismiss(toast.id);
                }}
              >
                {toast.actionLabel}
              </Button>
            )}
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}
