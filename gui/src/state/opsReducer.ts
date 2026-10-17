// opsReducer — single active operation (one at a time, matching the footer
// journal model). Progress lines accumulate into a capped ring; Done is
// terminal and late progress/cancel traffic for a different (or finished) op
// id is ignored — the same generation-guard idea the search reducer uses.
import type { GuiError, OpMessage, OpRequest } from "../types";

/** journal ring cap — the operation journal stays readable in the UI */
export const MAX_OP_LINES = 100;

export type OpStatus = "running" | "canceling" | "ok" | "failed";

export interface ActiveOp {
  id: number;
  label: string;
  /** human summary for the banner (e.g. "Install firefox") */
  phase: string;
  lines: string[];
  status: OpStatus;
  exit_code: number | null;
  error: GuiError | null;
}

export interface OpsState {
  op: ActiveOp | null;
}

export type OpsAction =
  | { type: "start"; id: number; label: string; request: OpRequest }
  | { type: "message"; msg: OpMessage }
  | { type: "cancel_sent"; id: number }
  | { type: "dismiss" };

export function opsReducer(state: OpsState, action: OpsAction): OpsState {
  switch (action.type) {
    case "start":
      return {
        op: {
          id: action.id,
          label: action.label,
          phase: "prepare",
          lines: [],
          status: "running",
          exit_code: null,
          error: null,
        },
      };

    case "message": {
      const cur = state.op;
      if (cur === null) return state;
      if (cur.id !== action.msg.op_id) return state; // generation guard: wrong op
      switch (action.msg.kind) {
        case "progress":
          // late progress for a terminal op would resurrect a journal — drop it
          if (cur.status === "ok" || cur.status === "failed") return state;
          return {
            ...state,
            op: {
              ...cur,
              phase: action.msg.phase,
              lines: [...cur.lines, action.msg.line].slice(-MAX_OP_LINES),
            },
          };
        case "done":
          // terminal exactly once — a second done for the same id is dropped
          if (cur.status === "ok" || cur.status === "failed") return state;
          return {
            ...state,
            op: {
              ...cur,
              status: action.msg.ok ? "ok" : "failed",
              exit_code: action.msg.exit_code,
              error: action.msg.error,
              phase: action.msg.ok ? "done" : "error",
            },
          };
      }
      return state; // unreachable exhaustiveness guard
    }

    case "cancel_sent": {
      const cur = state.op;
      if (cur === null || cur.id !== action.id) return state;
      if (cur.status !== "running") return state;
      return {
        ...state,
        op: { ...cur, status: "canceling" },
      };
    }

    case "dismiss":
      // dismissing a RUNNING op is illegal (hook rejects) — clear only terminal
      if (state.op !== null && state.op.status === "running") return state;
      if (state.op !== null && state.op.status === "canceling") return state;
      return { op: null };
  }
}

/** Short human label for the banner, derived from the request. */
export function opLabel(req: OpRequest): string {
  let noun: string;
  let target: string | null = null;
  switch (req.kind) {
    case "install_repo":
      noun = "Install";
      target = req.args.name;
      break;
    case "install_flatpak":
      noun = "Install flatpak";
      target = req.args.app_id;
      break;
    case "install_aur":
      noun = "Build & install (AUR)";
      target = req.args.name;
      break;
    case "remove":
      noun = "Remove";
      target = req.args.name;
      break;
    case "remove_flatpak":
      noun = "Remove flatpak";
      target = req.args.app_id;
      break;
    case "remove_orphans":
      noun = "Remove orphans";
      break;
    case "upgrade":
      noun = "Full system upgrade";
      break;
    case "refresh":
      noun = "Refresh databases";
      break;
    case "clean":
      noun = "Clean package cache";
      break;
  }
  return target === null ? noun : `${noun} ${target}`;
}
