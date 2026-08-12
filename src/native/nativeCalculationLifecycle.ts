import type { RunMonitorState, RunMonitorStatus } from "../types";

export const NATIVE_CALCULATION_STATUSES = [
  "idle",
  "blocked",
  "queued",
  "validating",
  "running",
  "cancelling",
  "completed",
  "failed",
  "cancelled",
] as const satisfies readonly RunMonitorStatus[];

const ALLOWED_NEXT_STATUSES = {
  idle: ["blocked", "queued"],
  blocked: ["idle", "queued"],
  queued: ["validating", "running", "cancelling", "failed"],
  validating: ["queued", "running", "cancelling", "failed"],
  running: ["cancelling", "completed", "failed", "cancelled"],
  cancelling: ["running", "completed", "failed", "cancelled"],
  completed: ["idle", "blocked", "queued"],
  failed: ["idle", "blocked", "queued"],
  cancelled: ["idle", "blocked", "queued"],
} as const satisfies Record<RunMonitorStatus, readonly RunMonitorStatus[]>;

export type CalculationMonitorPatch =
  & Partial<Omit<RunMonitorState, "status" | "logs">>
  & Pick<RunMonitorState, "status">;

export class InvalidCalculationTransitionError extends Error {
  readonly from: RunMonitorStatus;
  readonly to: RunMonitorStatus;

  constructor(from: RunMonitorStatus, to: RunMonitorStatus) {
    super(`Invalid calculation state transition: ${from} -> ${to}`);
    this.name = "InvalidCalculationTransitionError";
    this.from = from;
    this.to = to;
  }
}

export function isCalculationActive(status: RunMonitorStatus): boolean {
  return status === "queued"
    || status === "validating"
    || status === "running"
    || status === "cancelling";
}

export function nativeCalculationPhaseLabel(
  phase: string | null | undefined,
  status: RunMonitorStatus,
): string {
  const normalized = (phase ?? "").trim().replace(/[_-]+/g, " ").replace(/\s+/g, " ");
  const fallback = status.replace(/[_-]+/g, " ");
  const value = normalized || fallback;
  return value ? `${value.charAt(0).toUpperCase()}${value.slice(1)}` : "Ready";
}

export function isAllowedCalculationTransition(from: RunMonitorStatus, to: RunMonitorStatus): boolean {
  if (from === to) return true;
  return (ALLOWED_NEXT_STATUSES[from] as readonly RunMonitorStatus[]).includes(to);
}

export function transitionCalculationMonitor(
  current: Readonly<RunMonitorState>,
  patch: CalculationMonitorPatch,
): RunMonitorState {
  if (!isAllowedCalculationTransition(current.status, patch.status)) {
    throw new InvalidCalculationTransitionError(current.status, patch.status);
  }
  return { ...current, ...patch };
}
