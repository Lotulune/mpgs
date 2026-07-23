// Unified empty / error state. The "box" variant renders the dashed
// .state-box with an optional large glyph; "plain" renders the one-line
// .empty-state note. Copy stays at the call sites — honest degradation
// wording (未知 / 日期未定 / 离线快照) must not be hidden by this wrapper.

import type { HTMLAttributes, ReactNode } from "react";

export function EmptyState({
  glyph,
  alert = false,
  variant = "box",
  children,
  ...rest
}: HTMLAttributes<HTMLDivElement> & {
  glyph?: ReactNode;
  /** Renders role="alert" for error states. */
  alert?: boolean;
  variant?: "box" | "plain";
}) {
  if (variant === "plain") {
    return <p className="empty-state">{children}</p>;
  }
  return (
    <div className="state-box" role={alert ? "alert" : undefined} {...rest}>
      {glyph !== undefined && glyph !== null && <span className="big">{glyph}</span>}
      {children}
    </div>
  );
}
