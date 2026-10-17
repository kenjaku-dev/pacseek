// kbd — consistent keyboard-cap chip used in hints and the shortcut overlay.
export function kbd(children: React.ReactNode) {
  return (
    <kbd className="rounded border border-border bg-card-muted px-1 py-0.5 font-mono text-2xs text-foreground">
      {children}
    </kbd>
  );
}
