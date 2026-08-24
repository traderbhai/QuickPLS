import { beforeEach, describe, expect, it, vi } from "vitest";
import type { MultiModRecipeConfigV1 } from "../domain/multimodContractsV1";
import {
  parseNativeMultiModCompletedResultV1,
  parseNativeMultiModGroupingProfileV1,
  parseNativeMultiModJobSnapshotV1,
  parseNativeMultiModPreflightV1,
  profileNativeMultiModGroupingV1,
  resumeNativeMultiModRequestV1,
  stageNativeMultiModRequestV1,
  type NativeMultiModArchiveAuthorityV1,
} from "./nativeMultiModJobV1";

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));

const UUIDS = {
  project: "00000000-0000-0000-0000-000000000101",
  dataset: "00000000-0000-0000-0000-000000000102",
  recipe: "00000000-0000-0000-0000-000000000103",
  staged: "00000000-0000-0000-0000-000000000104",
  job: "00000000-0000-0000-0000-000000000105",
  cache: "00000000-0000-0000-0000-000000000106",
} as const;

const authority: NativeMultiModArchiveAuthorityV1 = {
  archivePath: "D:\\projects\\multimod.qpls",
  archiveSha256: "a".repeat(64),
  projectId: UUIDS.project,
  datasetId: UUIDS.dataset,
  datasetFingerprint: "b".repeat(64),
  modelId: "model-1",
  modelScientificSha256: "c".repeat(64),
  sourceRecipeId: UUIDS.recipe,
  sourceRecipeDocumentSha256: "d".repeat(64),
};

const heterogeneityRequest = {
  kind: "pls_unobserved_heterogeneity_v2",
  config: {
    schema_version: 2,
    profile: "p0_structural",
    phase: {
      kind: "discovery",
      candidate_k: [2],
      algorithms: ["fimix_pls_v2"],
    },
    seed: 42,
    fimix: {
      starts: 30,
      max_iterations: 5_000,
      relative_log_likelihood_tolerance: 1e-10,
      consecutive_converged_iterations: 3,
      likelihood_decrease_tolerance: 1e-9,
      residual_variance_floor: 1e-8,
      rank_tolerance: 1e-11,
      minimum_class_share: 0.05,
      required_reproducing_starts: 2,
      optimum_relative_log_likelihood_tolerance: 1e-8,
      optimum_maximum_coefficient_difference: 1e-6,
      optimum_mean_posterior_difference: 1e-4,
    },
    pls_pos: {
      starts: 10,
      strict_improvement_tolerance: 1e-10,
      stable_objective_tolerance: 1e-10,
      minimum_reproducing_starts: 2,
    },
  },
} as const satisfies MultiModRecipeConfigV1;

function cacheReceipt(
  stage: "mga_execution" | "archive_ready" = "archive_ready",
  target:
    | "mga_multigroup_v1"
    | "pls_heterogeneity_v2" = "pls_heterogeneity_v2",
) {
  return {
    schemaVersion: 1,
    cacheId: UUIDS.cache,
    cacheDirectory:
      "C:\\Users\\tester\\AppData\\Local\\QuickPLS\\multimod-cache-v1\\cache",
    manifestSha256: "e".repeat(64),
    embeddedAuthoritySha256: "f".repeat(64),
    sourceArchiveSha256: authority.archiveSha256,
    resultId: "qpls-multimod-result",
    recipeId: UUIDS.staged,
    target,
    stage,
    createdAt: "2026-08-24T10:00:00.000Z",
  } as const;
}

describe("strict native MultiMod job adapter", () => {
  beforeEach(() => mocks.invoke.mockReset());

  it("maps the public heterogeneity config to the exact native compiler target", () => {
    const staged = stageNativeMultiModRequestV1(
      authority,
      heterogeneityRequest,
      {
        recipeId: UUIDS.staged,
        createdAt: "2026-08-24T10:00:00.000Z",
      },
    );
    expect(staged.config.kind).toBe("pls_heterogeneity_v2");
    expect(staged.expectedArchiveSha256).toBe(authority.archiveSha256);
    expect(staged.stagedRecipeId).toBe(UUIDS.staged);
  });

  it("allows publication resume for every family but rejects MGA execution cache reuse by another family", () => {
    const staged = stageNativeMultiModRequestV1(
      authority,
      heterogeneityRequest,
      {
        recipeId: UUIDS.staged,
        createdAt: "2026-08-24T10:00:00.000Z",
      },
    );
    expect(
      resumeNativeMultiModRequestV1(staged, cacheReceipt()).resumeCache?.stage,
    ).toBe("archive_ready");
    expect(() =>
      resumeNativeMultiModRequestV1(
        staged,
        cacheReceipt("mga_execution", "pls_heterogeneity_v2"),
      ),
    ).toThrow(/supported resume stage/u);
  });

  it("fails closed on unknown preflight fields and parses only exact built-in readiness", () => {
    const wire = {
      schemaVersion: 1,
      target: "pls_heterogeneity_v2",
      capabilityCellId: "qpls.multimod.heterogeneity.v2",
      readiness: "built_in_from_dataset",
      stableReasonCodes: [],
      stagedRecipeId: UUIDS.staged,
      stagedRecipeDocumentSha256: "1".repeat(64),
      compilationIdentitySha256: "2".repeat(64),
      mgaGroupEligibility: null,
    };
    expect(parseNativeMultiModPreflightV1(wire).readiness).toBe(
      "built_in_from_dataset",
    );
    expect(() =>
      parseNativeMultiModPreflightV1({ ...wire, fabricated: true }),
    ).toThrow(/not part of the versioned contract/u);
  });

  it("retains an archive-ready receipt on cancellation without exposing a result payload", () => {
    const parsed = parseNativeMultiModJobSnapshotV1({
      schemaVersion: 1,
      jobId: UUIDS.job,
      target: "pls_heterogeneity_v2",
      state: "cancelled",
      phase: "multimod_cancelled",
      shardId: "multimod:cancelled",
      completedUnits: 0,
      totalUnits: 1,
      message: "Cancelled without publication",
      warningCodes: [],
      failure: null,
      resumeCache: cacheReceipt(),
      queuedAt: "2026-08-24T10:00:00.000Z",
      startedAt: "2026-08-24T10:00:01.000Z",
      completedAt: "2026-08-24T10:00:02.000Z",
    });
    expect(parsed.state).toBe("cancelled");
    expect(parsed.failure).toBeNull();
    expect(parsed.resumeCache?.stage).toBe("archive_ready");
  });

  it("accepts only an MGA-bound intra-estimation cache stage", () => {
    const base = {
      schemaVersion: 1,
      jobId: UUIDS.job,
      target: "mga_multigroup_v1",
      state: "cancelled",
      phase: "multimod_cancelled",
      shardId: "multimod:cancelled",
      completedUnits: 3,
      totalUnits: 9,
      message: "Validated completed shards are resumable",
      warningCodes: [],
      failure: null,
      resumeCache: cacheReceipt("mga_execution", "mga_multigroup_v1"),
      queuedAt: "2026-08-24T10:00:00.000Z",
      startedAt: "2026-08-24T10:00:01.000Z",
      completedAt: "2026-08-24T10:00:02.000Z",
    };
    expect(parseNativeMultiModJobSnapshotV1(base).resumeCache?.stage).toBe(
      "mga_execution",
    );
    expect(() =>
      parseNativeMultiModJobSnapshotV1({
        ...base,
        target: "pls_heterogeneity_v2",
        resumeCache: cacheReceipt("mga_execution", "pls_heterogeneity_v2"),
      }),
    ).toThrow(/differs from its target or lifecycle state/u);
    expect(() =>
      parseNativeMultiModJobSnapshotV1({
        ...base,
        resumeCache: {
          ...cacheReceipt("mga_execution", "mga_multigroup_v1"),
          stage: "unknown",
        },
      }),
    ).toThrow(/unsupported value/u);
  });

  it("preserves integer and numeric grouping identities and binds the profile to archive authority", async () => {
    const wire = {
      schemaVersion: 1,
      archiveSha256: authority.archiveSha256,
      datasetFingerprint: authority.datasetFingerprint,
      columns: [
        {
          column: "cohort",
          label: "Cohort",
          usedAsIndicator: false,
          groups: [
            {
              groupId: "integer-one",
              label: "Integer 1",
              value: { kind: "integer", value: 1 },
              selectedRows: 20,
              completeCases: 18,
            },
            {
              groupId: "number-one",
              label: "Number 1",
              value: { kind: "number", value: 1 },
              selectedRows: 22,
              completeCases: 21,
            },
          ],
        },
      ],
      omittedHighCardinalityColumns: [],
      sourceRecheckedUnchanged: true,
    };
    expect(
      parseNativeMultiModGroupingProfileV1(
        wire,
        authority,
      ).columns[0].groups.map((group) => group.value.kind),
    ).toEqual(["integer", "number"]);
    expect(() =>
      parseNativeMultiModGroupingProfileV1(
        { ...wire, archiveSha256: "f".repeat(64) },
        authority,
      ),
    ).toThrow(/differs from the requested archive/u);

    mocks.invoke.mockResolvedValue(wire);
    await expect(
      profileNativeMultiModGroupingV1(authority),
    ).resolves.toMatchObject({ schemaVersion: 1 });
    expect(mocks.invoke).toHaveBeenCalledWith(
      "profile_internal_labs_multimod_grouping_v1",
      {
        request: expect.objectContaining({
          archivePath: authority.archivePath,
          expectedArchiveSha256: authority.archiveSha256,
          sourceRecipeDocumentSha256: authority.sourceRecipeDocumentSha256,
        }),
      },
    );
  });

  it("rejects completed-result receipt disagreement before scientific rendering", () => {
    const attachment = {
      schema_version: 1,
      result_id: "qpls-multimod-result",
      recipe_id: UUIDS.staged,
      result_sha256: "3".repeat(64),
      identity_sha256: "4".repeat(64),
      sidecars: [],
      result: {
        kind: "pls_multigroup_analysis_v1",
        analysis: {
          schema_version: 1,
          provenance: {
            method_version: "qpls.mga.multigroup.v1",
            recipe_id: UUIDS.staged,
            recipe_analytical_sha256: "5".repeat(64),
            config_sha256: "6".repeat(64),
            model_id: authority.modelId,
            model_scientific_sha256: authority.modelScientificSha256,
            dataset_id: authority.datasetId,
            dataset_fingerprint: authority.datasetFingerprint,
            engine_version: "2.56.0-multimod.1",
            seed: 42,
            capability_cell: {
              registry_schema_version: 2,
              capability_id: "quickpls.multimod",
              cell_id: "qpls.multimod.mga.v1",
              capability_version: "mga_multigroup_v1",
            },
            qualification: "unqualified_labs",
          },
          profile: "general_sem_pls",
          group_eligibility: [
            {
              group_id: "a",
              label: "A",
              complete_cases: 10,
              selected_rows: 10,
              eligible: true,
              warnings: [],
              blockers: [],
            },
            {
              group_id: "b",
              label: "B",
              complete_cases: 10,
              selected_rows: 10,
              eligible: true,
              warnings: [],
              blockers: [],
            },
          ],
          group_parameters: [],
          micom_pairs: [],
          omnibus: [],
          pairwise: [],
          multiplicity: "holm",
          replicate_ledgers: [],
          excluded_rows: [],
          sidecars: [],
        },
      },
    };
    const capability = attachment.result.analysis.provenance.capability_cell;
    const canonicalDocument = {
      schema_version: 2,
      document_id: "result.multimod:qpls-multimod-result",
      title: "MultiMod result",
      provenance: {
        run_id: attachment.result_id,
        project_id: UUIDS.project,
        model_id: authority.modelId,
        model_digest: authority.modelScientificSha256,
        dataset_id: UUIDS.dataset,
        dataset_fingerprint: authority.datasetFingerprint,
        recipe_id: UUIDS.staged,
        recipe_digest:
          attachment.result.analysis.provenance.recipe_analytical_sha256,
        capability_cell: capability,
        method_version: attachment.result.analysis.provenance.method_version,
        engine_version: attachment.result.analysis.provenance.engine_version,
        seed: 42,
        workers: 1,
        started_at: "2026-08-24T10:00:00.000Z",
        completed_at: "2026-08-24T10:00:02.000Z",
      },
      capability_cells: [capability],
      sections: [
        {
          id: "multimod_scope",
          title: "Scope",
          table_ids: ["multimod_scope"],
          chart_ids: [],
          capability_cells: [capability],
        },
      ],
      tables: [
        {
          id: "multimod_scope",
          title: "Scope",
          columns: [
            {
              id: "status",
              label: "Status",
              data_type: "text",
              description: "Result status.",
            },
          ],
          rows: [
            {
              id: "completed",
              cells: [{ kind: "text", value: "Completed" }],
            },
          ],
          footnote_ids: [],
          capability_cells: [capability],
        },
      ],
      charts: [],
      notices: [],
      exclusions: [],
      footnotes: [],
      presentation: {
        default_section_id: "multimod_scope",
        default_table_id: "multimod_scope",
        precision: 4,
        missing_value_label: "Not reported",
        chart_defaults: {},
      },
    };
    expect(() =>
      parseNativeMultiModCompletedResultV1({
        schemaVersion: 1,
        jobId: UUIDS.job,
        archivePath: authority.archivePath,
        archiveSha256: "7".repeat(64),
        projectId: UUIDS.project,
        datasetId: UUIDS.dataset,
        modelId: authority.modelId,
        attachment,
        canonicalDocument,
        appendReceipt: {
          schema_version: 1,
          project_id: UUIDS.project,
          result_id: attachment.result_id,
          source_archive_sha256: authority.archiveSha256,
          updated_archive_sha256: "8".repeat(64),
          sidecar_count: 0,
          source_verified_at_commit: true,
          post_write_validated: true,
          rollback_removed: true,
        },
        cacheReceipt: cacheReceipt(),
        cacheRemovedAfterCommit: true,
      }),
    ).toThrow(/identities differ/u);
  });
});
