import { describe, expect, it } from "vitest";
import type { ColumnMetadata, Dataset } from "../types";
import {
  defaultNativeColumnMetadata,
  nativeVariableMetadataDraft,
  validateNativeVariableMetadata,
} from "./nativeVariableMetadata";

const metadata: ColumnMetadata = {
  name: "SAT_1",
  label: null,
  column_type: "numeric",
  scale_type: "continuous",
  missing_markers: ["", "NA"],
  theoretical_min: null,
  theoretical_max: null,
  value_labels: { "1": "Low" },
};

describe("native variable metadata", () => {
  it("creates a stable editable draft and preserves value labels on save", () => {
    const result = validateNativeVariableMetadata(metadata, {
      ...nativeVariableMetadataDraft(metadata),
      label: "Satisfaction item 1",
      scaleType: "ordinal",
      theoreticalMin: "1",
      theoreticalMax: "7",
    });
    expect(result.error).toBeNull();
    expect(result.metadata).toMatchObject({
      name: "SAT_1",
      label: "Satisfaction item 1",
      scale_type: "ordinal",
      theoretical_min: 1,
      theoretical_max: 7,
      missing_markers: ["", "NA"],
      value_labels: { "1": "Low" },
    });
  });

  it("rejects nonnumeric or reversed theoretical bounds", () => {
    expect(validateNativeVariableMetadata(metadata, { ...nativeVariableMetadataDraft(metadata), theoreticalMin: "bad" }).error).toContain("finite number");
    expect(validateNativeVariableMetadata(metadata, { ...nativeVariableMetadataDraft(metadata), theoreticalMin: "8", theoreticalMax: "7" }).error).toContain("cannot exceed");
  });

  it("infers safe defaults only when imported metadata is unavailable", () => {
    const dataset: Dataset = { id: "d", name: "Data", columns: ["group"], rows: [{ group: "A" }], missing: 0 };
    expect(defaultNativeColumnMetadata(dataset, "group")).toMatchObject({ column_type: "text", scale_type: "nominal" });
  });
});
