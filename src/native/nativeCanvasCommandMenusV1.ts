import type { NativeCommandAction } from "./nativeCommands";

export interface NativeCanvasSplitMenuItemV1 {
  readonly id: string;
  readonly label: string;
  readonly action: NativeCommandAction;
  readonly separatorBefore?: boolean;
}

export const NATIVE_CANVAS_ARRANGE_MENU_V1: readonly NativeCanvasSplitMenuItemV1[] = [
  { id: "tidy-selection", label: "Tidy selection", action: { id: "model.arrange", strategy: "tidy-selection" } },
  { id: "align-left", label: "Align left", action: { id: "model.arrange", strategy: "align-left" }, separatorBefore: true },
  { id: "align-center", label: "Align center", action: { id: "model.arrange", strategy: "align-center" } },
  { id: "align-right", label: "Align right", action: { id: "model.arrange", strategy: "align-right" } },
  { id: "align-top", label: "Align top", action: { id: "model.arrange", strategy: "align-top" } },
  { id: "align-middle", label: "Align middle", action: { id: "model.arrange", strategy: "align-middle" } },
  { id: "align-bottom", label: "Align bottom", action: { id: "model.arrange", strategy: "align-bottom" } },
  { id: "distribute-horizontal", label: "Distribute horizontally", action: { id: "model.arrange", strategy: "distribute-horizontal" }, separatorBefore: true },
  { id: "distribute-vertical", label: "Distribute vertically", action: { id: "model.arrange", strategy: "distribute-vertical" } },
  { id: "arrange-horizontal", label: "Arrange model left-to-right", action: { id: "model.arrange", strategy: "model-horizontal" }, separatorBefore: true },
  { id: "arrange-vertical", label: "Arrange model top-to-bottom", action: { id: "model.arrange", strategy: "model-vertical" } },
];

export const NATIVE_CANVAS_FIT_MENU_V1: readonly NativeCanvasSplitMenuItemV1[] = [
  { id: "fit-structure", label: "Fit structure", action: { id: "model.fit", scope: "structure" } },
  { id: "fit-all", label: "Fit all", action: { id: "model.fit", scope: "all" } },
  { id: "fit-selection", label: "Fit selection", action: { id: "model.fit", scope: "selection" } },
];
