export interface NativeIndicatorSelectionV1 {
  readonly selected: readonly string[];
  readonly anchor: string | null;
}

export interface NativeIndicatorSelectionInputV1 {
  readonly visible: readonly string[];
  readonly current: NativeIndicatorSelectionV1;
  readonly indicator: string;
  readonly toggle: boolean;
  readonly range: boolean;
}

/** Windows-style list selection with deterministic visible-order output. */
export function nextNativeIndicatorSelectionV1(
  input: NativeIndicatorSelectionInputV1,
): NativeIndicatorSelectionV1 {
  const visible = [...new Set(input.visible)];
  if (!visible.includes(input.indicator)) return input.current;
  const selected = new Set(input.current.selected.filter((value) => visible.includes(value)));
  if (input.range && input.current.anchor && visible.includes(input.current.anchor)) {
    const start = visible.indexOf(input.current.anchor);
    const end = visible.indexOf(input.indicator);
    const range = visible.slice(Math.min(start, end), Math.max(start, end) + 1);
    if (!input.toggle) selected.clear();
    for (const value of range) selected.add(value);
    return { selected: visible.filter((value) => selected.has(value)), anchor: input.current.anchor };
  }
  if (input.toggle) {
    if (selected.has(input.indicator)) selected.delete(input.indicator);
    else selected.add(input.indicator);
    return { selected: visible.filter((value) => selected.has(value)), anchor: input.indicator };
  }
  return { selected: [input.indicator], anchor: input.indicator };
}

export function nativeIndicatorDragSelectionV1(
  selected: readonly string[],
  dragged: string,
): readonly string[] {
  return selected.includes(dragged) ? [...selected] : [dragged];
}

export function nativeIndicatorDragLabelV1(indicators: readonly string[]): string {
  const concise = indicators.slice(0, 3).join(", ");
  const remaining = Math.max(0, indicators.length - 3);
  return `${indicators.length} indicator${indicators.length === 1 ? "" : "s"}: ${concise}${remaining ? `, +${remaining}` : ""}`;
}
