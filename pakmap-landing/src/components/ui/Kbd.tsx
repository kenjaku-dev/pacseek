export function Kbd({ children }: { children: React.ReactNode }) {
  return (
    <kbd className="inline-flex h-5 min-w-5 items-center justify-center rounded border border-border/80 bg-muted px-1.5 text-[10px] font-medium leading-none text-foreground shadow-[inset_0_-1.5px_0_0_rgba(0,0,0,0.45)]">
      {children}
    </kbd>
  );
}
