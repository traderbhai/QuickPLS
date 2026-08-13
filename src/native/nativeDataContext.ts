import type { NativeCommandContext } from "./nativeCommands";

export type NativeDataContextTarget =
  | { kind: "variable"; column: string }
  | { kind: "dataset" }
  | { kind: "none" };

export interface NativeDataContextMenuRequest {
  clientX: number;
  clientY: number;
  returnFocus: HTMLElement | null;
  target: NativeDataContextTarget;
}

/** Converts an exact Data-surface target into the registry's selection vocabulary. */
export function nativeDataContextSelection(
  target: Readonly<NativeDataContextTarget>,
): NativeCommandContext["selection"] {
  if (target.kind === "variable") return { kind: "variable", count: 1 };
  if (target.kind === "dataset") return { kind: "dataset", count: 1 };
  return { kind: "none", count: 0 };
}
