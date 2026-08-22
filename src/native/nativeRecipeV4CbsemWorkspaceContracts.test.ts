import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const workspaceSource = readFileSync(new URL("./NativeRecipeV4CbsemWorkspace.tsx", import.meta.url), "utf8");

describe("CB-SEM Recipe-v4 workspace interaction contracts", () => {
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

  it("navigates calculation results only after append and strict schema-6 readback succeed", () => {
    expect(workspaceSource).toContain("strictlyPublishCbsemCalculationResultV1");
    expect(workspaceSource).toContain('if (publication.status === "blocked")');
    expect(workspaceSource).toContain("setArchiveFailure(publication.diagnostic)");
    expect(workspaceSource).toContain("document: publication.entry.canonicalDocument");
    expect(workspaceSource).not.toContain("Compatibility projects that cannot accept schema-6 append");
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
