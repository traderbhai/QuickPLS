import { describe, expect, it } from "vitest";
import type { AnalysisUiSettings, Dataset } from "../types";
import {
  nativeEligibleGroupColumns,
  nativeGroupSelectionAssessment,
  nativeGroupOptionLabel,
  nativeMgaProfileAssessment,
  residentDatasetGroupProfile,
} from "./nativeMga";

const settings = (patch: Partial<AnalysisUiSettings> = {}): AnalysisUiSettings => ({
  method: "mga",
  weightingScheme: "path",
  preprocessing: "standardized",
  bootstrapSamples: 0,
  studentizedInnerSamples: 0,
  permutationSamples: 0,
  seed: 20_260_718,
  workers: 2,
  confidenceLevel: 0.95,
  groupColumn: "segment",
  groupAValue: "A",
  groupBValue: "B",
  groupMethods: "micom,mga_permutation",
  groupPermutationSamples: 5_000,
  micomConfiguralConfirmed: true,
  ...patch,
});

const dataset: Dataset = {
  id: "grouped",
  name: "grouped.csv",
  columns: ["x1", "x2", "y1", "y2", "segment"],
  rowCount: 25,
  rows: [
    ...Array.from({ length: 10 }, (_, index) => ({ x1: index + 1, x2: index + 2, y1: index + 3, y2: index + 4, segment: "A" })),
    ...Array.from({ length: 11 }, (_, index) => ({ x1: index + 2, x2: index + 3, y1: index + 4, y2: index + 5, segment: "B" })),
    { x1: 1, x2: 2, y1: 3, y2: 4, segment: "C" },
    { x1: 1, x2: null, y1: 3, y2: 4, segment: "C" },
    { x1: 1, x2: 2, y1: 3, y2: 4, segment: null },
    { x1: 1, x2: 2, y1: 3, y2: 4, segment: "A" },
  ],
  missing: 2,
  columnMetadata: [{
    name: "segment",
    label: "Customer segment",
    column_type: "text",
    scale_type: "nominal",
    missing_markers: [""],
    theoretical_min: null,
    theoretical_max: null,
    value_labels: { A: "Treatment", B: "Control" },
  }],
};

describe("native combined MICOM and two-group permutation MGA setup contract", () => {
  it("never offers an assigned model indicator as a group variable", () => {
    expect(nativeEligibleGroupColumns(dataset, ["x1", "x2", "y1", "y2"]))
      .toEqual(["segment"]);
  });

  it("derives complete-case counts only when every row is resident", () => {
    const profile = residentDatasetGroupProfile(dataset, "segment", ["x1", "x2", "y1", "y2"]);
    expect(profile).toMatchObject({ rowCount: 25, missingCount: 1, unsupportedCount: 0, truncated: false });
    expect(profile?.groups).toEqual([
      { value: "A", label: "Treatment", observations: 11, completeCases: 11 },
      { value: "B", label: "Control", observations: 11, completeCases: 11 },
      { value: "C", label: null, observations: 2, completeCases: 1 },
    ]);
    expect(residentDatasetGroupProfile({ ...dataset, rowCount: 26 }, "segment", ["x1"]))
      .toBeNull();
  });

  it("requires explicit distinct groups with ten complete model cases each", () => {
    const profile = residentDatasetGroupProfile(dataset, "segment", ["x1", "x2", "y1", "y2"]);
    expect(nativeMgaProfileAssessment(profile, settings())).toMatchObject({ canRun: true });

    const duplicate = nativeMgaProfileAssessment(profile, settings({ groupBValue: "A" }));
    expect(duplicate.canRun).toBe(false);
    expect(duplicate.blockers.join(" ")).toContain("different values");

    const undersized = nativeMgaProfileAssessment(profile, settings({ groupBValue: "C" }));
    expect(undersized.canRun).toBe(false);
    expect(undersized.blockers.join(" ")).toContain("1 complete model cases");

    const stale = nativeMgaProfileAssessment({ ...profile!, columnName: "other" }, settings());
    expect(stale.canRun).toBe(false);
    expect(stale.blockers.join(" ")).toContain("stale");
  });

  it("discloses missing and unselected rows and enforces a bounded permutation plan", () => {
    const profile = residentDatasetGroupProfile(dataset, "segment", ["x1", "x2", "y1", "y2"]);
    const assessment = nativeMgaProfileAssessment(profile, settings());
    expect(assessment.warnings).toEqual([
      "1 row has a missing group value and will be excluded.",
      "2 rows belong to unselected group values and will be excluded.",
    ]);
    expect(nativeMgaProfileAssessment(profile, settings({ groupPermutationSamples: 4_999 })).blockers.join(" "))
      .toContain("5,000 to 10,000 permutations");
  });

  it("keeps Step 1 researcher-confirmed and rejects incomplete group-method selections", () => {
    const profile = residentDatasetGroupProfile(dataset, "segment", ["x1", "x2", "y1", "y2"]);
    const unconfirmed = nativeMgaProfileAssessment(profile, settings({ micomConfiguralConfirmed: false }));
    expect(unconfirmed.canRun).toBe(false);
    expect(unconfirmed.blockers.join(" ")).toContain("explicit researcher review");

    const incomplete = nativeMgaProfileAssessment(profile, settings({ groupMethods: "micom" }));
    expect(incomplete.canRun).toBe(false);
    expect(incomplete.blockers.join(" ")).toContain("requires both MICOM and structural-path permutation MGA");

    const selectionOnly = nativeGroupSelectionAssessment(profile, settings({
      groupMethods: "micom",
      micomConfiguralConfirmed: false,
      groupPermutationSamples: 1,
    }));
    expect(selectionOnly).toMatchObject({ canRun: true, blockers: [] });
  });

  it("returns the typed extreme-imbalance blocker before calculation", () => {
    const profile = residentDatasetGroupProfile(dataset, "segment", ["x1", "x2", "y1", "y2"]);
    const imbalanced = {
      ...profile!,
      rowCount: 111,
      missingCount: 0,
      groups: [
        { value: "A", label: null, observations: 101, completeCases: 101 },
        { value: "B", label: null, observations: 10, completeCases: 10 },
      ],
    };
    const assessment = nativeMgaProfileAssessment(imbalanced, settings());
    expect(assessment.canRun).toBe(false);
    expect(assessment.blockers.join(" ")).toContain("micom.extreme_group_imbalance");
  });

  it("keeps raw group identities visible when value labels are present", () => {
    expect(nativeGroupOptionLabel({ value: "A", label: "Treatment", observations: 11, completeCases: 10 }))
      .toBe("Treatment [A] · N=10 · 1 excluded");
  });
});
