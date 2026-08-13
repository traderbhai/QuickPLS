import { describe, expect, it } from "vitest";
import { neutralizeSpreadsheetFormula, spreadsheetSafeCsvCell } from "./spreadsheetSafety";

describe("spreadsheet export safety", () => {
  it("neutralizes formula-like text including whitespace-prefixed payloads", () => {
    expect(neutralizeSpreadsheetFormula("=HYPERLINK(\"https://example.invalid\")")).toBe("'=HYPERLINK(\"https://example.invalid\")");
    expect(neutralizeSpreadsheetFormula("+SUM(1,2)")).toBe("'+SUM(1,2)");
    expect(neutralizeSpreadsheetFormula("-cmd|' /C calc'!A0")).toBe("'-cmd|' /C calc'!A0");
    expect(neutralizeSpreadsheetFormula("  @SUM(1,2)")).toBe("'  @SUM(1,2)");
    expect(neutralizeSpreadsheetFormula("\t=1+1")).toBe("'\t=1+1");
  });

  it("retains ordinary labels and signed numeric output", () => {
    expect(neutralizeSpreadsheetFormula("Construct A")).toBe("Construct A");
    expect(neutralizeSpreadsheetFormula("-0.1234")).toBe("-0.1234");
    expect(neutralizeSpreadsheetFormula("+1.2e-5")).toBe("+1.2e-5");
    expect(neutralizeSpreadsheetFormula("0")).toBe("0");
  });

  it("applies formula neutralization before RFC-style CSV quoting", () => {
    expect(spreadsheetSafeCsvCell("=SUM(1,2)")).toBe("\"'=SUM(1,2)\"");
    expect(spreadsheetSafeCsvCell('A "quoted" label')).toBe('"A ""quoted"" label"');
    expect(spreadsheetSafeCsvCell("line one\nline two")).toBe('"line one\nline two"');
  });
});
