// Real IPC gateway — the only place `invoke`/`Channel` are touched.
import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  ConfigFlags,
  ConfigSummary,
  GuiError,
  NewsItemDto,
  OpMessage,
  OpRequest,
  PackageDto,
  SearchEvent,
  SearchFilters,
  StatsDto,
  UpdatesResult,
} from "../types";

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

// ---------------------------------------------------------------------------
// AD operations surface
// ---------------------------------------------------------------------------

/** Kick an operation off. Progress/Done stream through the channel sink. */
export async function runOp(
  request: OpRequest,
  sink: (msg: OpMessage) => void,
): Promise<number> {
  const channel = new Channel<OpMessage>();
  channel.onmessage = sink;
  try {
    return await invoke<number>("run_op", { op: request, onEvent: channel });
  } catch (err) {
    throw normalizeGuiError(err);
  }
}

/** Phase-boundary cancel. False = op unknown/already terminal. */
export function cancelOperation(opId: number): Promise<boolean> {
  return invoke<boolean>("cancel_operation", { opId });
}

/** Free the backend cancel-flag entry after a terminal banner is dismissed. */
export function dropOperation(opId: number): Promise<void> {
  return invoke<void>("drop_operation", { opId });
}

export function getConfigFlags(): Promise<ConfigFlags> {
  return invoke<ConfigFlags>("get_config_flags");
}

export async function getUpdates(): Promise<UpdatesResult> {
  return invoke<UpdatesResult>("get_updates");
}

export async function getNews(): Promise<NewsItemDto[]> {
  return invoke<NewsItemDto[]>("get_news");
}

export function getStats(): Promise<StatsDto> {
  return invoke<StatsDto>("get_stats");
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
