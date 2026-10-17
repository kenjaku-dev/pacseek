// Reducer fixture tests — mirrors the 4 TUI staged-flow tests (context.md Y):
// accumulate→swap, superseded discard, carry-payload Done, ghost-scratch replace.
import { describe, expect, it } from "vitest";
import {
  arrivedCount,
  emptyFilters,
  flattenRows,
  initialSearchState,
  searchReducer,
  type SearchState,
} from "./searchReducer";
import type { PackageDto } from "../types";

const pkg = (name: string, repo: string): PackageDto => ({
  name,
  version: "1.0-1",
  description: `desc ${name}`,
  repo,
  arch: "x86_64",
  url: null,
  installed: false,
  votes: null,
  popularity: null,
  out_of_date: null,
  maintainer: null,
  last_modified: null,
  depends: null,
  optdepends: null,
  required_by: null,
  optional_for: null,
  reason: null,
  orphan: false,
});

const wire = (
  generation: number,
  stage: "repo" | "aur" | "flatpak" | "done",
  packages: PackageDto[] = [],
) => ({ generation, stage, packages });

/** start run `id`; subsequent id-bound stages, then return the state chain */
function beginRun(
  s: SearchState,
  id: number,
  query: string,
  filters = emptyFilters,
  carried = false,
  at = 1_000_000,
): SearchState {
  return searchReducer(s, { type: "start", identity: id, query, filters, carried, at });
}

function stage(
  s: SearchState,
  id: number,
  stage: "repo" | "aur" | "flatpak" | "done",
  packages: PackageDto[] = [],
  at = 1_000_000,
): SearchState {
  return searchReducer(s, { type: "stage", identity: id, event: wire(id, stage, packages), at });
}

describe("staged search reducer", () => {
  it("accumulate → Done swaps the generation in atomically (TUI test 1)", () => {
    let s = beginRun(initialSearchState, 1, "firefox");
    s = stage(s, 1, "repo", [pkg("firefox", "extra")], 1_000_040);
    s = stage(s, 1, "aur", [pkg("firefox-bin", "aur")], 1_000_120);
    s = stage(s, 1, "flatpak", [pkg("Firefox", "flatpak")], 1_000_200);
    s = stage(s, 1, "done", [], 1_000_205);

    expect(s.done).toBe(true);
    expect(s.error).toBeNull();
    expect(s.groups.repo).toHaveLength(1);
    expect(s.groups.aur).toHaveLength(1);
    expect(s.groups.flatpak).toHaveLength(1);
    expect(arrivedCount(s)).toBe(3);
    expect(s.elapsedMs).toBe(205);
    expect(flattenRows(s).map((r) => r.pkg.name)).toEqual(["firefox", "firefox-bin", "Firefox"]);
  });

  it("superseded generations drop out incrementally (TUI test 2)", () => {
    let s = beginRun(initialSearchState, 1, "fire");
    s = stage(s, 1, "repo", [pkg("firefox", "extra")], 1_000_030);

    // run 2 supersedes run 1 while only its repo leg arrived
    s = beginRun(s, 2, "firefox", emptyFilters, false, 1_000_060);
    // a ghost AUR stage from run 1 must never leak into run 2
    s = stage(s, 1, "aur", [pkg("ghost", "aur")], 1_000_090);
    s = stage(s, 1, "done", [], 1_000_095);
    // run 2 completes independently
    s = stage(s, 2, "repo", [pkg("firefox", "extra")], 1_000_120);
    s = stage(s, 2, "done", [], 1_000_130);

    expect(s.generation).toBe(2);
    expect(s.groups.aur).toBeNull(); // ghost stage discarded
    expect(s.done).toBe(true); // run-1 Done must not finish run 2
    expect(arrivedCount(s)).toBe(1);
    expect(flattenRows(s).map((r) => r.pkg.name)).toEqual(["firefox"]);
  });

  it("carry-payload Done fills the repo slot and clears others (TUI test 3)", () => {
    let s = beginRun(
      initialSearchState,
      5,
      "",
      { ...emptyFilters, installed_only: true },
      true,
    );
    s = stage(s, 5, "flatpak", [pkg("should-vanish", "flatpak")], 1_000_010);
    expect(s.groups.flatpak).toHaveLength(1); // still visible pre-done
    s = stage(s, 5, "done", [pkg("htop", "local"), pkg("pacman", "local")], 1_000_050);

    expect(s.done).toBe(true);
    expect(s.carriedDone).toBe(true);
    expect(s.groups.repo?.map((p) => p.name)).toEqual(["htop", "pacman"]);
    expect(s.groups.flatpak).toBeNull(); // torn-down (leg N/A), not merged
    expect(s.elapsedMs).toBe(50);
  });

  it("groups reset on every start (ghost scratch replace, TUI test 4)", () => {
    let s = beginRun(initialSearchState, 1, "a");
    s = stage(s, 1, "repo", [pkg("leftover", "extra")], 1_000_010);
    s = beginRun(s, 2, "b", emptyFilters, false, 1_000_020);
    // a stale run-1 stage arriving before any run-2 leg is discarded
    s = stage(s, 1, "aur", [pkg("ghost", "aur")], 1_000_025);
    s = stage(s, 2, "repo", [pkg("fresh", "extra")], 1_000_040);

    expect(s.groups.repo?.map((p) => p.name)).toEqual(["fresh"]);
    expect(s.groups.aur).toBeNull();
    expect(flattenRows(s).map((r) => r.pkg.name)).toEqual(["fresh"]);
  });

  it("error action terminates a run and stop guard holds", () => {
    let s = beginRun(initialSearchState, 1, "x");
    s = searchReducer(s, {
      type: "error",
      error: { code: "repo_search_failed", message: "db missing", hint: "run pacman -Sy" },
    });
    expect(s.done).toBe(true);
    expect(s.error?.code).toBe("repo_search_failed");
    // late events from the same run are harmless, late state stays terminal
    s = stage(s, 1, "repo", [pkg("late", "extra")], 1_000_099);
    expect(s.error).not.toBeNull();
    s = searchReducer(s, { type: "reset" });
    expect(s).toEqual(initialSearchState);
  });
});
