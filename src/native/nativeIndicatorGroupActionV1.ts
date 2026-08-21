import type { DiagramPoint, ModelEditCommandV1 } from "../types";

export type NativeIndicatorGroupActionV1 =
  | {
    readonly kind: "create_construct";
    readonly constructId: string;
    readonly label: string;
    readonly position?: DiagramPoint;
  }
  | {
    readonly kind: "assign_indicators";
    readonly constructId: string;
  };

export type NativeIndicatorGroupActionPlanV1 =
  | {
    readonly status: "ready";
    readonly indicatorCount: number;
    readonly command: Extract<ModelEditCommandV1, { kind: "add_construct" | "assign_indicators" }>;
  }
  | {
    readonly status: "blocked";
    readonly code: "empty_selection" | "invalid_construct" | "invalid_label";
    readonly message: string;
  };

/**
 * Converts one Windows multi-selection into exactly one model-edit command.
 * Dataset order wins, duplicates disappear, and a reserved grouping column is
 * never silently assigned as an indicator.
 */
export function planNativeIndicatorGroupActionV1(
  visibleColumns: readonly string[],
  selectedColumns: readonly string[],
  action: NativeIndicatorGroupActionV1,
  reservedColumn: string | null = null,
): NativeIndicatorGroupActionPlanV1 {
  const selected = new Set(selectedColumns);
  const columns = [...new Set(visibleColumns)]
    .filter((column) => selected.has(column) && column !== reservedColumn);
  if (!columns.length) {
    return { status: "blocked", code: "empty_selection", message: "Select one or more available indicators." };
  }
  const constructId = action.constructId.trim();
  if (!constructId) {
    return { status: "blocked", code: "invalid_construct", message: "Choose or create a construct first." };
  }
  if (action.kind === "assign_indicators") {
    return {
      status: "ready",
      indicatorCount: columns.length,
      command: { kind: "assign_indicators", constructId, columns },
    };
  }
  const label = action.label.trim();
  if (!label) {
    return { status: "blocked", code: "invalid_label", message: "Enter a construct name." };
  }
  return {
    status: "ready",
    indicatorCount: columns.length,
    command: {
      kind: "add_construct",
      constructId,
      label,
      columns,
      ...(action.position ? { position: { ...action.position } } : {}),
    },
  };
}
