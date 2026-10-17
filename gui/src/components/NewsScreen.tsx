// NewsScreen — Arch news feed (phase AD parity with the TUI news panel).
// Manual-intervention items surface both as a page-level banner (upgrade
// guard visibility) and a per-item badge. AF: links open in the system
// browser via the opener plugin (capability: allow-open-url only); a copy
// fallback stays for users who want the raw URL.
import { Copy } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { getNews, normalizeGuiError, openExternal } from "../lib/ipc";
import type { NewsItemDto } from "../types";

export function NewsScreen() {
  const [items, setItems] = useState<NewsItemDto[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    getNews()
      .then((n) => {
        if (live) setItems(n);
      })
      .catch((e) => {
        if (live) setError(normalizeGuiError(e).message);
      });
    return () => {
      live = false;
    };
  }, []);

  const interventions = useMemo(
    () => items?.filter((i) => i.needs_action) ?? [],
    [items],
  );

  if (error !== null) {
    return (
      <div className="p-4">
        <div role="alert" className="bg-destructive/10 px-4 py-2.5 text-xs">
          <span className="font-semibold text-destructive">news_failed</span>{" "}
          <span>{error}</span>
        </div>
      </div>
    );
  }
  if (items === null) {
    return (
      <div className="flex h-full items-center justify-center">
        <p role="status" className="text-xs text-muted-foreground">
          Fetching Arch news…
        </p>
      </div>
    );
  }

  return (
    <div className="mx-auto h-full max-w-3xl overflow-y-auto px-4 py-4">
      <h2 className="text-sm font-semibold">Arch news</h2>

      {interventions.length > 0 && (
        <div
          role="alert"
          className="mt-3 rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2.5"
        >
          <p className="text-xs font-semibold text-destructive">
            Manual intervention required before upgrading
          </p>
          <p className="mt-0.5 text-nano text-muted-foreground">
            {interventions.length} item(s) below describe steps you must take yourself — read
            them before running "Upgrade all".
          </p>
        </div>
      )}

      <ul role="list" className="mt-4 divide-y divide-border/60">
        {items.map((item) => (
          <li key={item.link} className="py-2.5">
            <div className="flex items-start justify-between gap-3">
              <div className="min-w-0">
                <p className="text-xs font-medium leading-snug">
                  {item.title}
                  {item.needs_action && (
                    <span className="ml-2 rounded bg-destructive/15 px-1.5 py-0.5 align-middle text-2xs text-destructive">
                      [ACTION REQUIRED]
                    </span>
                  )}
                </p>
                <p className="mt-0.5 text-2xs text-muted-foreground">
                  {item.pub_date ?? "unknown date"}
                </p>
              </div>
              <div className="flex shrink-0 items-center gap-1.5">
                <button
                  type="button"
                  aria-label={`Open news item in browser: ${item.title}`}
                  onClick={() => void openExternal(item.link)}
                  className="shrink-0 cursor-pointer rounded border border-border px-2 py-0.5 text-2xs text-muted-foreground transition-colors hover:bg-card-muted hover:text-foreground"
                >
                  Open
                </button>
                <button
                  type="button"
                  aria-label={`Copy link to news item: ${item.title}`}
                  onClick={() => void navigator.clipboard.writeText(item.link)}
                  className="flex h-6 w-6 shrink-0 cursor-pointer items-center justify-center rounded text-muted-foreground transition-colors hover:bg-card-muted hover:text-foreground"
                >
                  <Copy size={12} aria-hidden="true" />
                  <span className="sr-only">Copy link</span>
                </button>
              </div>
            </div>
          </li>
        ))}
      </ul>

      {items.length === 0 && (
        <p className="mt-8 text-center text-xs text-muted-foreground">
          No news items available right now.
        </p>
      )}
    </div>
  );
}
