// Design-system panel: bordered content block (.panel) with an optional h4
// title. Use `as` when the panel must be a section or a form.

import type { HTMLAttributes, ReactNode } from "react";

export function Panel({
  as: Tag = "div",
  title,
  className,
  children,
  ...rest
}: HTMLAttributes<HTMLElement> & {
  as?: "div" | "section" | "form";
  title?: ReactNode;
}) {
  const classes = ["panel", className].filter(Boolean).join(" ");
  return (
    <Tag className={classes} {...rest}>
      {title !== undefined && title !== null && <h4>{title}</h4>}
      {children}
    </Tag>
  );
}
