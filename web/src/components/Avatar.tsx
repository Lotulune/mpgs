// Avatar image with a first-letter fallback. Renders an <img> when `src` is
// non-empty and loads; otherwise a centered initial span (.avatar-fallback).
// The fallback fills its container, so callers control sizing via the parent
// (same selector pattern as the img). Used by the account menu, profile page
// and community facepile so a broken/empty avatar URL never shows a broken
// image icon.

import { useEffect, useState } from "react";

export function Avatar({
  src,
  name,
  alt,
  className,
}: {
  src: string;
  name: string;
  /** Meaningful alt text for the image. When set, the fallback span is hidden
   *  from AT (the letter is decorative); omit to mark the whole thing hidden. */
  alt?: string;
  className?: string;
}) {
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    setFailed(false);
  }, [src]);

  const initial = name.trim().slice(0, 1).toUpperCase() || "?";
  if (!src || failed) {
    return (
      <span
        className={["avatar-fallback", className].filter(Boolean).join(" ")}
        aria-hidden={alt ? undefined : true}
        role={alt ? "img" : undefined}
        aria-label={alt}
      >
        {initial}
      </span>
    );
  }
  return <img className={className} src={src} alt={alt ?? ""} onError={() => setFailed(true)} />;
}
