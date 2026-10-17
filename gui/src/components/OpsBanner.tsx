// OpsBanner — the operation journal: spinner + label + current phase + the
// live line, expandable journal, cancel button (phase-boundary semantics
// spelled out in the copy), and the terminal state with dismiss. aria-live
// delivers phase transitions to AT without stealing focus (usability: the
// user keeps searching while an op runs).
import { useState } from "react";
import { ChevronDown, ChevronUp, X } from "lucide-react";
import type { OpsState } from "../state/opsReducer";

const SPIN = "inline-block animate-spin";

export function OpsBanner({
  ops,
  onCancel,
  onDismiss,
}: {
  ops: OpsState;
  onCancel: () => void;
  onDismiss: () => void;
}) {
  const op = ops.op;
  if (op === null) return null;

  const running = op.status === "running" || op.status === "canceling";
  const lastLine = op.lines[op.lines.length - 1] ?? "";
  const failed = op.status === "failed";

  return (
    <div
      role="status"
      aria-live="polite"
      aria-label={running ? "Operation progress" : "Operation result"}
      data-testid="ops-banner"
      className="banner-in border-t border-border bg-card px-4 py-2"
    >
      <div className="flex items-center gap-3">
        {running ? (
          <span
            className={SPIN}
            style={{ borderTop: "2px solid currentColor", borderRight: "2px solid transparent", width: 12, height: 12, borderRadius: "50%" }}
            aria-hidden="true"
          />
        ) : (
          <span
            aria-hidden="true"
            className={failed ? "text-destructive" : "text-accent-tertiary"}
          >
            {failed ? "✕" : "✓"}
          </span>
        )}

        <span className="max-w-40 shrink-0 truncate text-xs font-semibold text-foreground">
          {op.label}
        </span>

        <span className="shrink-0 rounded bg-card-muted px-1.5 text-2xs uppercase tracking-wide text-muted-foreground">
          {op.phase}
        </span>

        {running ? (
          <span className="min-w-0 flex-1 truncate text-nano text-muted-foreground">
            {op.status === "canceling"
              ? "canceling before the next phase — the running transaction is never killed mid-flight"
              : lastLine}
          </span>
        ) : (
          <span className="min-w-0 flex-1 truncate text-nano">
            {op.status === "ok" ? (
              <span className="text-muted-foreground">
                done{op.exit_code !== null ? ` (exit ${op.exit_code})` : ""} — remember to re-search
                for the current state
              </span>
            ) : (
              <span className="text-destructive">
                <strong>{op.error?.code ?? "op_failed"}</strong>{" "}
                {op.error?.message ?? "operation failed"}
                {op.error?.hint && (
                  <span className="text-muted-foreground"> — hint: {op.error.hint}</span>
                )}
              </span>
            )}
          </span>
        )}

        {running && (
          <button
            type="button"
            onClick={onCancel}
            disabled={op.status === "canceling"}
            className="shrink-0 cursor-pointer rounded border border-border px-2 py-0.5 text-nano text-foreground transition-colors hover:bg-card-muted disabled:cursor-default disabled:opacity-50"
          >
            Cancel
          </button>
        )}

        {op.lines.length > 0 && <JournalToggle lines={op.lines} running={running} />}

        {!running && (
          <button
            type="button"
            aria-label="Dismiss operation result"
            onClick={onDismiss}
            className="shrink-0 cursor-pointer rounded p-1 text-muted-foreground transition-colors hover:text-foreground"
          >
            <X size={13} aria-hidden="true" />
          </button>
        )}
      </div>

      {op.status === "failed" && !running && (
        <p className="mt-0.5 text-2xs text-muted-foreground">
          exit code {op.exit_code ?? "none"} · the transaction was never killed mid-flight ·
          classified from the operation journal
        </p>
      )}
    </div>
  );
}

function JournalToggle({ lines, running }: { lines: string[]; running: boolean }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="shrink-0">
      <button
        type="button"
        aria-expanded={open}
        aria-label={open ? "Collapse operation journal" : "Expand operation journal"}
        onClick={() => setOpen((v) => !v)}
        className="cursor-pointer rounded p-1 text-muted-foreground transition-colors hover:text-foreground"
      >
        {open ? <ChevronUp size={13} aria-hidden="true" /> : <ChevronDown size={13} aria-hidden="true" />}
      </button>
      {open && (
        <pre
          data-testid="op-journal"
          className="mt-2 max-h-40 overflow-y-auto rounded bg-card-muted p-2 text-left text-2xs leading-4 text-muted-foreground"
        >
          {lines.join("\n")}
          {running && "\n…"}
        </pre>
      )}
    </div>
  );
}
