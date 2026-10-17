// App — AC search screen assembly: header (brand + field), filter bar,
// master-detail @container grid, status stripe. Keyboard-complete per the
// approved wireframe (gui/design/ac-search-brief.md).
import { useCallback, useEffect, useMemo, useState } from "react";
import { kbd } from "./components/kbd";
import { FilterBar } from "./components/FilterBar";
import { SearchField } from "./components/SearchField";
import { StagedResults } from "./components/StagedResults";
import { DetailPane, type Selection } from "./components/DetailPane";
import { StatusStripe } from "./components/StatusStripe";
import { flattenRows } from "./state/searchReducer";
import { useSearch } from "./lib/useSearch";
import type { RowRef } from "./types";

export default function App() {
  const search = useSearch();
  const [selection, setSelection] = useState<RowRef | null>(null);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);

  // Fresh generation ⇒ the old selection belongs to a torn-down list.
  useEffect(() => setSelection(null), [search.state.generation]);

  const rows = useMemo(() => flattenRows(search.state), [search.state]);

  const picked = useMemo<Selection | null>(() => {
    if (selection === null) return null;
    const hit = rows.find((r) => r.leg === selection.stage && r.pkg.name === selection.name);
    return hit ? { leg: hit.leg, pkg: hit.pkg } : null;
  }, [rows, selection]);

  const move = useCallback(
    (delta: 1 | -1) => {
      if (rows.length === 0) return;
      const cur = selection
        ? rows.findIndex((r) => r.leg === selection.stage && r.pkg.name === selection.name)
        : -1;
      const next = rows[(cur + delta + rows.length) % rows.length];
      setSelection({ stage: next.leg, name: next.pkg.name });
    },
    [rows, selection],
  );

  // Window-level hotkeys (skipped while typing; the input handles its own Esc).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const typing =
        document.activeElement instanceof HTMLInputElement ||
        document.activeElement instanceof HTMLTextAreaElement;
      if (e.key === "?" && !typing) {
        e.preventDefault();
        setShortcutsOpen(true);
      } else if (e.key === "/" && !typing) {
        e.preventDefault();
        document.querySelector<HTMLInputElement>('input[aria-label="Search packages"]')?.focus();
      } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        move(e.key === "ArrowDown" ? 1 : -1);
      } else if (e.key === "Escape" && selection !== null) {
        setSelection(null);
      } else if (e.key === "Enter" && selection !== null && !typing) {
        e.preventDefault(); // the row itself is actionable; focus follows the pane
        document.querySelector<HTMLElement>('aside[aria-label="Package detail"]')?.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [move, selection]);

  return (
    <div className="flex h-screen flex-col">
      <header className="flex items-center gap-4 border-b border-border bg-card px-4">
        <div className="select-none">
          <span className="text-sm font-bold tracking-tight text-foreground">pakmap</span>
          <span className="ml-2 rounded bg-card-muted px-1.5 text-[10px] text-muted-foreground">
            {search.summary ? `v${search.summary.app_version}` : "…"}
          </span>
        </div>
        <SearchField
          value={search.query}
          onInput={search.setQuery}
          onSubmit={search.run}
          onClear={search.clear}
        />
      </header>

      <FilterBar
        filters={search.filters}
        flatpakAvailable={search.summary?.flatpak_enabled ?? true}
        onSource={search.setSource}
        onToggle={search.toggle}
      />

      {search.state.error !== null && (
        <ErrorCard
          code={search.state.error.code}
          message={search.state.error.message}
          hint={search.state.error.hint}
        />
      )}

      <main
        className="@container relative flex min-h-0 flex-1 overflow-hidden"
        aria-label="Search results"
      >
        <StagedResults state={search.state} selected={selection} onSelect={setSelection} />

        {picked !== null && (
          <div className="@min-[1020px]:static absolute inset-y-0 right-0 z-10 @min-[1020px]:w-[min(24rem,40%)] w-85 max-w-full shadow-lg">
            <DetailPane selection={picked} onClose={() => setSelection(null)} />
          </div>
        )}

        {picked !== null && (
          <button
            type="button"
            className="@min-[1020px]:hidden absolute inset-0 z-[5] bg-black/40"
            aria-label="Close detail overlay"
            tabIndex={-1}
            onClick={() => setSelection(null)}
          />
        )}
      </main>

      <StatusStripe state={search.state} />

      {shortcutsOpen && <ShortcutsOverlay onClose={() => setShortcutsOpen(false)} />}
    </div>
  );
}

function ErrorCard({ code, message, hint }: { code: string; message: string; hint: string | null }) {
  return (
    <div
      role="alert"
      className="flex items-start gap-3 border-b border-destructive/40 bg-destructive/10 px-4 py-2.5"
    >
      <div className="min-w-0">
        <p className="text-[12px]">
          <span className="font-semibold text-destructive">{code}</span>{" "}
          <span className="text-foreground">{message}</span>
        </p>
        {hint && <p className="mt-0.5 text-[11px] text-muted-foreground">hint: {hint}</p>}
      </div>
    </div>
  );
}

function ShortcutsOverlay({ onClose }: { onClose: () => void }) {
  // Minimal dialog: Esc + backdrop close; full focus trap lands with the AC
  // a11y pass (axe smoke) — noted for polish.
  return (
    <div
      className="fixed inset-0 z-50 grid place-items-center bg-black/50 p-6"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Keyboard shortcuts"
        className="w-96 max-w-full rounded-xl border border-border bg-card p-5 shadow-xl"
      >
        <h2 className="mb-3 text-sm font-semibold">Keyboard shortcuts</h2>
        <ul className="space-y-2 text-[12px] text-muted-foreground">
          <li className="flex justify-between gap-4">
            <span>Focus search</span>
            <span>{kbd("/")}</span>
          </li>
          <li className="flex justify-between gap-4">
            <span>Search now (skip debounce)</span>
            <span>{kbd("Enter")}</span>
          </li>
          <li className="flex justify-between gap-4">
            <span>Move between rows</span>
            <span>
              {kbd("↑")} {kbd("↓")}
            </span>
          </li>
          <li className="flex justify-between gap-4">
            <span>Open detail on selected row</span>
            <span>{kbd("Enter")}</span>
          </li>
          <li className="flex justify-between gap-4">
            <span>Close detail / clear search</span>
            <span>{kbd("Esc")}</span>
          </li>
          <li className="flex justify-between gap-4">
            <span>Toggle regex / installed / orphans</span>
            <span>click chips</span>
          </li>
        </ul>
        <p className="mt-3 text-[10px] text-muted-foreground">Esc closes this dialog.</p>
      </div>
    </div>
  );
}
