// ErrorCard — the shared failure surface: code (strong), message, hint. The
// same component backs search errors (App) and per-leg degraded update legs.
export function ErrorCard({ code, message, hint }: { code: string; message: string; hint: string | null }) {
  return (
    <div
      role="alert"
      className="flex items-start gap-3 border-b border-destructive/40 bg-destructive/10 px-4 py-2.5"
    >
      <div className="min-w-0">
        <p className="text-[12px]">
          <span className="font-semibold text-destructive">{code}</span>{" "}
          <span className="text-foreground">{message}</span>
        </p>
        {hint && <p className="mt-0.5 text-[11px] text-muted-foreground">hint: {hint}</p>}
      </div>
    </div>
  );
}
