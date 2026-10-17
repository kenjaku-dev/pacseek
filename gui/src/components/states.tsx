// Designed states — empty/first-run, quick hint, and the status footer with
// the exact TUI status wording (parity contract).
import { kbd } from "./kbd";

export function EmptyState() {
  return (
    <div className="mx-auto flex max-w-md flex-col items-center gap-3 pt-24 text-center">
      <h2 className="text-sm font-semibold text-foreground">Search Arch, AUR and Flatpak</h2>
      <p className="text-[12px] leading-relaxed text-muted-foreground">
        Results stream in per source — the repo leg lands first, AUR and Flatpak follow
        as they arrive. Nothing is joined behind the scenes.
      </p>
      <ul className="mt-2 space-y-1.5 text-[11px] text-muted-foreground">
        <li>
          {kbd("/")} focus search · {kbd("Enter")} search now
        </li>
        <li>
          {kbd("↑")}/{kbd("↓")} rows · {kbd("Enter")} open detail
        </li>
        <li>
          {kbd("Esc")} close detail / clear · {kbd("?")} all shortcuts
        </li>
      </ul>
      <p className="mt-4 text-[11px] text-muted-foreground">
        Configured limit and sources come from the shared config
        <span className="text-foreground"> ~/.config/pakmap/config.toml</span>.
      </p>
    </div>
  );
}

export function QuickHint({ text }: { text: string }) {
  return (
    <p className="rounded-lg border border-border bg-card px-3 py-2.5 text-[12px] text-muted-foreground">
      {text}
    </p>
  );
}
