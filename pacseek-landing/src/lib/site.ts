export const SITE = {
  name: "pacseek",
  version: "0.3.0",
  license: "MIT",
  platforms: "Arch / Artix",
  tagline: "Search Arch packages at terminal speed.",
  github: "https://github.com/achraf/pacseek",
  githubReadme: "https://github.com/achraf/pacseek#readme",
  githubIssues: "https://github.com/achraf/pacseek/issues",
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
