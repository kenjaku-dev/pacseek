// useOps — owner hook for the single active operation. The backend dispatches
// detached; every op_id gets its own channel, the reducer guards ids, and
// terminal `Done` always arrives exactly once (backend contract).
import { useCallback, useMemo, useReducer, useRef } from "react";
import {
  opsReducer,
  opLabel,
  type OpsAction,
  type OpsState,
} from "../state/opsReducer";
import { cancelOperation, dropOperation, runOp } from "./ipc";
import type { OpRequest } from "../types";

export interface OpsApi {
  state: OpsState;
  run: (request: OpRequest, label?: string) => Promise<void>;
  cancel: () => Promise<void>;
  dismiss: () => void;
}

export function useOps(): OpsApi {
  const [state, dispatch] = useReducer(
    (s: OpsState, a: OpsAction) => opsReducer(s, a),
    { op: null } as OpsState,
  );

  // one op at a time: pending start while one is live is rejected here
  const busyId = useRef<number | null>(null);
  const latestId = useRef(0);

  const run = useCallback(async (request: OpRequest, label?: string) => {
    const id = await runOp(request, (msg) => dispatch({ type: "message", msg }));
    latestId.current = id;
    busyId.current = id;
    dispatch({ type: "start", id, label: label ?? opLabel(request), request });
  }, []);

  const cancel = useCallback(async () => {
    const id = busyId.current;
    if (id === null) return;
    const known = await cancelOperation(id);
    if (known) dispatch({ type: "cancel_sent", id });
  }, []);

  const dismiss = useCallback(() => {
    const cur = state.op;
    if (cur === null) return;
    if (cur.status === "running" || cur.status === "canceling") return; // protected
    dispatch({ type: "dismiss" });
    void dropOperation(cur.id); // free the backend cancel-flag entry
  }, [state.op]);

  return useMemo(
    () => ({ state, run, cancel, dismiss }),
    [state, run, cancel, dismiss],
  );
}
