"use client";

import { useEffect, useRef } from "react";
import { AnimatePresence, motion, useReducedMotion } from "framer-motion";
import { Menu, X } from "lucide-react";
import { NavLinks } from "./NavLinks";
import { SITE } from "@/lib/site";

type MobileNavProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  activeHref?: string;
};

export function MobileNav({ open, onOpenChange, activeHref }: MobileNavProps) {
  const reduce = useReducedMotion();
  const panelRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!open) return;

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onOpenChange(false);
        triggerRef.current?.focus();
        return;
      }

      if (e.key !== "Tab" || !panelRef.current) return;

      const focusables = panelRef.current.querySelectorAll<HTMLElement>(
        'a[href], button:not([disabled])',
      );
      if (focusables.length === 0) return;

      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      const active = document.activeElement;

      if (e.shiftKey && active === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && active === last) {
        e.preventDefault();
        first.focus();
      }
    };

    document.addEventListener("keydown", onKeyDown);
    const timer = window.setTimeout(() => {
      const firstLink = panelRef.current?.querySelector<HTMLElement>("a[href]");
      firstLink?.focus();
    }, 50);

    return () => {
      document.removeEventListener("keydown", onKeyDown);
      window.clearTimeout(timer);
    };
  }, [open, onOpenChange]);

  return (
    <>
      <button
        ref={triggerRef}
        type="button"
        id="nav-menu-trigger"
        aria-label={open ? "Close menu" : "Open menu"}
        aria-expanded={open}
        aria-controls="mobile-nav-panel"
        onClick={() => onOpenChange(!open)}
        className="inline-flex h-11 w-11 cursor-pointer items-center justify-center rounded-md border border-border text-muted-foreground transition-colors duration-200 hover:border-accent/50 hover:text-foreground sm:hidden"
      >
        {open ? (
          <X aria-hidden="true" className="h-5 w-5" />
        ) : (
          <Menu aria-hidden="true" className="h-5 w-5" />
        )}
      </button>

      <AnimatePresence>
        {open && (
          <>
            <motion.div
              key="backdrop"
              initial={reduce ? { opacity: 1 } : { opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={reduce ? { opacity: 0 } : { opacity: 0 }}
              transition={{ duration: 0.2 }}
              className="fixed inset-0 z-40 bg-background/70 sm:hidden"
              onClick={() => {
                onOpenChange(false);
                triggerRef.current?.focus();
              }}
              aria-hidden="true"
            />
            <motion.div
              key="panel"
              ref={panelRef}
              id="mobile-nav-panel"
              role="dialog"
              aria-modal="true"
              aria-label="Site menu"
              initial={reduce ? { opacity: 1 } : { opacity: 0, y: -8 }}
              animate={{ opacity: 1, y: 0 }}
              exit={reduce ? { opacity: 0 } : { opacity: 0, y: -8 }}
              transition={{ duration: 0.2, ease: [0.22, 1, 0.36, 1] }}
              className="fixed inset-x-0 top-14 z-50 border-b border-border bg-background/95 px-5 pb-6 pt-3 backdrop-blur-md sm:hidden"
            >
              <NavLinks
                activeHref={activeHref}
                onNavigate={() => onOpenChange(false)}
                className="gap-0"
              />
              <a
                href="#install"
                onClick={() => onOpenChange(false)}
                className="mt-3 flex min-h-11 w-full items-center justify-center rounded-md bg-accent px-4 text-sm font-semibold text-on-accent transition-opacity duration-200 hover:opacity-90"
              >
                Install {SITE.name}
              </a>
            </motion.div>
          </>
        )}
      </AnimatePresence>
    </>
  );
}
