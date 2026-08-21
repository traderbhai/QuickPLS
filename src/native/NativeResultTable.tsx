import { useMemo } from "react";
import type { ResultTable } from "../domain/resultTables";
import { resultTablePresentation } from "../domain/resultTablePresentation";
import { nativeGridClipboardText, useNativeScientificGrid } from "./nativeScientificGrid";

export interface NativeResultTableProps {
  table: ResultTable;
  gridKey: string;
  headingId: string;
  confidenceLevel?: number | null;
  onActiveRowChange?: (rowIndex: number) => void;
}

/**
 * Shared accessible renderer for string-backed native result tables. Semantic
 * data attributes keep responsive and sticky-column policy in CSS while the
 * archived scientific table remains byte-for-byte unchanged.
 */
export function NativeResultTable({
  table,
  gridKey,
  headingId,
  confidenceLevel,
  onActiveRowChange,
}: NativeResultTableProps) {
  const presentation = useMemo(
    () => resultTablePresentation(table, confidenceLevel),
    [confidenceLevel, table],
  );
  const grid = useNativeScientificGrid({
    gridKey,
    rowCount: table.rows.length,
    columnCount: table.columns.length,
    getClipboardText: ({ rowIndex, columnIndex }) => nativeGridClipboardText(table.rows[rowIndex]?.[columnIndex]),
    onActiveCellChange: ({ rowIndex }) => onActiveRowChange?.(rowIndex),
  });
  const instructionsId = `${headingId}-grid-instructions`;
  const overflowHintId = `${headingId}-overflow-hint`;
  const confidenceId = `${headingId}-confidence`;
  const describedBy = [
    instructionsId,
    presentation.hasHorizontalOverflowRisk ? overflowHintId : null,
    presentation.confidenceHeading ? confidenceId : null,
  ].filter(Boolean).join(" ");

  return <>
    {presentation.confidenceHeading ? <p
      className="nd-result-confidence-heading"
      id={confidenceId}
      role="note"
      data-result-confidence-heading="true"
    >{presentation.confidenceHeading}</p> : null}
    {presentation.hasHorizontalOverflowRisk ? <p
      className="nd-result-horizontal-scroll-hint"
      id={overflowHintId}
      data-result-horizontal-scroll-hint="true"
    >More columns are available horizontally.</p> : null}
    <div
      className="nd-table-scroll nd-result-table-scroll"
      role="region"
      aria-labelledby={headingId}
      aria-describedby={describedBy}
      data-result-horizontal-scroll="true"
    >
      <table
        ref={grid.tableRef}
        className="nd-result-table nd-scientific-grid"
        role="grid"
        aria-labelledby={headingId}
        aria-describedby={describedBy}
        aria-rowcount={table.rows.length + 1}
        aria-colcount={table.columns.length}
        aria-multiselectable="false"
        aria-keyshortcuts="Control+C"
        data-result-responsive-columns={presentation.hasLowerPriorityColumns ? "true" : "false"}
        data-result-overflow-risk={presentation.hasHorizontalOverflowRisk ? "true" : "false"}
        tabIndex={table.rows.length && table.columns.length ? undefined : 0}
        onKeyDown={grid.handleKeyDown}
      >
        <thead><tr role="row" aria-rowindex={1}>{table.columns.map((column, columnIndex) => {
          const columnPresentation = presentation.columns[columnIndex]!;
          return <th
            key={`${columnIndex}:${column}`}
            role="columnheader"
            scope="col"
            aria-colindex={columnIndex + 1}
            className={`nd-result-column nd-result-column--${columnPresentation.kind} nd-result-column--${columnPresentation.priority}`}
            data-result-column-kind={columnPresentation.kind}
            data-result-column-priority={columnPresentation.priority}
            data-result-identity-column={columnPresentation.sticky ? "true" : undefined}
          >{column}</th>;
        })}</tr></thead>
        <tbody>{table.rows.map((row, rowIndex) => {
          const rowPresentation = table.presentation?.rows?.[rowIndex];
          const hasModelFocus = Boolean(
            rowPresentation?.nodeIds?.length
            || rowPresentation?.relationIds?.length
            || rowPresentation?.interactionTermIds?.length,
          );
          return <tr
            role="row"
            aria-rowindex={rowIndex + 2}
            data-result-row-active={grid.activeCell.rowIndex === rowIndex ? "true" : undefined}
            data-result-row-model-focus={hasModelFocus ? "true" : undefined}
            key={rowPresentation?.key ?? rowIndex}
          >{table.columns.map((_column, columnIndex) => {
            const columnPresentation = presentation.columns[columnIndex]!;
            return <td
              {...grid.cellProps(rowIndex, columnIndex)}
              aria-colindex={columnIndex + 1}
              className={`nd-result-column nd-result-column--${columnPresentation.kind} nd-result-column--${columnPresentation.priority}`}
              data-result-column-kind={columnPresentation.kind}
              data-result-column-priority={columnPresentation.priority}
              data-result-identity-column={columnPresentation.sticky ? "true" : undefined}
              key={columnIndex}
            >{row[columnIndex] ?? ""}</td>;
          })}</tr>;
        })}</tbody>
      </table>
    </div>
    <span className="nd-sr-only" id={instructionsId}>Use the arrow keys to move between cells. Press Control+C to copy the selected cell.</span>
    <span className="nd-sr-only" role="status" aria-live="polite">{grid.announcement}</span>
  </>;
}

export default NativeResultTable;
