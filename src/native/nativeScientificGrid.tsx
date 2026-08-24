import {
  useEffect,
  useId,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
  type RefObject,
  type TdHTMLAttributes,
} from "react";

export interface NativeGridPosition {
  rowIndex: number;
  columnIndex: number;
}

export interface NativeGridKeyModifiers {
  ctrlKey?: boolean;
  metaKey?: boolean;
}

export function nativeGridPositionForKey(
  current: NativeGridPosition,
  key: string,
  rowCount: number,
  columnCount: number,
  modifiers: NativeGridKeyModifiers = {},
): NativeGridPosition | null {
  if (rowCount < 1 || columnCount < 1) return null;
  const rowIndex = Math.min(rowCount - 1, Math.max(0, current.rowIndex));
  const columnIndex = Math.min(columnCount - 1, Math.max(0, current.columnIndex));
  const toBoundary = Boolean(modifiers.ctrlKey || modifiers.metaKey);

  switch (key) {
    case "ArrowLeft":
      return { rowIndex, columnIndex: Math.max(0, columnIndex - 1) };
    case "ArrowRight":
      return { rowIndex, columnIndex: Math.min(columnCount - 1, columnIndex + 1) };
    case "ArrowUp":
      return { rowIndex: Math.max(0, rowIndex - 1), columnIndex };
    case "ArrowDown":
      return { rowIndex: Math.min(rowCount - 1, rowIndex + 1), columnIndex };
    case "Home":
      return toBoundary ? { rowIndex: 0, columnIndex: 0 } : { rowIndex, columnIndex: 0 };
    case "End":
      return toBoundary
        ? { rowIndex: rowCount - 1, columnIndex: columnCount - 1 }
        : { rowIndex, columnIndex: columnCount - 1 };
    case "PageUp":
      return { rowIndex: 0, columnIndex };
    case "PageDown":
      return { rowIndex: rowCount - 1, columnIndex };
    default:
      return null;
  }
}

export function nativeGridClipboardText(value: unknown): string {
  return value == null ? "" : String(value);
}

async function writeNativeGridClipboard(text: string): Promise<boolean> {
  try {
    if (typeof navigator !== "undefined" && navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return true;
    }
  } catch {
    // Continue to the local document fallback when clipboard permission is unavailable.
  }

  if (typeof document === "undefined" || !document.body) return false;
  const field = document.createElement("textarea");
  field.value = text;
  field.setAttribute("readonly", "");
  field.style.position = "fixed";
  field.style.opacity = "0";
  field.style.pointerEvents = "none";
  document.body.appendChild(field);
  field.select();
  try {
    return document.execCommand("copy");
  } catch {
    return false;
  } finally {
    field.remove();
  }
}

interface UseNativeScientificGridOptions {
  gridKey: string;
  rowCount: number;
  columnCount: number;
  initialColumnIndex?: number;
  controlledColumnIndex?: number;
  /** Absolute source-row offset for announcements when the DOM is windowed. */
  rowIndexOffset?: number;
  getClipboardText: (position: NativeGridPosition) => string;
  onActiveCellChange?: (position: NativeGridPosition) => void;
}

export interface NativeScientificGridBinding {
  activeCell: NativeGridPosition;
  announcement: string;
  tableRef: RefObject<HTMLTableElement | null>;
  handleKeyDown: (event: ReactKeyboardEvent<HTMLTableElement>) => void;
  cellProps: (
    rowIndex: number,
    columnIndex: number,
  ) => TdHTMLAttributes<HTMLTableCellElement> & Record<`data-native-grid-${"row" | "column" | "cell"}`, string>;
}

function boundedPosition(
  position: NativeGridPosition,
  rowCount: number,
  columnCount: number,
): NativeGridPosition {
  return {
    rowIndex: Math.min(Math.max(0, rowCount - 1), Math.max(0, position.rowIndex)),
    columnIndex: Math.min(Math.max(0, columnCount - 1), Math.max(0, position.columnIndex)),
  };
}

export function useNativeScientificGrid({
  gridKey,
  rowCount,
  columnCount,
  initialColumnIndex = 0,
  controlledColumnIndex,
  rowIndexOffset = 0,
  getClipboardText,
  onActiveCellChange,
}: UseNativeScientificGridOptions): NativeScientificGridBinding {
  const instanceId = useId().replace(/:/g, "");
  const tableRef = useRef<HTMLTableElement>(null);
  const [activeCell, setActiveCell] = useState<NativeGridPosition>(() => boundedPosition(
    { rowIndex: 0, columnIndex: initialColumnIndex },
    rowCount,
    columnCount,
  ));
  const [announcement, setAnnouncement] = useState("");
  const gridKeyRef = useRef(gridKey);

  useEffect(() => {
    if (gridKeyRef.current === gridKey) return;
    gridKeyRef.current = gridKey;
    setActiveCell(boundedPosition(
      { rowIndex: 0, columnIndex: initialColumnIndex },
      rowCount,
      columnCount,
    ));
    setAnnouncement("");
  }, [columnCount, gridKey, initialColumnIndex, rowCount]);

  useEffect(() => {
    setActiveCell((current) => boundedPosition(current, rowCount, columnCount));
  }, [columnCount, rowCount]);

  useEffect(() => {
    if (controlledColumnIndex == null) return;
    setActiveCell((current) => boundedPosition(
      { ...current, columnIndex: controlledColumnIndex },
      rowCount,
      columnCount,
    ));
  }, [columnCount, controlledColumnIndex, rowCount]);

  const activateCell = (position: NativeGridPosition, focus: boolean) => {
    const next = boundedPosition(position, rowCount, columnCount);
    setActiveCell(next);
    onActiveCellChange?.(next);
    if (!focus) return;
    const cell = tableRef.current?.querySelector<HTMLElement>(
      `[data-native-grid-row="${next.rowIndex}"][data-native-grid-column="${next.columnIndex}"]`,
    );
    cell?.focus({ preventScroll: true });
    cell?.scrollIntoView({ block: "nearest", inline: "nearest" });
  };

  const handleKeyDown = (event: ReactKeyboardEvent<HTMLTableElement>) => {
    const target = event.target instanceof Element
      ? event.target.closest<HTMLElement>("[data-native-grid-cell]")
      : null;
    if (!target || !event.currentTarget.contains(target)) return;
    const rowIndex = Number(target.dataset.nativeGridRow);
    const columnIndex = Number(target.dataset.nativeGridColumn);
    if (!Number.isInteger(rowIndex) || !Number.isInteger(columnIndex)) return;
    const current = { rowIndex, columnIndex };

    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "c") {
      event.preventDefault();
      const text = getClipboardText(current);
      void writeNativeGridClipboard(text).then((copied) => {
        setAnnouncement(copied
          ? `Copied row ${rowIndexOffset + rowIndex + 1}, column ${columnIndex + 1}.`
          : "The selected cell could not be copied.");
      });
      return;
    }

    const next = nativeGridPositionForKey(
      current,
      event.key,
      rowCount,
      columnCount,
      { ctrlKey: event.ctrlKey, metaKey: event.metaKey },
    );
    if (!next) return;
    event.preventDefault();
    activateCell(next, true);
  };

  const cellProps = (rowIndex: number, columnIndex: number) => {
    const selected = activeCell.rowIndex === rowIndex && activeCell.columnIndex === columnIndex;
    return {
      id: `nd-grid-${instanceId}-${rowIndex}-${columnIndex}`,
      role: "gridcell",
      tabIndex: selected ? 0 : -1,
      "aria-selected": selected,
      "data-native-grid-cell": "true",
      "data-native-grid-row": String(rowIndex),
      "data-native-grid-column": String(columnIndex),
      onFocus: () => activateCell({ rowIndex, columnIndex }, false),
      onClick: () => activateCell({ rowIndex, columnIndex }, true),
    } satisfies TdHTMLAttributes<HTMLTableCellElement> & Record<`data-native-grid-${"row" | "column" | "cell"}`, string>;
  };

  return { activeCell, announcement, tableRef, handleKeyDown, cellProps };
}
