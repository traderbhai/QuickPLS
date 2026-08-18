import { describe, expect, it } from "vitest";
import { tablesToCsv, tablesToHtml } from "../domain/resultTables";
import { completedCtaPlsRun } from "./nativeCtaPls.testFixture";
import { nativeRunProvenanceTable } from "./nativeExportTables";
import { nativeResultTables } from "./nativeResults";

describe("native CTA-PLS export contract", () => {
  it("exports same-run CTA tables with explicit method, covariance, case accounting, and fingerprint provenance", () => {
    const run = completedCtaPlsRun();
    const tables = [...nativeResultTables(run), nativeRunProvenanceTable(run)];
    const csv = tablesToCsv(tables);
    const html = tablesToHtml(tables);
    for (const value of [
      "CTA-PLS tetrad summary",
      "CTA-PLS tetrads",
      "CTA-PLS run details",
      "cta_pls_tetrad_v1",
      "sample_covariance_of_preprocessed_indicators_v1",
      "sha256:cta",
    ]) {
      expect(csv).toContain(value);
      expect(html).toContain(value);
    }
    expect(csv).toContain("Complete cases,80");
    expect(csv).toContain("Omitted cases,2");
    expect(csv).not.toContain("CTA-PLS requirements and exclusions");
    expect(html).not.toContain("CTA-PLS requirements and exclusions");
    expect(tables.at(-1)).toMatchObject({ id: "run_provenance", status: "validated" });
  });

  it("does not export CTA tables after an identity-bound payload is altered", () => {
    const run = completedCtaPlsRun();
    run.result!.cta_pls!.estimates[0].indicator_a = "other";
    expect(nativeResultTables(run)).toEqual([]);
  });
});
