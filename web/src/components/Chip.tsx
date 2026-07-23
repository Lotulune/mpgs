// Design-system chip: small status/meta pill (.chip with optional tone).

import type { HTMLAttributes } from "react";

type ChipTone = "ok" | "warn" | "danger" | "accent";

export function Chip({
  tone,
  className,
  ...rest
}: HTMLAttributes<HTMLSpanElement> & { tone?: ChipTone }) {
  const classes = ["chip", tone, className].filter(Boolean).join(" ");
  return <span className={classes} {...rest} />;
}
