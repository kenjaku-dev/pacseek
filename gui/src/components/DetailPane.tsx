// Detail pane — title + badges row, meta fields, AUR popularity, description,
// dependency metadata (resolved for installed rows via package_info). The pane
// becomes a slide-in overlay below the content breakpoint (container query in
// App grid).
import { X } from "lucide-react";
import { useEffect, useState } from "react";
import { getPackageInfo, normalizeGuiError } from "../lib/ipc";
import type { PackageDto } from "../types";
import { badgeLabels } from "./ResultRow";

export interface Selection {
  leg: "repo" | "aur" | "flatpak";
  pkg: PackageDto;
}

function Field({ k, v }: { k: string; v: React.ReactNode }) {
  if (v === null || v === "" || v === undefined) return null;
  return (
    <div className="flex gap-3 border-b border-border/60 py-1.5 text-[12px] last:border-none">
      <span className="w-28 shrink-0 text-muted-foreground">{k}</span>
      <span className="min-w-0 break-words">{v}</span>
    </div>
  );
}

function fmtDate(epochSecs: number | null): string | null {
  if (epochSecs === null) return null;
  const d = new Date(epochSecs * 1000);
  if (Number.isNaN(d.getTime())) return String(epochSecs);
  return d.toISOString().slice(0, 10);
}

function DepList({ title, list }: { title: string; list: string[] | null }) {
  if (!list || list.length === 0) return null;
  return (
    <div className="mt-3">
      <h4 className="mb-1 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
        {title}
        <span className="ml-1.5 font-normal normal-case">{list.length}</span>
      </h4>
      <ul className="flex flex-wrap gap-1.5">
        {list.map((d) => (
          <li key={d} className="rounded bg-card-muted px-1.5 py-0.5 text-[11px]">
            {d}
          </li>
        ))}
      </ul>
    </div>
  );
}

export function DetailPane({ selection, onClose }: { selection: Selection; onClose: () => void }) {
  const [full, setFull] = useState<PackageDto | null>(null);
  const [infoError, setInfoError] = useState<string | null>(null);

  useEffect(() => {
    setFull(null);
    setInfoError(null);
    if (!selection.pkg.installed) return; // remote rows carry their metadata
    let live = true;
    getPackageInfo(selection.pkg.name)
      .then((dto) => {
        if (live) setFull(dto);
      })
      .catch((e) => {
        if (live) setInfoError(normalizeGuiError(e).message);
      });
    return () => {
      live = false;
    };
  }, [selection.pkg.name, selection.pkg.installed]);

  const merged: PackageDto = full ?? selection.pkg;
  const votes = merged.votes ?? selection.pkg.votes;
  const badges = badgeLabels(merged).filter((b) => b !== "OOD" && b !== "ORPHAN");

  return (
    <aside
      aria-label="Package detail"
      className="h-full overflow-y-auto border-l border-border bg-card"
    >
      <header className="sticky top-0 flex items-start justify-between gap-2 border-b border-border bg-card px-4 py-3">
        <div className="min-w-0">
          <h2 className="truncate text-sm font-semibold">{merged.name}</h2>
          <p className="mt-0.5 flex flex-wrap items-center gap-1.5 text-[10px]">
            {merged.out_of_date !== null && (
              <span className="rounded bg-destructive/15 px-1.5 py-0.5 text-destructive">[OOD]</span>
            )}
            {merged.orphan && (
              <span className="rounded bg-destructive/15 px-1.5 py-0.5 text-destructive">[ORPHAN]</span>
            )}
            {badges.map((b) => (
              <span
                key={b}
                className="rounded bg-accent-tertiary/15 px-1.5 py-0.5 text-accent-tertiary"
              >
                [{b}]
              </span>
            ))}
          </p>
        </div>
        <button
          type="button"
          aria-label="Close detail"
          onClick={onClose}
          className="shrink-0 cursor-pointer rounded p-1 text-muted-foreground transition-colors hover:text-foreground"
        >
          <X size={14} aria-hidden="true" />
        </button>
      </header>

      <div className="px-4 py-3">
        <Field k="version" v={merged.version} />
        <Field k="repository" v={merged.repo} />
        <Field k="architecture" v={merged.arch} />
        <Field k="url" v={merged.url} />
        <Field
          k="popularity"
          v={
            votes !== null || merged.popularity !== null ? (
              <>
                {votes ?? "—"} votes{(merged.popularity !== null && ` · ${merged.popularity.toFixed(2)}`) ?? ""}
              </>
            ) : null
          }
        />
        <Field k="maintainer" v={merged.maintainer ?? "—"} />
        <Field k="install reason" v={merged.reason} />
        <Field k="last modified" v={fmtDate(merged.last_modified)} />

        {merged.description && (
          <p className="mt-3 text-[12px] leading-relaxed text-foreground/90">
            {merged.description}
          </p>
        )}
        {infoError && (
          <p className="mt-3 rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-[11px] text-destructive">
            detail lookup failed: {infoError}
          </p>
        )}

        <DepList title="depends" list={merged.depends} />
        <DepList title="optional depends" list={merged.optdepends} />
        <DepList title="required by" list={merged.required_by} />
        <DepList title="optional for" list={merged.optional_for} />
      </div>
    </aside>
  );
}
