const NUMERIC_LITERAL = /^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/;

/**
 * Prevent spreadsheet applications from interpreting user-controlled text as a
 * formula while retaining genuine signed numeric cells as numbers.
 */
export function neutralizeSpreadsheetFormula(value: string): string {
  const trimmed = value.trimStart();
  if (!trimmed) return value;
  const prefix = trimmed[0];
  const formulaLike = prefix === "="
    || prefix === "@"
    || ((prefix === "+" || prefix === "-") && !NUMERIC_LITERAL.test(trimmed));
  return formulaLike ? `'${value}` : value;
}

export function spreadsheetSafeCsvCell(value: string): string {
  const safeValue = neutralizeSpreadsheetFormula(value);
  return /[",\r\n]/.test(safeValue)
    ? `"${safeValue.replaceAll('"', '""')}"`
    : safeValue;
}
