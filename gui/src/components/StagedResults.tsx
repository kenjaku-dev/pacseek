// StagedResults — three group blocks in policy order (Repo → AUR → Flatpak).
// Each block shows a live count badge and a skeleton while its leg streams;
// rows are dense single-line (name · version · repo tag · badges), 12px rhythm.
// ARIA: one labelled listbox per leg — axe-core's aria-required-children
// requires `option` rows as direct children, so groups are presentational
// headers around a listbox, not wrappers inside one. Selection is tracked via
// aria-activedescendant on each listbox (palette behavior: focus stays in the
// search input while arrows navigate).
import type { LegStage, RowRef } from "../types";
import type { SearchState } from "../state/searchReducer";
import { LEGS, flattenRows } from "../state/searchReducer";
import { ResultRow } from "./ResultRow";
import { EmptyState } from "./states";

const LEG_META: Record<LegStage, { title: string; tone: string }> = {
  repo: { title: "Repo", tone: "text-accent-secondary" },
  aur: { title: "AUR", tone: "text-accent-tertiary" },
  flatpak: { title: "Flatpak", tone: "text-accent" },
};

interface StagedResultsProps {
  state: SearchState;
  selected: RowRef | null;
  onSelect: (ref: RowRef) => void;
  listRef?: React.RefObject<HTMLDivElement | null>;
}

export function StagedResults({ state, selected, onSelect, listRef }: StagedResultsProps) {
  const rows = flattenRows(state);
  const isEmptyQuery = state.query === "" && rows.length === 0;

  if (isEmptyQuery) {
    return (
      <div className="flex-1 overflow-auto p-4" ref={listRef}>
        <EmptyState />
      </div>
    );
  }

  const visibleLegs = state.carriedDone ? LEGS.filter((l) => l === "repo") : LEGS;

  return (
    <div className="flex-1 overflow-auto p-4" ref={listRef}>
      {state.query === "" && rows.length > 0 && (
        <p className="mb-3 text-[11px] text-muted-foreground">
          Showing a filtered view of the current run — type a query to stream all sources.
        </p>
      )}
      {visibleLegs.map((leg) => {
        const pkgs = state.groups[leg];
        const activeId =
          selected && selected.stage === leg
            ? `row-${selected.stage}--${selected.name}`
            : undefined;
        return (
          <section key={leg} aria-label={`${LEG_META[leg].title} results`}>
            <header className="flex items-center gap-2 px-1 pt-3 pb-1.5 first:pt-0">
              <h3 className={`text-[11px] font-semibold uppercase tracking-wider ${LEG_META[leg].tone}`}>
                {LEG_META[leg].title}
              </h3>
              <span
                data-testid={`count-${leg}`}
                className="min-w-7 rounded bg-card-muted px-1.5 text-center text-[10px] text-muted-foreground"
              >
                {pkgs === null ? "…" : String(pkgs.length)}
              </span>
              {pkgs === null && state.query !== "" && <SkeletonLine />}
            </header>

            <div role="listbox" aria-label={`${LEG_META[leg].title} results`} tabIndex={-1} aria-activedescendant={activeId}>
              {pkgs?.map((pkg) => (
                <ResultRow
                  key={`${leg}/${pkg.name}`}
                  rowKey={`${leg}/${pkg.name}`}
                  leg={leg}
                  pkg={pkg}
                  selected={selected?.stage === leg && selected?.name === pkg.name}
                  onSelect={() => onSelect({ stage: leg, name: pkg.name })}
                />
              ))}
            </div>

            {pkgs !== null && pkgs.length === 0 && (
              <p className="px-1 py-1.5 text-[11px] text-muted-foreground">
                {leg === "flatpak" ? "no flatpak results (or flatpak disabled)" : "nothing found"}
              </p>
            )}
          </section>
        );
      })}

      {rows.length === 0 && state.query !== "" && state.done && (
        <p className="mt-3 rounded-lg border border-border bg-card px-3 py-2.5 text-[12px] text-muted-foreground">
          Nothing matches “{state.query}” — try fewer filters or a shorter query.
        </p>
      )}
    </div>
  );
}

function SkeletonLine() {
  return <span className="h-2 w-24 animate-pulse rounded bg-card-muted" />;
}
