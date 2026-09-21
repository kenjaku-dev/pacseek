import { CopyButton } from "./ui/CopyButton";
import { Section } from "./ui/Section";

const BLOCKS = [
  {
    label: "From source (recommended)",
    command: `git clone https://github.com/achraf/pacseek
cd pacseek
cargo install --path . --force`,
  },
  {
    label: "Release binary",
    command: `cargo build --release
sudo install -Dm755 target/release/pacseek /usr/local/bin/pacseek`,
  },
  {
    label: "Everyday use",
    command: `pacseek                 # TUI (TTY)
pacseek firefox --no-tui  # CLI / scripts
pacseek --init-config     # write config.toml`,
  },
] as const;

const DEPS =
  "Requires pacman 5.1+ (libalpm), pacman-contrib, git, base-devel for AUR builds, Rust 1.85+.";

export function Install() {
  return (
    <Section
      id="install"
      labelledBy="install-heading"
      className="mx-auto w-full max-w-5xl px-5 py-16 sm:px-8 sm:py-20"
    >
      <div className="mb-10 max-w-2xl">
        <h2
          id="install-heading"
          className="text-2xl font-semibold tracking-tight text-foreground sm:text-3xl"
        >
          Install in one go
        </h2>
        <p className="mt-3 text-sm leading-relaxed text-muted-foreground sm:text-base">
          Stripped release build is a few megabytes. Refresh sync DB once before
          the first repo search:{" "}
          <code className="text-accent-secondary">sudo pacman -Sy</code>
        </p>
      </div>

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-3">
        {BLOCKS.map((block) => (
          <div
            key={block.label}
            className="flex flex-col rounded-lg border border-border/70 bg-card p-4"
          >
            <div className="mb-3 flex items-start justify-between gap-3">
              <h3 className="text-xs font-medium text-muted-foreground">
                {block.label}
              </h3>
              <CopyButton text={block.command} label={`Copy ${block.label}`} />
            </div>
            <pre className="flex-1 overflow-x-auto text-[11px] leading-relaxed text-foreground sm:text-xs">
              <code>{block.command}</code>
            </pre>
          </div>
        ))}
      </div>

      <p className="mt-5 text-xs leading-relaxed text-muted-foreground">
        {DEPS}
      </p>
    </Section>
  );
}
