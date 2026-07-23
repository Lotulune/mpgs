// Design-system button: wraps the .btn token styles. Variants map to the
// modifier classes in base.css (.btn.primary / .ghost / .danger / .accent, .btn.small).

import type { ButtonHTMLAttributes, Ref } from "react";

type ButtonVariant = "primary" | "ghost" | "danger" | "accent";

export function Button({
  variant,
  size,
  className,
  type = "button",
  ref,
  ...rest
}: ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: ButtonVariant;
  size?: "small";
  ref?: Ref<HTMLButtonElement>;
}) {
  const classes = ["btn", variant, size, className].filter(Boolean).join(" ");
  return <button ref={ref} type={type} className={classes} {...rest} />;
}
