import { Section } from "./ui/Section";

const STEPS = [
  {
    step: "01",
    title: "Type a query",
    body: "Open pacseek (TTY defaults to TUI) or run pacseek neovim --no-tui. Repo and AUR search run in parallel.",
  },
  {
    step: "02",
    title: "Pick a result",
    body: "↑↓ or j/k to move. Press i for details — votes, maintainer, description — without leaving the list.",
  },
  {
    step: "03",
    title: "Enter to install",
    body: "Confirm once. Official packages go through pacman -S; AUR shows the PKGBUILD, then git clone + makepkg -si. Tab removes.",
  },
] as const;

export function HowItWorks() {
  return (
    <Section
      id="how"
      labelledBy="how-heading"
      className="border-y border-border/50 bg-muted/30"
    >
      <div className="mx-auto w-full max-w-5xl px-5 py-16 sm:px-8 sm:py-20">
        <div className="mb-10 max-w-2xl">
          <h2
            id="how-heading"
            className="text-2xl font-semibold tracking-tight text-foreground sm:text-3xl"
          >
            From query to installed
          </h2>
          <p className="mt-3 text-sm leading-relaxed text-muted-foreground sm:text-base">
            The same three moves whether the package lives in extra or on the AUR.
          </p>
        </div>

        <ol className="grid grid-cols-1 gap-6 md:grid-cols-3 md:gap-8">
          {STEPS.map((item) => (
            <li key={item.step} className="relative">
              <div className="mb-3 flex items-center gap-3">
                <span
                  aria-hidden="true"
                  className="flex h-8 w-8 items-center justify-center rounded-md border border-accent/40 bg-card text-xs font-semibold text-accent"
                >
                  {item.step}
                </span>
                <span
                  aria-hidden="true"
                  className="hidden h-px flex-1 bg-border/60 md:block"
                />
              </div>
              <h3 className="text-sm font-semibold text-foreground">
                {item.title}
              </h3>
              <p className="mt-2 text-[13px] leading-relaxed text-muted-foreground">
                {item.body}
              </p>
            </li>
          ))}
        </ol>
      </div>
    </Section>
  );
}
