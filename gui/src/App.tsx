import { useCallback, useEffect, useState, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";

// AB spike panel: proves the three IPC round trips (ping · config · repo
// search) against the Rust core through Tauri commands. The real staged
// streaming UI is phase AC — expect intentionally minimal visuals here.

interface ConfigSummary {
  app_version: string;
  limit: number;
  source: string;
  timeout_secs: number;
  flatpak_enabled: boolean;
  custom_aur_rpc: boolean;
}

interface PackageDto {
  name: string;
  version: string;
  description: string | null;
  repo: string;
  installed: boolean;
}

interface GuiError {
  code: string;
  message: string;
  hint?: string;
}

function asGuiError(e: unknown): GuiError {
  if (typeof e === "object" && e !== null && "code" in e && "message" in e) {
    return e as GuiError;
  }
  return { code: "unknown", message: String(e) };
}

function Card({
  title,
  children,
}: {
  title: string;
  children: ReactNode;
}) {
  return (
    <section className="rounded-xl border border-slate-700 bg-slate-800/60 p-4 shadow">
      <h2 className="mb-2 text-sm font-semibold uppercase tracking-wide text-slate-400">
        {title}
      </h2>
      {children}
    </section>
  );
}

export default function App() {
  const [greeting, setGreeting] = useState<string | null>(null);
  const [config, setConfig] = useState<ConfigSummary | null>(null);
  const [results, setResults] = useState<PackageDto[] | null>(null);
  const [error, setError] = useState<GuiError | null>(null);

  useEffect(() => {
    invoke<ConfigSummary>("get_config_summary")
      .then(setConfig)
      .catch((e) => setError(asGuiError(e)));
  }, []);

  const runSearch = useCallback((query: string) => {
    setError(null);
    setResults(null);
    invoke<PackageDto[]>("search_once", { query })
      .then(setResults)
      .catch((e) => setError(asGuiError(e)));
  }, []);

  return (
    <main className="min-h-screen bg-slate-900 p-8 text-slate-100">
      <h1 className="mb-1 text-2xl font-bold">pakmap · GUI spike (AB)</h1>
      <p className="mb-6 text-sm text-slate-400">
        Tauri 2.12 + React 19 — the search UI itself arrives in phase AC.
      </p>

      <div className="grid gap-4 md:grid-cols-2">
        <Card title="IPC ping (greet)">
          <button
            type="button"
            onClick={() =>
              invoke<string>("greet", { name: "pakmap" })
                .then(setGreeting)
                .catch((e) => setError(asGuiError(e)))
            }
            className="rounded-lg bg-blue-600 px-3 py-1.5 text-sm font-medium hover:bg-blue-500"
          >
            Call greet
          </button>
          {greeting && <p className="mt-2 text-slate-300">{greeting}</p>}
        </Card>

        <Card title="Config summary (get_config_summary)">
          {config ? (
            <ul className="text-sm text-slate-300">
              <li>version: {config.app_version}</li>
              <li>
                search limit: {config.limit} · source: {config.source}
              </li>
              <li>
                timeout: {config.timeout_secs}s · flatpak:{" "}
                {config.flatpak_enabled ? "on" : "off"}
              </li>
            </ul>
          ) : (
            <p className="text-slate-500">loading…</p>
          )}
        </Card>

        <Card title="Repo search (search_once — local DB, no network)">
          {["firefox", "pacman", "ripgrep"].map((q) => (
            <button
              key={q}
              type="button"
              onClick={() => runSearch(q)}
              className="mr-2 rounded-lg bg-slate-700 px-3 py-1.5 text-sm hover:bg-slate-600"
            >
              {q}
            </button>
          ))}
          {results && (
            <ul className="mt-3 max-h-56 space-y-1 overflow-auto text-sm">
              {results.map((p) => (
                <li key={`${p.repo}/${p.name}`} className="truncate">
                  <span className="rounded bg-slate-700 px-1.5 py-0.5 text-xs text-slate-300">
                    {p.repo}
                  </span>{" "}
                  <span className="font-medium">{p.name}</span>{" "}
                  <span className="text-slate-400">{p.version}</span>
                </li>
              ))}
            </ul>
          )}
        </Card>

        <Card title="Error envelope demo">
          {error ? (
            <div className="text-sm">
              <p className="font-mono text-amber-300">{error.code}</p>
              <p className="text-slate-300">{error.message}</p>
              {error.hint && (
                <p className="mt-1 text-slate-400">hint: {error.hint}</p>
              )}
            </div>
          ) : (
            <p className="text-sm text-slate-500">
              no error yet — the empty-query trip fires on demand
            </p>
          )}
        </Card>
      </div>
    </main>
  );
}
