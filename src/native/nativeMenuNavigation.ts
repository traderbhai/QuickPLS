export type MenuItemNavigationKey = "ArrowDown" | "ArrowUp" | "Home" | "End";

export function nextMenuIndex(current: number, count: number, direction: -1 | 1): number {
  if (count <= 0) return -1;
  return (current + direction + count) % count;
}

export function nextEnabledItemIndex(
  disabled: readonly boolean[],
  current: number,
  key: MenuItemNavigationKey,
): number {
  const enabled = disabled
    .map((isDisabled, index) => isDisabled ? -1 : index)
    .filter((index) => index >= 0);
  if (!enabled.length) return -1;
  if (key === "Home") return enabled[0];
  if (key === "End") return enabled[enabled.length - 1];
  const position = enabled.indexOf(current);
  if (position < 0) return key === "ArrowUp" ? enabled[enabled.length - 1] : enabled[0];
  const direction = key === "ArrowUp" ? -1 : 1;
  return enabled[(position + direction + enabled.length) % enabled.length];
}

export function isContextMenuKeyboardGesture(key: string, shiftKey: boolean): boolean {
  return key === "ContextMenu" || (key === "F10" && shiftKey);
}

export function contextMenuCoordinates(
  requestedX: number,
  requestedY: number,
  viewportWidth: number,
  viewportHeight: number,
  menuWidth = 244,
  menuHeight = 240,
): { x: number; y: number } {
  return {
    x: Math.max(4, Math.min(requestedX, viewportWidth - menuWidth - 4)),
    y: Math.max(4, Math.min(requestedY, viewportHeight - menuHeight - 4)),
  };
}
