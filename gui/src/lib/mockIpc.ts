// Mock gateway — E2E/smoke harness only (Playwright dev-browser mode).
// Scripted staged legs + fixtures that mirror what the real backend sends,
// including supersede semantics (a newer run silences an older one) and the
// same trigger verb "err" that exercises the error card.
//
// Gated by VITE_PAKMAP_MOCK_IPC=1 so the production bundle never includes it.
import type { ConfigSummary, GuiError, PackageDto, SearchEvent, SearchFilters } from "../types";
import { normalizeGuiError } from "./tauriGateway";

export type SearchEventSink = (event: SearchEvent) => void;

const FAST = import.meta.env.VITE_PAKMAP_MOCK_FAST === "1";
const STEP = FAST ? 25 : 300;

const SUMMARY: ConfigSummary = {
  app_version: "0.7.0",
  limit: 50,
  source: "all",
  timeout_secs: 30,
  flatpak_enabled: true,
  custom_aur_rpc: false,
  debounce_ms: 50,
};

const pkg = (name: string, repo: string, extra: Partial<PackageDto> = {}): PackageDto => ({
  name,
  version: "1.0-1",
  description: `${name} — mock description for testing`,
  repo,
  arch: "x86_64",
  url: "https://example.org",
  installed: false,
  votes: null,
  popularity: null,
  out_of_date: null,
  maintainer: "someone",
  last_modified: 1600000000,
  depends: null,
  optdepends: null,
  required_by: null,
  optional_for: null,
  reason: null,
  orphan: false,
  ...extra,
});

const LEGS: Record<"repo" | "aur" | "flatpak", PackageDto[]> = {
  repo: [
    pkg("firefox", "extra", { installed: true, description: "Mozilla Firefox web browser" }),
    pkg("firefox-esr", "extra"),
  ],
  aur: [
    pkg("firefox-bin", "aur", { votes: 421, popularity: 3.14, installed: true }),
    pkg("firefox-nightly", "aur", { votes: 42, out_of_date: 1600000000 }),
    pkg("orphaned-thing", "aur", { votes: 3, maintainer: null }),
  ],
  flatpak: [pkg("org.mozilla.firefox", "flatpak", { arch: null, maintainer: null })],
};

const LEGS_NOTHING: Record<"repo" | "aur" | "flatpak", PackageDto[]> = {
  repo: [],
  aur: [],
  flatpak: [],
};

let counter = 0;

function legsFor(query: string): Record<"repo" | "aur" | "flatpak", PackageDto[]> {
  // Fixture selector keeps tests deterministic: "*" → empty everything.
  if (query.startsWith("*")) return LEGS_NOTHING;
  return LEGS;
}

export async function getConfigSummary(): Promise<ConfigSummary> {
  return SUMMARY;
}

export async function startSearch(
  query: string,
  _filters: SearchFilters,
  sink: SearchEventSink,
): Promise<{ generation: number }> {
  const myGen = ++counter;
  const q = query.trim();

  // scripted failure surface — same envelope shape as the real channel errors
  if (q.startsWith("err")) {
    throw {
      code: "repo_search_failed",
      message: `mock failure for '${q}'`,
      hint: "run pacman -Sy (mock)",
    } satisfies GuiError;
  }

  const legs = q.startsWith("orph:") ? LEGS_NOTHING : legsFor(q);

  const send = (stage: "repo" | "aur" | "flatpak" | "done", packages: PackageDto[], delay: number) => {
    setTimeout(() => {
      // Supersede guard: a newer run (counter moved on) silences us — the
      // backend's active_search guard behaves exactly like this.
      if (counter !== myGen) return;
      const event: SearchEvent = { generation: myGen, stage, packages };
      sink(event);
    }, delay);
  };

  // Orphans mode mirrors the local path: carried-payload done.
  if (q.startsWith("orph:")) {
    setTimeout(() => {
      if (counter !== myGen) return;
      sink({
        generation: myGen,
        stage: "done",
        packages: [pkg("some-widget", "local", { installed: true, orphan: true, reason: "dependency" })],
      });
    }, STEP);
  } else {
    send("repo", legs.repo, STEP);
    send("aur", legs.aur, STEP * 2);
    send("flatpak", legs.flatpak, STEP * 3);
    send("done", [], STEP * 4);
  }
  return { generation: myGen };
}

export async function cancelSearch(): Promise<number> {
  counter += 1; // drops all in-flight sends
  return counter;
}

export async function getPackageInfo(name: string): Promise<PackageDto> {
  const hit = LEGS.repo
    .concat(LEGS.aur, LEGS.flatpak)
    .find((p) => p.name === name && p.installed);
  if (!hit) {
    throw {
      code: "package_not_found",
      message: `installed package '${name}' not found in localdb`,
      hint: "the package may not be installed",
    } satisfies GuiError;
  }
  return { ...hit, depends: ["gtk3", "nss"], optdepends: ["langpack"], reason: "explicit", required_by: [] };
}

export { normalizeGuiError };
