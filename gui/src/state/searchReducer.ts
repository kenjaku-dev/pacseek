// Typed stream reducer implementing the staged protocol with the generation
// guard — the exact invariants the TUI enforces (context.md Y), unit-tested to
// mirror its 4 staged-flow tests:
//
// 1. staged accumulate → Done swaps the generation in atomically
// 2. superseded generations drop out incrementally (ghost stages never leak)
// 3. every start REPLACES the whole scratch area (fresh groups)
// 4. carry-payload Done fills the group slot instead of appending
//
// Guard model: `identity` is a frontend-issued monotonic id bound into the
// per-call event sink (see useSearch) — immune to IPC ordering races; the
// backend additionally guards sends with its own active_search generation.
import type { GuiError, PackageDto, SearchEvent, SearchFilters, LegStage } from "../types";

export interface SearchState {
  /** current run id; two runs never mix (TUI `active_search` analogue) */
  generation: number;
  query: string;
  filters: SearchFilters;
  /** null = leg not expected/arrived (skeleton), [] = arrived empty */
  groups: Record<LegStage, PackageDto[] | null>;
  /** true when the final Done marker for `generation` arrived */
  done: boolean;
  /** local search path (installed-only / orphans-only): payload Done */
  carriedDone: boolean;
  /** wallclock of `start` — elapsed shown on Done like the TUI status */
  startedAt: number | null;
  elapsedMs: number | null;
  error: GuiError | null;
}

export const LEGS: readonly LegStage[] = ["repo", "aur", "flatpak"];

const WIRE_LEGS: readonly string[] = ["repo", "aur", "flatpak"];

export type SearchAction =
  | {
      type: "start";
      identity: number;
      query: string;
      filters: SearchFilters;
      carried: boolean;
      at: number;
    }
  | { type: "stage"; identity: number; event: SearchEvent; at: number }
  | { type: "error"; error: GuiError }
  | { type: "reset" };

export const emptyFilters: SearchFilters = {
  source: "all",
  aur_by: "name-desc",
  use_regex: false,
  installed_only: false,
  orphans_only: false,
  limit: null,
};

export const initialSearchState: SearchState = {
  generation: 0,
  query: "",
  filters: emptyFilters,
  groups: { repo: null, aur: null, flatpak: null },
  done: true,
  carriedDone: false,
  startedAt: null,
  elapsedMs: null,
  error: null,
};

const freshGroups = (): Record<LegStage, PackageDto[] | null> => ({
  repo: null,
  aur: null,
  flatpak: null,
});

export function searchReducer(state: SearchState, action: SearchAction): SearchState {
  switch (action.type) {
    case "reset":
      return { ...initialSearchState };

    case "start":
      return {
        ...state,
        generation: action.identity,
        query: action.query,
        filters: action.filters,
        groups: freshGroups(),
        done: false,
        carriedDone: action.carried,
        startedAt: action.at,
        elapsedMs: null,
        error: null,
      };

    case "error":
      return { ...state, done: true, error: action.error };

    case "stage": {
      // Identity guard: anything not from the current run is a ghost from a
      // superseded search — dropped, never mixed (TUI invariant #2).
      if (action.identity !== state.generation) return state;

      const ev = action.event;

      if (ev.stage === "done") {
        // Carry-payload Done (local path): fills the repo slot and clears the
        // others — the TUI's atomic swap happens through the same marker.
        if (ev.packages.length > 0) {
          return {
            ...state,
            groups: { ...freshGroups(), repo: ev.packages },
            done: true,
            carriedDone: true,
            elapsedMs: state.startedAt === null ? null : action.at - state.startedAt,
          };
        }
        return {
          ...state,
          done: true,
          elapsedMs: state.startedAt === null ? null : action.at - state.startedAt,
        };
      }

      if (!WIRE_LEGS.includes(ev.stage)) return state;
      const leg = ev.stage as LegStage;
      // Same-run late stage for an already-arrived leg replaces — within a
      // generation each leg arrives once; replace semantics are the invariant.
      return { ...state, groups: { ...state.groups, [leg]: ev.packages } };
    }
  }
}

export function isSearching(state: SearchState): boolean {
  return !state.done && !state.error;
}

export function arrivedCount(state: SearchState): number {
  return LEGS.reduce((sum, leg) => sum + (state.groups[leg]?.length ?? 0), 0);
}

export function totalFound(state: SearchState): number {
  if (!state.done && !state.error) return arrivedCount(state);
  return (
    (state.groups["repo"]?.length ?? 0) +
    (state.groups["aur"]?.length ?? 0) +
    (state.groups["flatpak"]?.length ?? 0)
  );
}

/** Flattened visible rows in group order repo → aur → flatpak (policy order). */
export function flattenRows(state: SearchState): Array<{ leg: LegStage; pkg: PackageDto }> {
  const out: Array<{ leg: LegStage; pkg: PackageDto }> = [];
  for (const leg of LEGS) {
    for (const pkg of state.groups[leg] ?? []) out.push({ leg, pkg });
  }
  return out;
}
