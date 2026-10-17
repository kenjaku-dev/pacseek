// IPC facade — the app's single import point for backend calls.
// In mock mode (VITE_PAKMAP_MOCK_IPC=1, dev/E2E only) everything routes through
// the scripted gateway; in production the env replacement is a compile-time
// false and the mock module is tree-shaken out of the bundle.
import * as realGateway from "./tauriGateway";
import * as mockGateway from "./mockIpc";
import { normalizeGuiError } from "./tauriGateway";
import type { ConfigSummary, GuiError, PackageDto, SearchEvent, SearchFilters } from "../types";

export type SearchEventSink = (event: SearchEvent) => void;
export { normalizeGuiError };
export type { GuiError };

const MOCK = import.meta.env.VITE_PAKMAP_MOCK_IPC === "1";
const gw = MOCK ? mockGateway : realGateway;

export function startSearch(
  query: string,
  filters: SearchFilters,
  sink: (event: SearchEvent) => void,
): Promise<{ generation: number }> {
  return gw.startSearch(query, filters, sink);
}

export function cancelSearch(): Promise<number> {
  return gw.cancelSearch();
}

export function getConfigSummary(): Promise<ConfigSummary> {
  return gw.getConfigSummary();
}

export function getPackageInfo(name: string): Promise<PackageDto> {
  return gw.getPackageInfo(name);
}

/** normalizeGuiError is re-exported above — shared by both gateways. */

export type { SearchEvent };
