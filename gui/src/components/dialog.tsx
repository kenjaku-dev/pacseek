// ModalDialog — the one true modal primitive: focus moves into the panel on
// open, Tab cycles inside, Esc/backdrop close, keydown stops the window
// hotkeys from double-firing. Extracted from the AC ShortcutsOverlay so the
// confirm dialogs (AD) and shortcuts share the identical a11y behavior.
import { useEffect, useRef, type ReactNode } from "react";

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select, textarea, [tabindex]:not([tabindex="-1"])';

export function ModalDialog({
  label,
  wide,
  children,
  onClose,
}: {
  label: string;
  /** dialogs with journal content get more width */
  wide?: boolean;
  children: ReactNode;
  onClose: () => void;
}) {
  const panel = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = panel.current;
    if (el === null) return;
    const first =
      el.querySelector<HTMLElement>('[data-autofocus]') ??
      el.querySelector<HTMLElement>(FOCUSABLE) ??
      el;
    first.focus({ preventScroll: true });

    const onKey = (e: KeyboardEvent) => {
      e.stopPropagation(); // window hotkeys (Esc-clear etc.) must not double-fire
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
        return;
      }
      if (e.key !== "Tab") return;
      const items = Array.from(el.querySelectorAll<HTMLElement>(FOCUSABLE));
      if (items.length === 0) return;
      const cur = items.indexOf(document.activeElement as HTMLElement);
      const next = e.shiftKey
        ? cur <= 0
          ? items.length - 1
          : cur - 1
        : cur === -1 || cur === items.length - 1
          ? 0
          : cur + 1;
      e.preventDefault();
      items[next].focus();
    };

    el.addEventListener("keydown", onKey);
    return () => el.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div
      className="fixed inset-0 z-50 grid place-items-center bg-black/50 p-6"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={panel}
        role="dialog"
        aria-modal="true"
        aria-label={label}
        tabIndex={-1}
        className="modal-in relative max-w-full rounded-xl border border-border bg-card p-5 shadow-xl outline-none"
        style={{ width: wide ? "34rem" : "24rem" }}
      >
        <button
          type="button"
          aria-label={`Close ${label}`}
          onClick={onClose}
          className="absolute right-3 top-3 cursor-pointer rounded p-1 text-muted-foreground transition-colors hover:text-foreground"
        >
          ✕
        </button>
        {children}
      </div>
    </div>
  );
}
