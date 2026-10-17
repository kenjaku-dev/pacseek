// FilterBar — segmented source control + toggle chips. Keyboard-reachable
// (Tab), aria-pressed state, no dropdowns (target audience: TUI refugees).
import type { SearchFilters, SourceChoice } from "../types";

const SOURCES: ReadonlyArray<{ id: SourceChoice; label: string }> = [
  { id: "all", label: "All" },
  { id: "repo", label: "Repo" },
  { id: "aur", label: "AUR" },
  { id: "flatpak", label: "Flatpak" },
];

interface FilterBarProps {
  filters: SearchFilters;
  flatpakAvailable: boolean;
  onSource: (s: SourceChoice) => void;
  onToggle: (k: "use_regex" | "installed_only" | "orphans_only") => void;
}

function Chip({
  label,
  pressed,
  onClick,
  hint,
}: {
  label: string;
  pressed: boolean;
  onClick: () => void;
  hint: string;
}) {
  return (
    <button
      type="button"
      aria-pressed={pressed}
      title={hint}
      onClick={onClick}
      className={
        "h-6 cursor-pointer rounded-full border px-3 text-nano transition-colors " +
        (pressed
          ? "border-accent bg-accent/15 text-accent"
          : "border-border text-muted-foreground hover:border-accent-secondary/60 hover:text-foreground")
      }
    >
      {label}
    </button>
  );
}

export function FilterBar({ filters, flatpakAvailable, onSource, onToggle }: FilterBarProps) {
  return (
    <div className="flex items-center gap-3 border-b border-border bg-card px-4" role="toolbar" aria-label="Search filters">
      <div
        className="rounded-lg border border-border bg-card-muted p-0.5"
        role="group"
        aria-label="Source filter"
      >
        {SOURCES.map((s) => {
          const disabled = s.id === "flatpak" && !flatpakAvailable;
          return (
            <button
              key={s.id}
              type="button"
              disabled={disabled}
              aria-pressed={filters.source === s.id}
              onClick={() => onSource(s.id)}
              className={
                "h-6 cursor-pointer rounded-md px-2.5 text-nano transition-colors disabled:cursor-not-allowed disabled:opacity-40 " +
                (filters.source === s.id
                  ? "bg-accent-secondary/20 text-accent-secondary"
                  : "text-muted-foreground hover:text-foreground")
              }
            >
              {s.label}
            </button>
          );
        })}
      </div>

      <Chip
        label="regex"
        hint="Interpret the query as a regular expression (matches like the TUI)"
        pressed={filters.use_regex}
        onClick={() => onToggle("use_regex")}
      />
      <Chip
        label="installed"
        hint="Only installed packages (localdb fast path)"
        pressed={filters.installed_only}
        onClick={() => onToggle("installed_only")}
      />
      <Chip
        label="orphans"
        hint="Installed as dependency with nothing requiring it (pacman -Qdt)"
        pressed={filters.orphans_only}
        onClick={() => onToggle("orphans_only")}
      />
    </div>
  );
}
