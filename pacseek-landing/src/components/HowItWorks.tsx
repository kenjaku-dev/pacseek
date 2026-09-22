import { Kbd } from "./ui/Kbd";
import { Section } from "./ui/Section";

const STEPS = [
  {
    step: "01",
    title: "Type a query",
    body: "Open pacseek (TTY defaults to TUI) or run pacseek neovim --no-tui. Repo and AUR search run in parallel.",
    keys: ["/"],
  },
  {
    step: "02",
    title: "Pick a result",
    body: "↑↓ or j/k to move. Press i for details — votes, maintainer, description — without leaving the list.",
    keys: ["j", "k", "i"],
  },
  {
    step: "03",
    title: "Enter to install",
    body: "Confirm once. Official packages go through pacman -S; AUR shows the PKGBUILD, then git clone + makepkg -si. Tab removes.",
    keys: ["Enter", "Tab"],
  },
] as const;

export function HowItWorks() {
  return (
    <Section
      id="how"
      labelledBy="how-heading"
      className="cv-section border-y border-border/50 bg-muted/20"
    >
      <div className="mx-auto w-full max-w-5xl px-5 py-16 sm:px-8 sm:py-24">
        <div className="mb-12 max-w-2xl">
          <p className="mb-3 text-[11px] font-medium uppercase tracking-[0.25em] text-accent">
            Workflow
          </p>
          <h2
            id="how-heading"
            className="text-balance text-2xl font-semibold tracking-tight text-foreground sm:text-3xl"
          >
            From query to installed
          </h2>
          <p className="mt-3 text-sm leading-relaxed text-muted-foreground sm:text-base">
            The same three moves whether the package lives in extra or on the
            AUR.
          </p>
        </div>

        <ol className="relative grid grid-cols-1 gap-10 md:grid-cols-3 md:gap-8">
          <span
            aria-hidden="true"
            className="absolute left-4 top-4 hidden h-px w-[calc(100%-2rem)] bg-gradient-to-r from-accent/50 via-border to-transparent md:block"
          />
          {STEPS.map((item) => (
            <li key={item.step} className="group relative">
              <div className="mb-4 flex items-center gap-3">
                <span
                  aria-hidden="true"
                  className="relative z-10 flex h-8 w-8 items-center justify-center rounded-lg border border-accent/40 bg-card text-xs font-semibold text-accent shadow-[0_0_16px_-4px] shadow-accent/40 transition-shadow duration-300 group-hover:shadow-accent/70"
                >
                  {item.step}
                </span>
                <span
                  aria-hidden="true"
                  className="h-px flex-1 bg-gradient-to-r from-border/60 to-transparent md:hidden"
                />
              </div>
              <h3 className="text-sm font-semibold text-foreground">
                {item.title}
              </h3>
              <p className="mt-2 text-[13px] leading-relaxed text-muted-foreground">
                {item.body}
              </p>
              <p className="mt-3 flex items-center gap-1.5" aria-hidden="true">
                {item.keys.map((key) => (
                  <Kbd key={key}>{key}</Kbd>
                ))}
              </p>
            </li>
          ))}
        </ol>
      </div>
    </Section>
  );
}
