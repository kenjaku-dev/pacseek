// StatusStripe — the footer parity contract: exact TUI wording.
// Searching: `Searching for 'q' — N found so far...`
// Done:      `Found N (repo X aur Y flatpak Z)` (+ elapsed ms, GUI addition)
import type { GuiError } from "../types";
import type { SearchState } from "../state/searchReducer";
import { totalFound } from "../state/searchReducer";

export function StatusStripe({ state }: { state: SearchState }) {
  const n = totalFound(state);

  let text: string;
  if (state.error !== null) {
    text = errorLine(state.error);
  } else if (!state.done) {
    text = `Searching for '${state.query}' — ${n} found so far...`;
  } else if (state.query === "" && state.carriedDone) {
    // local list paths (installed/orphans) mirror the TUI Done wording
    text = `Found ${n}`;
  } else {
    text = `Found ${n} (repo ${len(state.groups.repo)} aur ${len(state.groups.aur)} flatpak ${len(state.groups.flatpak)})`;
  }

  return (
    <footer
      role="status"
      aria-live="polite"
      className="flex h-8 items-center justify-between gap-4 border-t border-border bg-card px-4 text-[11px] text-muted-foreground"
    >
      <span className="truncate">{text}</span>
      <span className="shrink-0 tabular-nums">
        {state.elapsedMs !== null ? `${state.elapsedMs}ms` : ""}
      </span>
    </footer>
  );
}

function len(list: unknown[] | null): number {
  return list?.length ?? 0;
}

function errorLine(err: GuiError): string {
  return `${err.code}: ${err.message}${err.hint ? ` — ${err.hint}` : ""}`;
}
