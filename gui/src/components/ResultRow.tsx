// ResultRow — dense single-line row (12px rhythm, 8px grid). One signal per
// slot: repo tag, badges row, truncated description. Keyboard rows are focus-
// navigable via the list's arrow-key handler (row divs get tabindex=-1).
import type { LegStage, PackageDto } from "../types";

const LEG_TONE: Record<LegStage, string> = {
  repo: "text-accent-secondary",
  aur: "text-accent-tertiary",
  flatpak: "text-accent",
};

export function badgeLabels(pkg: PackageDto): string[] {
  // Same badge semantics as `Package::badges()` in the core model.
  const b: string[] = [];
  if (pkg.out_of_date !== null) b.push("OOD");
  if (pkg.orphan) b.push("ORPHAN");
  if (pkg.repo === "aur" && pkg.maintainer === null) b.push("UNMAINTAINED");
  return b;
}

interface ResultRowProps {
  leg: LegStage;
  pkg: PackageDto;
  selected: boolean;
  onSelect: () => void;
}

export function ResultRow({ leg, pkg, selected, onSelect }: ResultRowProps) {
  const badges = badgeLabels(pkg);
  return (
    <button
      type="button"
      tabIndex={-1}
      role="option"
      aria-selected={selected}
      onClick={onSelect}
      className={
        "row-in flex h-6 w-full cursor-pointer items-center gap-3 rounded-md border px-2 text-left transition-colors " +
        (selected
          ? "border-accent-secondary bg-accent-secondary/10"
          : "border-transparent hover:border-border hover:bg-card")
      }
      title={pkg.description ?? pkg.name}
    >
      <span className="min-w-44 truncate font-medium text-foreground">
        {pkg.name}
      </span>
      <span className="min-w-20 shrink-0 text-muted-foreground">{pkg.version}</span>
      <span className={`shrink-0 rounded bg-card-muted px-1.5 text-[10px] uppercase tracking-wider ${LEG_TONE[leg]}`}>
        {pkg.repo}
      </span>
      {pkg.installed && <span className="shrink-0 text-[10px] text-accent">[installed]</span>}
      {badges.length > 0 && (
        <span className="shrink-0 text-[10px] text-destructive">[{badges.join("][")}]</span>
      )}
      <span className="truncate text-muted-foreground">{pkg.description ?? ""}</span>
    </button>
  );
}
