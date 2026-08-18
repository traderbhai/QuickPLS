import { describe, expect, it } from "vitest";
import type { Dataset, DatasetVersionRecord } from "../types";
import {
  ProjectDataLineageV1Error,
  parseProjectDatasetVersionRecordV1,
  parseProjectDatasetVersionRecordsV1,
} from "./projectDataLineageV1";

const SOURCE_ID = "11111111-1111-4abc-8def-111111111111";
const OUTPUT_ID = "22222222-2222-4abc-8def-222222222222";
const SOURCE_FINGERPRINT = `v2:${"a".repeat(64)}`;
const OUTPUT_FINGERPRINT = `v2:${"b".repeat(64)}`;

async function sha256(value: string) {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(value));
  return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

function datasets(): Dataset[] {
  return [{
    id: SOURCE_ID,
    name: "source",
    columns: ["x", "y"],
    rows: [],
    rowCount: 3,
    missing: 0,
    fingerprint: SOURCE_FINGERPRINT,
    kind: "raw",
  }, {
    id: OUTPUT_ID,
    name: "derived",
    columns: ["x", "y", "x_reversed"],
    rows: [],
    rowCount: 3,
    missing: 0,
    fingerprint: OUTPUT_FINGERPRINT,
    kind: "raw",
  }];
}

async function records(): Promise<DatasetVersionRecord[]> {
  const spec = {
    kind: "reverse_scale" as const,
    source_column: "x",
    target_column: "x_reversed",
    scale_min: 1,
    scale_max: 3,
  };
  const specSha256 = "c".repeat(64);
  const operationDigest = await sha256(`${SOURCE_FINGERPRINT}\u0000${specSha256}\u0000${OUTPUT_ID}`);
  return [{
    datasetId: SOURCE_ID,
    parentDatasetId: null,
    operation: "import",
    createdAt: null,
    summary: "Imported source",
    sourceColumn: null,
    targetColumn: null,
  }, {
    datasetId: OUTPUT_ID,
    parentDatasetId: SOURCE_ID,
    operation: "transform",
    createdAt: "2026-08-15T00:00:00.000Z",
    summary: "Derived x_reversed",
    sourceColumn: "x",
    targetColumn: "x_reversed",
    transformation: {
      schema_version: 2,
      engine: "qpls.dataset_transform.v2",
      operation_id: `dataset_transform:${operationDigest.slice(0, 24)}`,
      source_dataset_id: SOURCE_ID,
      source_dataset_fingerprint: SOURCE_FINGERPRINT,
      output_dataset_id: OUTPUT_ID,
      output_dataset_fingerprint: OUTPUT_FINGERPRINT,
      created_at: "2026-08-15T00:00:00.000Z",
      spec_sha256: specSha256,
      spec,
      input_columns: ["x"],
      output_columns: ["x_reversed"],
      source_row_count: 3,
      output_missing_count: 0,
    },
  }];
}

describe("project data lineage v1 untrusted reader", () => {
  it("accepts absent history without synthesis and validates an exact reconstructable graph", async () => {
    await expect(parseProjectDatasetVersionRecordsV1(undefined, datasets())).resolves.toEqual([]);
    const fixture = await records();
    await expect(parseProjectDatasetVersionRecordsV1(fixture, datasets())).resolves.toEqual(fixture);
  });

  it("rejects null, unknown fields, noncanonical IDs, and missing datasets", async () => {
    await expect(parseProjectDatasetVersionRecordsV1(null, datasets())).rejects.toBeInstanceOf(ProjectDataLineageV1Error);
    const fixture = await records();
    await expect(parseProjectDatasetVersionRecordV1({ ...fixture[0], unexpected: true })).rejects.toMatchObject({ code: "data_lineage.field_unknown" });
    await expect(parseProjectDatasetVersionRecordV1({ ...fixture[0], datasetId: SOURCE_ID.toUpperCase() })).rejects.toMatchObject({ code: "data_lineage.uuid_noncanonical" });
    await expect(parseProjectDatasetVersionRecordsV1([fixture[0]], [datasets()[1]])).rejects.toMatchObject({ code: "data_lineage.output_unknown" });
  });

  it("rejects cycles and operation-shape drift", async () => {
    const fixture = await records();
    const cycle = [
      { ...fixture[0], operation: "metadata" as const, parentDatasetId: OUTPUT_ID, sourceColumn: "x" },
      { ...fixture[1], operation: "metadata" as const, parentDatasetId: SOURCE_ID, sourceColumn: "x", targetColumn: null, transformation: undefined },
    ];
    const cycleDatasets = [datasets()[0], { ...datasets()[1], columns: ["x", "y"] }];
    await expect(parseProjectDatasetVersionRecordsV1(cycle, cycleDatasets)).rejects.toMatchObject({ code: "data_lineage.cycle" });
    await expect(parseProjectDatasetVersionRecordV1({ ...fixture[0], parentDatasetId: OUTPUT_ID })).rejects.toMatchObject({ code: "data_lineage.import_shape_invalid" });
  });

  it("validates spec structure and operation identity and binds resident fingerprints", async () => {
    const fixture = await records();
    const transform = fixture[1];
    await expect(parseProjectDatasetVersionRecordV1({
      ...transform,
      transformation: {
        ...transform.transformation!,
        spec: { ...transform.transformation!.spec, target_column: "different" },
      },
    })).rejects.toMatchObject({ code: "data_lineage.transform_columns_mismatch" });
    await expect(parseProjectDatasetVersionRecordV1({
      ...transform,
      transformation: {
        ...transform.transformation!,
        spec_sha256: "not-a-sha256",
      },
    })).rejects.toMatchObject({ code: "data_lineage.spec_sha256_invalid" });
    await expect(parseProjectDatasetVersionRecordV1({
      ...transform,
      transformation: {
        ...transform.transformation!,
        operation_id: `dataset_transform:${"0".repeat(24)}`,
      },
    })).rejects.toMatchObject({ code: "data_lineage.operation_id_mismatch" });
    await expect(parseProjectDatasetVersionRecordsV1([fixture[0], {
      ...transform,
      transformation: { ...transform.transformation!, output_dataset_fingerprint: `v2:${"c".repeat(64)}` },
    }], datasets())).rejects.toMatchObject({ code: "data_lineage.transform_dataset_mismatch" });
  });

  it("accepts zero-input add-column and exact multi-output missing-marker receipts", async () => {
    const transform = (await records())[1];
    const add = {
      ...transform,
      sourceColumn: null,
      targetColumn: "cohort",
      transformation: {
        ...transform.transformation!,
        spec: { kind: "add_column" as const, target_column: "cohort", value: null, target_type: "numeric" as const, target_scale: "continuous" as const },
        input_columns: [],
        output_columns: ["cohort"],
        output_missing_count: 3,
      },
    } satisfies DatasetVersionRecord;
    await expect(parseProjectDatasetVersionRecordV1(add)).resolves.toEqual(add);
    await expect(parseProjectDatasetVersionRecordsV1([add], [
      datasets()[0],
      { ...datasets()[1], columns: ["x", "y", "cohort"] },
    ])).resolves.toEqual([add]);

    const multi = {
      ...transform,
      targetColumn: "x_clean",
      transformation: {
        ...transform.transformation!,
        spec: {
          kind: "missing_markers" as const,
          columns: [
            { source_column: "x", target_column: "x_clean", markers: [-99], target_type: "numeric" as const, target_scale: "continuous" as const },
            { source_column: "y", target_column: "y_clean", markers: ["NA"], target_type: "text" as const, target_scale: "nominal" as const },
          ],
        },
        input_columns: ["x", "y"],
        output_columns: ["x_clean", "y_clean"],
        output_missing_count: 2,
      },
    } satisfies DatasetVersionRecord;
    await expect(parseProjectDatasetVersionRecordV1(multi)).resolves.toEqual(multi);
    await expect(parseProjectDatasetVersionRecordsV1([multi], [
      datasets()[0],
      { ...datasets()[1], columns: ["x", "y", "x_clean", "y_clean"] },
    ])).resolves.toEqual([multi]);
    await expect(parseProjectDatasetVersionRecordV1({
      ...multi,
      transformation: { ...multi.transformation!, output_columns: ["x_clean"] },
    })).rejects.toMatchObject({ code: "data_lineage.transform_columns_mismatch" });
  });

  it("keeps legacy recode explicitly referential-only", async () => {
    const recode = {
      datasetId: OUTPUT_ID,
      parentDatasetId: SOURCE_ID,
      operation: "recode",
      createdAt: "2026-08-15T00:00:00Z",
      summary: "Legacy recode",
      sourceColumn: "x",
      targetColumn: "x_reversed",
    };
    await expect(parseProjectDatasetVersionRecordV1(recode)).resolves.toEqual(recode);
    await expect(parseProjectDatasetVersionRecordV1({ ...recode, operation: "transform" })).rejects.toMatchObject({ code: "data_lineage.transform_shape_invalid" });
  });
});
