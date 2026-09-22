import { BackToTop } from "./chrome/BackToTop";
import { Logo } from "./chrome/Logo";
import { FOOTER_COLUMNS, SITE } from "@/lib/site";

export function Footer() {
  return (
    <footer className="relative border-t border-border/60">
      <div
        aria-hidden="true"
        className="absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-accent/50 to-transparent"
      />
      <div className="mx-auto w-full max-w-5xl px-5 pb-8 pt-12 sm:px-8">
        <div className="grid grid-cols-1 gap-10 sm:grid-cols-[1.4fr_1fr_1fr]">
          <div className="max-w-sm">
            <div className="mb-3">
              <Logo />
            </div>
            <p className="text-sm leading-relaxed text-muted-foreground">
              Fast package search for Arch and Artix — official repos and the AUR
              in one Rust binary, with a terminal UI to install and remove.
            </p>
            <p className="mt-4 text-xs text-muted-foreground/80">
              Built with Rust, ratatui, and libalpm.
            </p>
          </div>

          {FOOTER_COLUMNS.map((column) => (
            <nav key={column.title} aria-label={column.title}>
              <h2 className="mb-3 text-xs font-semibold text-foreground">
                {column.title}
              </h2>
              <ul className="flex list-none flex-col gap-2">
                {column.links.map((link) => (
                  <li key={link.href}>
                    <a
                      href={link.href}
                      {...("external" in link && link.external
                        ? { target: "_blank", rel: "noopener noreferrer" }
                        : {})}
                      className="inline-flex min-h-8 items-center text-sm text-muted-foreground transition-colors duration-200 hover:text-foreground"
                    >
                      {link.label}
                    </a>
                  </li>
                ))}
              </ul>
            </nav>
          ))}
        </div>

        <div className="mt-10 flex flex-col gap-4 border-t border-border/50 pt-6 sm:flex-row sm:items-center sm:justify-between">
          <p className="text-xs text-muted-foreground">
            {SITE.name} v{SITE.version} is released under the{" "}
            <a
              href={`${SITE.github}/blob/main/LICENSE`}
              target="_blank"
              rel="noopener noreferrer"
              className="text-foreground underline decoration-border underline-offset-2 transition-colors duration-200 hover:decoration-accent"
            >
              {SITE.license} license
            </a>
            . Made for {SITE.platforms}.
          </p>
          <BackToTop />
        </div>
      </div>
    </footer>
  );
}
