"use client";

import { useCallback, useRef } from "react";

/**
 * Tracks the pointer position over an element and exposes it as the
 * `--spot-x` / `--spot-y` custom properties consumed by `.spotlight-card`.
 * Purely decorative — content stays fully visible without it.
 */
export function useSpotlight<T extends HTMLElement>() {
  const ref = useRef<T | null>(null);

  const onPointerMove = useCallback((e: React.PointerEvent<T>) => {
    const el = ref.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    el.style.setProperty("--spot-x", `${e.clientX - rect.left}px`);
    el.style.setProperty("--spot-y", `${e.clientY - rect.top}px`);
  }, []);

  return { ref, onPointerMove } as const;
}
