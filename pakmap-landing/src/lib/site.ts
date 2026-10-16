import { PAKMAP_VERSION } from "./version";

export const SITE = {
  name: "pakmap",
  // Single source of truth: ../Cargo.toml → src/lib/version.ts (generated).
  // Regenerate with `pnpm sync-version`; commit the file with version bumps.
  version: PAKMAP_VERSION,
  license: "MIT",
  platforms: "Arch / Artix",
  tagline: "Search Arch packages at terminal speed.",
  github: "https://github.com/kenjaku-dev/pakmap",
  githubReadme: "https://github.com/kenjaku-dev/pakmap#readme",
  githubIssues: "https://github.com/kenjaku-dev/pakmap/issues",
  year: 2026,
} as const;

export const NAV_ITEMS = [
  { href: "#features", label: "Features" },
  { href: "#how", label: "How it works" },
  { href: "#install", label: "Install" },
] as const;

export const FOOTER_COLUMNS = [
  {
    title: "Product",
    links: [
      { href: "#features", label: "Features" },
      { href: "#how", label: "How it works" },
      { href: "#install", label: "Install" },
    ],
  },
  {
    title: "Resources",
    links: [
      { href: SITE.githubReadme, label: "Documentation", external: true },
      { href: SITE.github, label: "Source code", external: true },
      { href: SITE.githubIssues, label: "Issues", external: true },
    ],
  },
] as const;
