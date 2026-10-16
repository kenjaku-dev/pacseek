"use client";

import { useEffect, useState } from "react";

/**
 * Observes a 1px sentinel placed immediately before the sticky header.
 * When the sentinel leaves the viewport, the header is stuck — toggles
 * `.is-stuck` as a fallback for browsers without scroll-state queries.
 */
export function useStuckFallback<T extends HTMLElement>() {
  const [stuck, setStuck] = useState(false);
  const [ref, setRef] = useState<T | null>(null);

  useEffect(() => {
    if (!ref || !ref.parentNode) return;

    const sentinel = document.createElement("div");
    sentinel.setAttribute("aria-hidden", "true");
    sentinel.style.cssText =
      "position:absolute;top:0;left:0;width:1px;height:1px;pointer-events:none;opacity:0;";
    ref.parentNode.insertBefore(sentinel, ref);

    const observer = new IntersectionObserver(
      ([entry]) => {
        const isStuck = !entry.isIntersecting;
        setStuck(isStuck);
        ref.classList.toggle("is-stuck", isStuck);
      },
      { threshold: [0, 1] },
    );

    observer.observe(sentinel);
    return () => {
      observer.disconnect();
      sentinel.remove();
    };
  }, [ref]);

  return { ref, stuck, setRef } as const;
}
