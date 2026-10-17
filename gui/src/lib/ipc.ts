// Typed IPC surface — the only place `invoke`/`Channel` are touched.
import { Channel, invoke } from "@tauri-apps/api/core";
import type { ConfigSummary, GuiError, PackageDto, SearchEvent, SearchFilters } from "../types";

export type SearchEventSink = (event: SearchEvent) => void;

/**
 * Kick off a staged search. Returns the generation immediately; staged
 * SearchEvents (repo → aur → flatpak → done) stream through the channel.
 */
export async function startSearch(
  query: string,
  filters: SearchFilters,
  sink: SearchEventSink,
): Promise<{ generation: number }> {
  const channel = new Channel<SearchEvent>();
  channel.onmessage = sink;
  try {
    const generation = await invoke<number>("search", { query, filters, onEvent: channel });
    return { generation };
  } catch (err) {
    // A command error rejects with the serialized GuiError envelope.
    throw normalizeGuiError(err);
  }
}

export function cancelSearch(): Promise<number> {
  return invoke<number>("cancel_search");
}

export function getConfigSummary(): Promise<ConfigSummary> {
  return invoke<ConfigSummary>("get_config_summary");
}

/** Detail resolution for installed rows (deps/reason via localdb). */
export async function getPackageInfo(name: string): Promise<PackageDto> {
  try {
    return await invoke<PackageDto>("package_info", { name });
  } catch (err) {
    throw normalizeGuiError(err);
  }
}

export function normalizeGuiError(err: unknown): GuiError {
  if (
    typeof err === "object" &&
    err !== null &&
    "code" in err &&
    typeof (err as { code: unknown }).code === "string"
  ) {
    return err as GuiError;
  }
  return { code: "unknown", message: String(err), hint: null };
}
