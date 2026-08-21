import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const appSource = readFileSync(new URL("./NativeDesktopApp.tsx", import.meta.url), "utf8");
const workspaceSource = readFileSync(new URL("./NativeRecipeV4CbsemWorkspace.tsx", import.meta.url), "utf8");

describe("CB-SEM Recipe-v4 workspace interaction contracts", () => {
  it("is reached from unified Calculate with Advanced Parameters remaining a separate modal", () => {
    expect(appSource).toContain('dialog === "calculation" ? <Suspense');
    expect(appSource).toContain("<NativeCalculationDialog");
    expect(appSource).toContain("unifiedSem={unifiedSemCalculation}");
    expect(appSource).toContain('dialog === "advanced-parameters" ? <NativeSemParameterTable');
    expect(appSource).toContain('presentation="dialog"');
    expect(appSource).toContain('advancedCalculationPlan?.route === "exact_cbsem_compatibility"');
    expect(appSource).toContain("<NativeRecipeV4CbsemWorkspace");
    expect(appSource).not.toContain('documentView === "cbsem_labs"');
    expect(workspaceSource).not.toContain("NativeCalculationDialog");
  });

  it("restores focus after run, failure, cancellation, and archive selection", () => {
    expect(workspaceSource).toContain("resultHeadingRef.current?.focus()");
    expect(workspaceSource).toContain("startButtonRef.current?.focus()");
    expect(workspaceSource).toContain("browseButtonRef.current?.focus()");
    expect(workspaceSource).toContain("monitorAbortRef.current?.abort()");
    expect(workspaceSource).toContain("services.cancel(jobId)");
  });

  it("does not surface a completed document after captured identity changed", () => {
    expect(workspaceSource).toContain('outcome.status === "completed" && !identityCancellationRequestedRef.current');
    expect(workspaceSource).toContain("setCompleted(null)");
  });

  it("uses native canonical documents and exact schema-6 services without rebuilding analytical tables", () => {
    expect(workspaceSource).toContain("completed?.canonicalDocument");
    expect(workspaceSource).toContain("reopenedEntry?.canonicalDocument");
    expect(workspaceSource).not.toContain("canonicalResultDocumentFromAnalysisRunV2");
    expect(workspaceSource).not.toContain("buildCanonicalResultDocument");
  });

  it("keeps point-only mean replacement separate and exposes exact-bootstrap controls", () => {
    expect(workspaceSource).toContain('useState<AnalysisRecipeV4MissingDataPolicy>("listwise_deletion")');
    expect(workspaceSource).toContain("missing_data: effectiveMissingDataPolicy");
    expect(workspaceSource).toContain("missingDataPolicy: effectiveMissingDataPolicy");
    expect(workspaceSource).toContain('value="mean_replacement"');
    expect(workspaceSource).toContain('id="nd-cbsem-v4-bootstrap-enabled"');
    expect(workspaceSource).toContain('id="nd-cbsem-v4-bootstrap-interval"');
    expect(workspaceSource).toContain("Exact bootstrap requires listwise deletion");
  });
});
