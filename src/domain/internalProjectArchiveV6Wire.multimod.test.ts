import { describe, expect, it } from "vitest";
import { parseInternalProjectArchiveV6Wire } from "./internalProjectArchiveV6Wire";
import { convertLegacyBasicModelV4 } from "./semModelV4";

const PROJECT_ID = "00000000-0000-0000-0000-000000000101";
const RECIPE_ID = "00000000-0000-0000-0000-000000000202";
const SHA_A = "a".repeat(64);
const SHA_B = "b".repeat(64);

function candidateReceipt() {
  return {
    schema_version: 1,
    authority_binding_sha256: "1".repeat(64),
    candidate_commit_sha: "2".repeat(40),
    candidate_version: "2.56.0",
    qualification_plan_sha256: "3".repeat(64),
    gate_binding_sha256: "4".repeat(64),
    capability_index_sha256: "5".repeat(64),
    prepackage_manifest_set_sha256: "6".repeat(64),
    required_profile_cells: [
      "conditional.multi_two_way_percentile.v2::explicit_path_target_math",
    ],
  };
}

function modelFixture() {
  return convertLegacyBasicModelV4({
    id: "multimod-model",
    name: "MultiMod model",
    constructs: [
      { id: "x", name: "Predictor", short_name: "X", mode: "reflective", indicators: ["x1", "x2"] },
      { id: "m", name: "Mediator", short_name: "M", mode: "reflective", indicators: ["m1", "m2"] },
      { id: "y", name: "Outcome", short_name: "Y", mode: "reflective", indicators: ["y1", "y2"] },
    ],
    paths: [{ source: "x", target: "m" }, { source: "m", target: "y" }],
    controls: [],
    higher_order_constructs: [],
    interactions: [],
  }, "pls_composite");
}

function conditionalConfig() {
  return {
    schema_version: 2,
    profile: "multi_two_way_percentile",
    paths: [{ path_id: "x-m-y", ordered_relation_ids: ["x-m", "m-y"] }],
    declared_interaction_ids: ["x-w-m"],
    hoc_ids: [],
    moderator_ids: ["w"],
    probes: [{ probe_id: "w", moderator_id: "w", scale: "standardized_score", values: [-1, 0, 1] }],
    explicit_joint_tuples: [],
    probe_contrasts: [],
    groups: [],
    group_contrasts: [],
    estimands: {
      conditional_specific_indirect: true,
      conditional_total_indirect: false,
      conditional_total_effect: false,
      scalar_index_when_affine: true,
      local_first_derivatives: false,
      local_second_and_cross_derivatives: false,
      finite_probe_contrasts: false,
    },
    inference: { interval: "percentile", alternative: "two_sided", outer_resamples: 5_000, inner_resamples: 0, seed: 42, confidence_level: 0.95 },
  };
}

function ledger() {
  return {
    requested: 5_000,
    usable: 5_000,
    minimum_required: 4_500,
    usable_fraction: 1,
    complete: true,
    ledger_sha256: SHA_A,
    failure_counts: {},
  };
}

function archiveFixture() {
  const model = modelFixture();
  const capability = { registry_schema_version: 2, capability_id: "quickpls.multimod", cell_id: "qpls.multimod.conditional.multi_two_way_percentile", capability_version: "general_sem_conditional_process_v2" };
  const canonicalDocument = {
    schema_version: 2,
    document_id: "result.multimod:conditional-result-1",
    title: "MultiMod result",
    provenance: {
      run_id: "conditional-result-1",
      project_id: PROJECT_ID,
      model_id: model.id,
      model_digest: SHA_A,
      dataset_id: "00000000-0000-0000-0000-000000000303",
      dataset_fingerprint: "dataset-fingerprint",
      recipe_id: RECIPE_ID,
      recipe_digest: SHA_A,
      capability_cell: capability,
      method_version: "qpls.general-sem-conditional-process.v2",
      engine_version: "2.56.0",
      seed: 42,
      workers: 1,
      started_at: "2026-08-24T09:00:31Z",
      completed_at: "2026-08-24T09:00:32Z",
    },
    capability_cells: [capability],
    sections: [{ id: "multimod_scope", title: "Scope", table_ids: ["multimod_run_provenance", "multimod_scope"], chart_ids: [], capability_cells: [capability] }],
    tables: [
      {
        id: "multimod_run_provenance",
        title: "MultiMod run provenance",
        columns: [
          { id: "qualification", label: "Qualification", data_type: "text", description: "Qualification state." },
          { id: "candidate_qualification_receipt_json", label: "Candidate receipt", data_type: "text", description: "Candidate authority receipt." },
        ],
        rows: [{
          id: "run",
          cells: [
            { kind: "text", value: "unqualified_labs" },
            { kind: "missing", reason: "not_applicable" },
          ],
        }],
        footnote_ids: [],
        capability_cells: [capability],
      },
      {
        id: "multimod_scope",
        title: "Scope",
        columns: [{ id: "status", label: "Status", data_type: "text", description: "Result status." }],
        rows: [{ id: "completed", cells: [{ kind: "text", value: "Completed" }] }],
        footnote_ids: [],
        capability_cells: [capability],
      },
    ],
    charts: [],
    notices: [],
    exclusions: [],
    footnotes: [],
    presentation: { default_section_id: "multimod_scope", default_table_id: "multimod_scope", precision: 4, missing_value_label: "Not reported", chart_defaults: {} },
  };
  return {
    schema_version: 6,
    project_id: PROJECT_ID,
    name: "MultiMod project",
    created_at: "2026-08-24T09:00:00Z",
    modified_at: "2026-08-24T09:01:00Z",
    origin: { kind: "new_project" },
    sem_generation: "general_sem_v1",
    recipes: [{
      schema_version: 4,
      id: RECIPE_ID,
      created_at: "2026-08-24T09:00:30Z",
      dataset_fingerprint: "dataset-fingerprint",
      model_binding: { kind: "embedded_sem_model_v4", model, scientific_sha256: SHA_A },
      estimand_confirmation: "not_legacy",
      settings: {
        method: "moderated_mediation",
        weighting_scheme: "path",
        tolerance: 1e-7,
        max_iterations: 3_000,
        bootstrap_samples: 5_000,
        studentized_inner_samples: 0,
        permutation_samples: 0,
        seed: 42,
        workers: 1,
        confidence_level: 0.95,
        preprocessing: "standardized",
        missing_data: "listwise_deletion",
        case_weight_column: null,
      },
      general_sem_conditional_process: conditionalConfig(),
      metadata: {},
    }],
    canonical_result_documents: [{
      document_id: canonicalDocument.document_id,
      run_id: canonicalDocument.provenance.run_id,
      document_schema_version: 2,
      canonical_document: canonicalDocument,
      canonical_document_sha256: SHA_B,
      immutable: true,
    }],
    multimod_results: [{
      schema_version: 1,
      result_id: "conditional-result-1",
      recipe_id: RECIPE_ID,
      result: {
        kind: "general_sem_conditional_process_result_v2",
        analysis: {
          schema_version: 2,
          provenance: {
            method_version: "qpls.general-sem-conditional-process.v2",
            recipe_id: RECIPE_ID,
            recipe_analytical_sha256: SHA_A,
            config_sha256: SHA_B,
            model_id: model.id,
            model_scientific_sha256: SHA_A,
            dataset_id: "00000000-0000-0000-0000-000000000303",
            dataset_fingerprint: "dataset-fingerprint",
            engine_version: "2.56.0",
            seed: 42,
            capability_cell: capability,
            qualification: "unqualified_labs",
          },
          profile_id: "multi_two_way_percentile",
          targets: [{
            target_id: "specific-indirect:x-m-y@w0",
            kind: "conditional_specific_indirect",
            path_id: "x-m-y",
            probe_values: { w: 0 },
            derivative_variables: [],
            estimate: 0.12,
            usable_replicates: 5_000,
          }],
          replicate_ledger: ledger(),
          sidecars: [],
          warnings: [],
        },
      },
      result_sha256: SHA_A,
      identity_sha256: SHA_B,
      sidecars: [],
    }],
  };
}

describe("schema-v6 MultiMod wire extensions", () => {
  it("reads additive Recipe V4 config and result attachments without changing legacy fields", () => {
    const parsed = parseInternalProjectArchiveV6Wire(archiveFixture());
    expect(parsed.recipes[0].general_sem_conditional_process).toMatchObject({
      schema_version: 2,
      paths: [{ path_id: "x-m-y", ordered_relation_ids: ["x-m", "m-y"] }],
    });
    expect(parsed.multimod_results?.[0]).toMatchObject({
      schema_version: 1,
      result_id: "conditional-result-1",
      recipe_id: RECIPE_ID,
      result: { kind: "general_sem_conditional_process_result_v2" },
    });
  });

  it("defaults the additive attachment collection for old schema-v6 documents", () => {
    const fixture = archiveFixture();
    delete (fixture as { multimod_results?: unknown }).multimod_results;
    expect(parseInternalProjectArchiveV6Wire(fixture).multimod_results).toEqual([]);
  });

  it("couples candidate qualification to its receipt while retaining receipt-free Labs archives", () => {
    const candidate = archiveFixture();
    const provenance = candidate.multimod_results[0].result.analysis.provenance;
    provenance.qualification = "release_qualified_candidate";
    expect(() => parseInternalProjectArchiveV6Wire(candidate)).toThrowError(
      expect.objectContaining({
        code: "project_archive_v6.multimod.candidate_receipt.state_coupling",
      }),
    );

    Object.assign(provenance, {
      candidate_qualification_receipt: candidateReceipt(),
    });
    const authorityRow = candidate.canonical_result_documents[0]
      .canonical_document.tables[0].rows[0];
    authorityRow.cells = [
      { kind: "text", value: "release_qualified_candidate" },
      { kind: "text", value: JSON.stringify(candidateReceipt()) },
    ];
    expect(
      parseInternalProjectArchiveV6Wire(candidate).multimod_results?.[0]
        .result.analysis.provenance.qualification,
    ).toBe("release_qualified_candidate");

    const invalidLabs = archiveFixture();
    Object.assign(
      invalidLabs.multimod_results[0].result.analysis.provenance,
      { candidate_qualification_receipt: candidateReceipt() },
    );
    expect(() => parseInternalProjectArchiveV6Wire(invalidLabs)).toThrowError(
      expect.objectContaining({
        code: "project_archive_v6.multimod.candidate_receipt.state_coupling",
      }),
    );
  });

  it("rejects a MultiMod result whose Recipe V4 is absent", () => {
    const fixture = archiveFixture();
    fixture.multimod_results[0].recipe_id = "00000000-0000-0000-0000-000000000999";
    expect(() => parseInternalProjectArchiveV6Wire(fixture)).toThrowError(expect.objectContaining({
      code: "project_archive_v6.multimod_result_recipe_unavailable",
    }));
  });

  it("rejects drifted fields inside versioned MultiMod configuration", () => {
    const fixture = archiveFixture();
    Object.assign(fixture.recipes[0].general_sem_conditional_process, { silent_path_guessing: true });
    expect(() => parseInternalProjectArchiveV6Wire(fixture)).toThrowError(expect.objectContaining({
      code: "project_archive_v6.multimod.shape.unknown_field",
    }));
  });

  it("requires each MultiMod family to use its frozen Recipe V4 method identity", () => {
    const fixture = archiveFixture();
    fixture.recipes[0].settings.method = "regression";
    expect(() => parseInternalProjectArchiveV6Wire(fixture)).toThrowError(expect.objectContaining({
      code: "project_archive_v6.multimod_recipe_method_mismatch",
    }));
  });

  it("rejects MultiMod Recipe V4 configuration outside a new General SEM project", () => {
    const fixture = archiveFixture();
    delete (fixture as { sem_generation?: unknown }).sem_generation;
    expect(() => parseInternalProjectArchiveV6Wire(fixture)).toThrowError(expect.objectContaining({
      code: "project_archive_v6.multimod_recipe_requires_general_sem_generation",
    }));
  });
});
