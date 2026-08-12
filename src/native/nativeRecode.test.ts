import { describe, expect, it } from "vitest";
import type { ColumnMetadata, Dataset } from "../types";
import {
  buildNativeRecodeSpec,
  defaultNativeRecodeDraft,
  deriveNativeRecodeTargetColumn,
  type NativeRecodeDraft,
  type NativeRecodeTargetScale,
  type NativeRecodeTargetType,
  validateNativeRecodeDraft,
} from "./nativeRecode";

const metadata = (
  name: string,
  columnType: ColumnMetadata["column_type"] = "numeric",
  scaleType: ColumnMetadata["scale_type"] = "continuous",
  label: string | null = null,
): ColumnMetadata => ({
  name,
  label,
  column_type: columnType,
  scale_type: scaleType,
  missing_markers: ["", "NA"],
  theoretical_min: null,
  theoretical_max: null,
  value_labels: {},
});

function dataset(overrides: Partial<Dataset> = {}): Dataset {
  return {
    id: "dataset-v1",
    name: "Survey",
    columns: ["score", "group", "flag"],
    rows: [],
    missing: 0,
    kind: "raw",
    columnMetadata: [
      metadata("score", "numeric", "ordinal", "Satisfaction score"),
      metadata("group", "text", "nominal"),
      metadata("flag", "boolean", "binary"),
    ],
    ...overrides,
  };
}

function validDraft(overrides: Partial<NativeRecodeDraft> = {}): NativeRecodeDraft {
  return {
    sourceColumn: "score",
    targetColumn: "score_recoded",
    targetLabel: "Recoded score",
    targetType: "numeric",
    targetScale: "ordinal",
    mappings: [
      { source: "1", target: "0" },
      { source: "2", target: "  " },
    ],
    unmapped: "keep_original",
    ...overrides,
  };
}

function issueCodes(input: ReturnType<typeof validateNativeRecodeDraft>) {
  return input.issues.map((entry) => entry.code);
}

describe("native recode defaults", () => {
  it("derives a unique target name and preserves compatible source metadata without reading rows", () => {
    const rowlessDataset = {
      ...dataset({ columns: ["score", "score_recoded", "score_recoded_2"] }),
      get rows(): Dataset["rows"] {
        throw new Error("recode defaults must not read dataset rows");
      },
    };

    expect(deriveNativeRecodeTargetColumn(rowlessDataset, "score")).toBe("score_recoded_3");
    expect(defaultNativeRecodeDraft(rowlessDataset, "score")).toEqual({
      sourceColumn: "score",
      targetColumn: "score_recoded_3",
      targetLabel: "Satisfaction score (recoded)",
      targetType: "numeric",
      targetScale: "ordinal",
      mappings: [{ source: "", target: "" }],
      unmapped: "keep_original",
    });
  });

  it("uses row-independent safe defaults when imported metadata is absent", () => {
    const draft = defaultNativeRecodeDraft(dataset({ columnMetadata: undefined }), "group");
    expect(draft).toMatchObject({ targetType: "text", targetScale: "nominal", targetColumn: "group_recoded" });
  });

  it("repairs incompatible imported metadata in the initial draft", () => {
    expect(
      defaultNativeRecodeDraft(dataset({ columnMetadata: [metadata("group", "text", "continuous")] }), "group"),
    ).toMatchObject({ targetType: "text", targetScale: "nominal" });
    expect(
      defaultNativeRecodeDraft(dataset({ columnMetadata: [metadata("flag", "boolean", "nominal")] }), "flag"),
    ).toMatchObject({ targetType: "boolean", targetScale: "binary" });
  });
});

describe("native recode spec building", () => {
  it("builds the exact camel-case command contract and normalizes blank targets to missing", () => {
    const spec = buildNativeRecodeSpec(dataset(), validDraft({ targetLabel: "  Recoded score  " }));
    expect(spec).toEqual({
      sourceColumn: "score",
      targetColumn: "score_recoded",
      targetLabel: "Recoded score",
      targetType: "numeric",
      targetScale: "ordinal",
      mappings: [
        { source: "1", target: "0" },
        { source: "2", target: null },
      ],
      unmapped: "keep_original",
    });
    expect(Object.keys(spec ?? {})).toEqual([
      "sourceColumn",
      "targetColumn",
      "targetLabel",
      "targetType",
      "targetScale",
      "mappings",
      "unmapped",
    ]);
  });

  it("normalizes a blank target label to null and accepts every backend unmapped policy", () => {
    for (const unmapped of ["keep_original", "set_missing", "error"] as const) {
      expect(buildNativeRecodeSpec(dataset(), validDraft({ targetLabel: " ", unmapped }))).toMatchObject({
        targetLabel: null,
        unmapped,
      });
    }
  });

  it("does not mutate or inspect dataset rows or mutate a frozen draft", () => {
    const columns = Object.freeze(["score", "group", "flag"]);
    const rowlessDataset = Object.freeze({
      ...dataset({ columns: columns as string[] }),
      get rows(): Dataset["rows"] {
        throw new Error("validation must not read dataset rows");
      },
    });
    const draft = Object.freeze({
      ...validDraft(),
      mappings: Object.freeze(validDraft().mappings.map((mapping) => Object.freeze(mapping))),
    }) as Readonly<NativeRecodeDraft>;

    expect(validateNativeRecodeDraft(rowlessDataset, draft).spec).not.toBeNull();
    expect(columns).toEqual(["score", "group", "flag"]);
    expect(draft.mappings[1]).toEqual({ source: "2", target: "  " });
  });
});

describe("native recode source and target validation", () => {
  it("requires a raw dataset and an existing source indicator", () => {
    const result = validateNativeRecodeDraft(
      dataset({ kind: "correlation" }),
      validDraft({ sourceColumn: "not_present" }),
    );
    expect(issueCodes(result)).toEqual(expect.arrayContaining(["dataset_not_raw", "source_not_found"]));
    expect(result.spec).toBeNull();
  });

  it.each([
    ["", "target_required"],
    ["   ", "target_required"],
    [" score_new", "target_not_trimmed"],
    ["score_new ", "target_not_trimmed"],
    ["score", "target_exists"],
  ] as const)("rejects target column %j with %s", (targetColumn, expectedCode) => {
    const result = validateNativeRecodeDraft(dataset(), validDraft({ targetColumn }));
    expect(issueCodes(result)).toContain(expectedCode);
    expect(result.spec).toBeNull();
  });

  it("uses exact backend column-name matching for uniqueness", () => {
    expect(validateNativeRecodeDraft(dataset(), validDraft({ targetColumn: "Score" })).spec?.targetColumn).toBe("Score");
  });
});

describe("native recode mapping validation", () => {
  it("requires at least one mapping and a nonblank source in every mapping", () => {
    expect(issueCodes(validateNativeRecodeDraft(dataset(), validDraft({ mappings: [] })))).toContain("mappings_required");
    const blank = validateNativeRecodeDraft(dataset(), validDraft({ mappings: [{ source: "  ", target: "1" }] }));
    expect(blank.issues).toContainEqual(expect.objectContaining({ code: "mapping_source_required", path: "mappings.0.source" }));
  });

  it("rejects duplicate sources using the backend source type semantics", () => {
    const numeric = validateNativeRecodeDraft(
      dataset(),
      validDraft({ mappings: [{ source: "1", target: "1" }, { source: "1.0", target: "2" }] }),
    );
    expect(numeric.issues).toContainEqual(expect.objectContaining({ code: "mapping_source_duplicate", path: "mappings.1.source" }));

    const boolean = validateNativeRecodeDraft(
      dataset(),
      validDraft({
        sourceColumn: "flag",
        targetColumn: "flag_recoded",
        targetType: "boolean",
        targetScale: "binary",
        mappings: [{ source: "TRUE", target: "false" }, { source: "1", target: "0" }],
      }),
    );
    expect(issueCodes(boolean)).toContain("mapping_source_duplicate");
  });

  it("preserves distinct text source values while rejecting an exact duplicate", () => {
    const base = validDraft({
      sourceColumn: "group",
      targetColumn: "group_recoded",
      targetType: "text",
      targetScale: "nominal",
    });
    expect(
      validateNativeRecodeDraft(dataset(), {
        ...base,
        mappings: [{ source: "A", target: "Alpha" }, { source: " A", target: "Leading-space A" }],
      }).spec,
    ).not.toBeNull();
    expect(
      issueCodes(validateNativeRecodeDraft(dataset(), { ...base, mappings: [{ source: "A", target: "x" }, { source: "A", target: "y" }] })),
    ).toContain("mapping_source_duplicate");
  });

  it.each(["hex:0x10", "comma:1,000", "infinite:1e999"])("rejects a non-Rust-compatible numeric mapping source (%s)", (value) => {
    expect(
      issueCodes(validateNativeRecodeDraft(dataset(), validDraft({ mappings: [{ source: value.split(":")[1], target: "1" }] }))),
    ).toContain("mapping_source_invalid");
  });

  it("accepts finite decimal and scientific numeric syntax supported by the backend", () => {
    const mappings = [".5", "1.", "-2.5e+3"].map((source, index) => ({ source, target: String(index) }));
    expect(validateNativeRecodeDraft(dataset(), validDraft({ mappings })).spec?.mappings).toEqual(mappings);
  });

  it("validates nonblank mapping targets against the selected target type", () => {
    expect(
      issueCodes(validateNativeRecodeDraft(dataset(), validDraft({ mappings: [{ source: "1", target: "not-a-number" }] }))),
    ).toContain("mapping_target_invalid");
    expect(
      issueCodes(
        validateNativeRecodeDraft(
          dataset(),
          validDraft({ targetType: "boolean", targetScale: "binary", mappings: [{ source: "1", target: "yes" }] }),
        ),
      ),
    ).toContain("mapping_target_invalid");
    expect(
      buildNativeRecodeSpec(
        dataset(),
        validDraft({ targetType: "boolean", targetScale: "binary", mappings: [{ source: "1", target: " FALSE " }] }),
      )?.mappings,
    ).toEqual([{ source: "1", target: " FALSE " }]);
  });
});

describe("native recode type and scale compatibility", () => {
  const validScales: Record<NativeRecodeTargetType, NativeRecodeTargetScale[]> = {
    numeric: ["continuous", "ordinal", "nominal", "binary", "identifier"],
    text: ["ordinal", "nominal", "binary", "identifier"],
    boolean: ["binary"],
  };

  for (const [targetType, scales] of Object.entries(validScales) as Array<[
    NativeRecodeTargetType,
    NativeRecodeTargetScale[],
  ]>) {
    it.each(scales)(`accepts ${targetType} with %s scale`, (targetScale) => {
      const mappingTarget = targetType === "boolean" ? "true" : targetType === "numeric" ? "1" : "One";
      expect(
        validateNativeRecodeDraft(
          dataset(),
          validDraft({ targetType, targetScale, mappings: [{ source: "1", target: mappingTarget }] }),
        ).spec,
      ).not.toBeNull();
    });
  }

  it("rejects text-continuous and every nonbinary boolean scale", () => {
    expect(issueCodes(validateNativeRecodeDraft(dataset(), validDraft({ targetType: "text", targetScale: "continuous" })))).toContain(
      "target_scale_incompatible",
    );
    for (const targetScale of ["continuous", "ordinal", "nominal", "identifier"] as const) {
      expect(
        issueCodes(validateNativeRecodeDraft(dataset(), validDraft({ targetType: "boolean", targetScale }))),
      ).toContain("target_scale_incompatible");
    }
  });

  it("defends the native boundary from unsupported runtime enum values", () => {
    const invalid = validDraft() as NativeRecodeDraft & Record<string, unknown>;
    invalid.targetType = "date" as NativeRecodeTargetType;
    invalid.targetScale = "ratio" as NativeRecodeTargetScale;
    invalid.unmapped = "drop_row" as NativeRecodeDraft["unmapped"];
    const result = validateNativeRecodeDraft(dataset(), invalid);
    expect(issueCodes(result)).toEqual(
      expect.arrayContaining(["target_type_invalid", "target_scale_invalid", "unmapped_policy_invalid"]),
    );
    expect(result.spec).toBeNull();
  });
});
