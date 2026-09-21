import {
  Gauge,
  Layers,
  PackageCheck,
  Paintbrush,
  ShieldCheck,
  SlidersHorizontal,
} from "lucide-react";
import { Section } from "./ui/Section";

const FEATURES = [
  {
    icon: Gauge,
    title: "One-shot CLI",
    description:
      "pacseek firefox runs repo search first, AUR after — parallel tokio, libalpm local DB. Pipe with --json, filter with --regex and --limit.",
    wide: true,
  },
  {
    icon: Layers,
    title: "TUI that matches your drawing",
    description:
      "Search bar, virtualized results, Enter to install, i for info, ? for help.",
    wide: false,
  },
  {
    icon: PackageCheck,
    title: "Install and remove",
    description:
      "Tab switches to Installed mode — filter, then Enter/d/x to remove with a confirm popup.",
    wide: false,
  },
  {
    icon: ShieldCheck,
    title: "AUR safety built in",
    description:
      "PKGBUILD shown before makepkg, namcap when available, makepkg never runs as root. HTTPS sources only.",
    wide: true,
  },
  {
    icon: Paintbrush,
    title: "Themeable config",
    description:
      "Colors, borders, layout, timeouts in ~/.config/pacseek/config.toml — no recompile.",
    wide: false,
  },
  {
    icon: SlidersHorizontal,
    title: "Script-friendly flags",
    description:
      "--source, --by, --installed-only, --bottom-up, --no-tui, --remove PKG.",
    wide: false,
  },
] as const;

export function Features() {
  return (
    <Section
      id="features"
      labelledBy="features-heading"
      className="mx-auto w-full max-w-5xl px-5 py-16 sm:px-8 sm:py-20"
    >
      <div className="mb-10 max-w-2xl">
        <h2
          id="features-heading"
          className="text-2xl font-semibold tracking-tight text-foreground sm:text-3xl"
        >
          Everything in one binary
        </h2>
        <p className="mt-3 text-sm leading-relaxed text-muted-foreground sm:text-base">
          Search, inspect, install, remove. No helper daemons, no root TUI —
          sudo only when pacman needs it.
        </p>
      </div>

      <ul className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {FEATURES.map((feature) => {
          const Icon = feature.icon;
          return (
            <li
              key={feature.title}
              className={`group rounded-lg border border-border/70 bg-card p-5 transition-colors duration-200 hover:border-accent/40 ${
                feature.wide ? "sm:col-span-2" : ""
              }`}
            >
              <div className="mb-3 flex h-9 w-9 items-center justify-center rounded-md border border-border bg-muted text-accent transition-colors duration-200 group-hover:border-accent/40">
                <Icon aria-hidden="true" className="h-4.5 w-4.5" />
              </div>
              <h3 className="text-sm font-semibold text-foreground">
                {feature.title}
              </h3>
              <p className="mt-2 text-[13px] leading-relaxed text-muted-foreground">
                {feature.description}
              </p>
            </li>
          );
        })}
      </ul>
    </Section>
  );
}
