import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CanonicalResultDocumentV2 } from "../domain/canonicalResultDocumentV2";
import {
  prepareCanonicalResultExportV2,
  type CanonicalResultExportFormatV2,
  type PreparedCanonicalResultExportV2,
} from "../domain/canonicalResultCrossFormatExportV2";
import { publishNativeCanonicalResultExportV2 } from "./canonicalResultExportPublicationV2Service";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), save: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ save: mocks.save }));

function documentFixture(): CanonicalResultDocumentV2 {
  const capability = {
    registry_schema_version: 2 as const,
    capability_id: "smartpls.moderation",
    cell_id: "qpls3.pls.general_sem_multiple_two_way_moderation_point",
    capability_version: "general_sem_pls_multiple_two_way_point_v1",
  };
  return {
    schema_version: 2,
    document_id: "result.general-sem:native-export-v2",
    title: "General SEM export",
    provenance: {
      run_id: "run:native-export-v2",
      project_id: "project:native-export-v2",
      model_id: "model:native-export-v2",
      model_digest: "a".repeat(64),
      dataset_id: "dataset:native-export-v2",
      dataset_fingerprint: "b".repeat(64),
      recipe_id: "recipe:native-export-v2",
      recipe_digest: "c".repeat(64),
      capability_cell: capability,
      method_version: "qpls.general-sem-pls.multiple-two-way.point.v1",
      engine_version: "qpls-estimation-test",
      seed: 42,
      workers: 1,
      started_at: "2026-08-19T10:00:00Z",
      completed_at: "2026-08-19T10:00:01Z",
    },
    capability_cells: [capability],
    sections: [{ id: "results", title: "Results", table_ids: ["effects"], chart_ids: ["plot"], capability_cells: [capability] }],
    tables: [{
      id: "effects",
      title: "Effects",
      columns: [{ id: "estimate", label: "Estimate", data_type: "number", description: "Estimate." }],
      rows: [{ id: "gamma", cells: [{ kind: "number", value: 0.2 }] }],
      footnote_ids: [],
      capability_cells: [capability],
    }],
    charts: [{
      id: "plot",
      title: "Interaction plot",
      description: "Persisted interaction plot.",
      kind: "line",
      series: [{ id: "low", label: "Low level", points: [{ x: -1, y: 0.1 }, { x: 1, y: 0.3 }] }],
      source_table_id: "effects",
      display: { x_axis_label: "X", y_axis_label: "Y" },
    }],
    notices: [],
    exclusions: [],
    footnotes: [],
    presentation: { default_section_id: "results", default_table_id: "effects", precision: 4, missing_value_label: "NA", chart_defaults: {} },
  };
}

function prepared(format: CanonicalResultExportFormatV2): PreparedCanonicalResultExportV2 {
  const result = prepareCanonicalResultExportV2(documentFixture(), {
    format,
    tableIds: format === "svg" || format === "png" ? [] : ["effects"],
    chartIds: format === "svg" || format === "png" || format === "html" || format === "pdf" ? ["plot"] : [],
  });
  if (!result.ok) throw new Error(result.errors.join("\n"));
  return result.artifact;
}

interface MockNativeRequest {
  schemaVersion: number;
  format: CanonicalResultExportFormatV2;
  destinationPath: string;
  identity: Record<string, unknown>;
  payload: {
    kind: "exact_bytes" | "xlsx_tables_json";
    byteLength: number;
    sha256: string;
    contentsBase64?: string;
    tablesJson?: string;
  };
}

function successfulReceipt(request: MockNativeRequest) {
  const exact = request.payload.kind === "exact_bytes";
  return {
    schemaVersion: 2,
    format: request.format,
    path: request.destinationPath,
    bytes: exact ? request.payload.byteLength : 4096,
    sha256: exact ? request.payload.sha256 : "e".repeat(64),
    payloadSha256: request.payload.sha256,
    identity: request.identity,
  };
}

describe("native canonical cross-format publication service", () => {
  beforeEach(() => {
    mocks.save.mockReset();
    mocks.invoke.mockReset();
    mocks.save.mockImplementation(async ({ defaultPath }: { defaultPath: string }) => `D:\\exports\\${defaultPath}`);
    mocks.invoke.mockImplementation(async (_command: string, args: { request: MockNativeRequest }) => successfulReceipt(args.request));
  });

  it("publishes all six validated formats through one command with exact provenance and payload bindings", async () => {
    for (const format of ["csv", "xlsx", "html", "pdf", "svg", "png"] as const) {
      const artifact = prepared(format);
      const receipt = await publishNativeCanonicalResultExportV2(artifact);
      expect(receipt).toMatchObject({
        schemaVersion: 2,
        format,
        path: `D:\\exports\\${artifact.defaultFileName}`,
        identity: {
          documentId: documentFixture().document_id,
          runId: documentFixture().provenance.run_id,
          methodVersion: documentFixture().provenance.method_version,
          datasetFingerprint: documentFixture().provenance.dataset_fingerprint,
          stableTableIds: artifact.semantic.selection.table_ids,
          stableChartIds: artifact.semantic.selection.chart_ids,
          semanticSha256: artifact.semantic.semantic_sha256,
        },
      });
    }

    expect(mocks.invoke).toHaveBeenCalledTimes(6);
    for (const [command, args] of mocks.invoke.mock.calls as Array<[string, { request: MockNativeRequest }]>) {
      expect(command).toBe("publish_canonical_result_export_v2");
      expect(args.request.schemaVersion).toBe(2);
      expect(args.request.payload.sha256).toMatch(/^[a-f0-9]{64}$/u);
      expect(args.request.payload.byteLength).toBeGreaterThan(0);
    }
    const xlsxCall = (mocks.invoke.mock.calls as Array<[string, { request: MockNativeRequest }]>).find(([, args]) => args.request.format === "xlsx");
    expect(xlsxCall?.[1].request.payload).toMatchObject({ kind: "xlsx_tables_json" });
    expect(xlsxCall?.[1].request.payload.tablesJson).toContain('"id":"effects"');
    expect(xlsxCall?.[1].request.payload.tablesJson).toContain('"rows":[["gamma","0.2"]]');
  });

  it("rejects unknown format and path tamper before invoking the native writer", async () => {
    const unknown = { ...prepared("csv"), format: "docx", extension: "docx", defaultFileName: "result.docx" } as unknown as PreparedCanonicalResultExportV2;
    await expect(publishNativeCanonicalResultExportV2(unknown)).rejects.toThrow("Unknown canonical export format");
    expect(mocks.save).not.toHaveBeenCalled();
    expect(mocks.invoke).not.toHaveBeenCalled();

    mocks.save.mockResolvedValue("D:\\exports\\result.txt");
    await expect(publishNativeCanonicalResultExportV2(prepared("csv"))).rejects.toThrow("must end in .csv");
    expect(mocks.invoke).not.toHaveBeenCalled();
  });

  it("fails closed on native byte or identity receipt tamper", async () => {
    mocks.invoke.mockImplementationOnce(async (_command: string, args: { request: MockNativeRequest }) => ({
      ...successfulReceipt(args.request),
      sha256: "f".repeat(64),
    }));
    await expect(publishNativeCanonicalResultExportV2(prepared("pdf"))).rejects.toThrow("bytes differ");

    mocks.invoke.mockImplementationOnce(async (_command: string, args: { request: MockNativeRequest }) => ({
      ...successfulReceipt(args.request),
      identity: { ...args.request.identity, runId: "run:tampered" },
    }));
    await expect(publishNativeCanonicalResultExportV2(prepared("xlsx"))).rejects.toThrow("receipt does not match");
  });

  it("propagates writer failure without claiming a publication", async () => {
    mocks.invoke.mockRejectedValueOnce(new Error("[CANONICAL_EXPORT_WRITE_FAILED] injected writer failure"));
    await expect(publishNativeCanonicalResultExportV2(prepared("html"))).rejects.toThrow("CANONICAL_EXPORT_WRITE_FAILED");
    expect(mocks.invoke).toHaveBeenCalledOnce();
  });

  it("cancels before native write for a pre-aborted signal, dialog cancellation, or abort during the dialog", async () => {
    const preAborted = new AbortController();
    preAborted.abort();
    await expect(publishNativeCanonicalResultExportV2(prepared("csv"), preAborted.signal)).resolves.toBeNull();
    expect(mocks.save).not.toHaveBeenCalled();
    expect(mocks.invoke).not.toHaveBeenCalled();

    mocks.save.mockResolvedValueOnce(null);
    await expect(publishNativeCanonicalResultExportV2(prepared("svg"))).resolves.toBeNull();
    expect(mocks.invoke).not.toHaveBeenCalled();

    const duringDialog = new AbortController();
    mocks.save.mockImplementationOnce(async () => {
      duringDialog.abort();
      return "D:\\exports\\cancelled.png";
    });
    await expect(publishNativeCanonicalResultExportV2(prepared("png"), duringDialog.signal)).resolves.toBeNull();
    expect(mocks.invoke).not.toHaveBeenCalled();
  });
});
