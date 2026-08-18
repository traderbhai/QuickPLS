import { describe, expect, it } from "vitest";
import appSource from "./NativePrototypeApp.tsx?raw";
import adapterSource from "./nativePrototypeAdapters.ts?raw";
import { fallbackNativePrototypeData, trustRows } from "./nativePrototypeData";

const withoutDeveloperDiagnostics = appSource.replace(
  /<span className="np-parity-required-text">[\s\S]*?<\/span>/g,
  "",
);

describe("legacy prototype customer language", () => {
  it("uses the approved customer vocabulary on visible prototype surfaces", () => {
    for (const approved of [
      "Method Details",
      "Run Details",
      "Supported setup",
      "Requirements",
      "Known Limitations",
    ]) {
      expect(withoutDeveloperDiagnostics).toContain(approved);
    }
  });

  it("does not expose internal qualification-governance phrases", () => {
    for (const prohibited of [
      /Method Scope/i,
      /Validation Evidence/i,
      /Validated documented scope/i,
      /documented QuickPLS scope/i,
      /\b(?:release|native)-qualified\b/i,
      /promotion evidence/i,
      /evidence (?:loaded|index|detail)/i,
      /"Candidate"/,
    ]) {
      expect(withoutDeveloperDiagnostics).not.toMatch(prohibited);
      expect(adapterSource).not.toMatch(prohibited);
    }
  });

  it("keeps scientific requirements, references, and limitations in fallback content", () => {
    const customerContent = JSON.stringify({
      trustRows,
      reportWording: fallbackNativePrototypeData.resultSummary.reportWording,
      methodCards: fallbackNativePrototypeData.methodCards,
      messages: fallbackNativePrototypeData.messages,
    });
    expect(customerContent).toMatch(/requirements/i);
    expect(customerContent).toMatch(/supported setup/i);
    expect(customerContent).toMatch(/bootstrap recommended for inference/i);
    expect(customerContent).not.toMatch(/validated|qualified|candidate|promotion|documented scope|bounded scope/i);
  });
});
