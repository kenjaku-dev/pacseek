// ConfirmDialog — every mutating operation passes through here. Risk copy is
// per action (AUR builds and package removal say what the transaction will
// actually do); Enter confirms, Esc cancels. Readonly setups never see a
// confirm: the actions are disabled up-front.
import { ModalDialog } from "./dialog";

export interface ConfirmSpec {
  title: string;
  body: string[];
  /** danger styles the confirm button destructive (remove/upgrade) */
  danger?: boolean;
  confirmLabel: string;
}

export function ConfirmDialog({
  spec,
  onConfirm,
  onClose,
}: {
  spec: ConfirmSpec;
  onConfirm: () => void;
  onClose: () => void;
}) {
  return (
    <ModalDialog label={spec.title} onClose={onClose}>
      <h2 className="mb-3 text-sm font-semibold">{spec.title}</h2>
      <div className="space-y-1.5 text-[12px] text-muted-foreground">
        {spec.body.map((line, i) => (
          <p key={i}>{line}</p>
        ))}
      </div>
      <div className="mt-4 flex items-center justify-between gap-3">
        <span className="text-[10px] text-muted-foreground">
          <kbd>Esc</kbd> cancels
        </span>        <div className="flex gap-2">
          <button
            type="button"
            onClick={onClose}
            className="cursor-pointer rounded border border-border px-3 py-1.5 text-[12px] text-foreground transition-colors hover:bg-card-muted"
          >
            Cancel
          </button>
          <button
            type="button"
            data-autofocus
            onClick={onConfirm}
            className={`cursor-pointer rounded px-3 py-1.5 text-[12px] font-semibold text-white transition-colors ${
              spec.danger
                ? "bg-destructive-strong hover:bg-destructive/90"
                : "bg-accent-tertiary-strong hover:bg-accent-tertiary/90"
            }`}
          >
            {spec.confirmLabel}
          </button>
        </div>
      </div>
    </ModalDialog>
  );
}
