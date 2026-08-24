import { writeFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import type { CanonicalResultDocumentV2 } from "./canonicalResultDocumentV2";
import {
  prepareCanonicalResultExportV2,
  readPreparedCanonicalResultExportSemanticV2,
  verifyPreparedCanonicalResultExportV2,
  type CanonicalResultExportFormatV2,
  type PreparedCanonicalResultExportV2,
} from "./canonicalResultCrossFormatExportV2";

const FORMATS = ["csv", "xlsx", "json", "html", "pdf", "svg", "png"] as const;
const DIGEST = "a".repeat(64);

interface ProfileFixtureV1 {
  family: "mga" | "heterogeneity" | "conditional" | "causal";
  familyCapability: string;
  profile: string;
  requiredCell: string;
  tableId: string;
}

const PROFILES: readonly ProfileFixtureV1[] = [
  ...[
    ["mga.general_sem_pls.v1", "point_fit"],
    ["mga.multiple_two_way_moderation.v1", "point_fit_path_gamma_slopes"],
    ["mga.bounded_three_way_moderation.v1", "point_fit_path_gamma_delta_slopes"],
    ["mga.bounded_two_way_moderated_mediation.v1", "point_fit_bounded_conditional_targets"],
    ["mga.multiple_nonnested_hoc.v1", "point_fit_hoc_stages"],
    ["mga.case_weighted_pls.v1", "weighted_point_fit"],
    ["mga.frequency_weighted_pls.v1", "count_space_point_fit"],
    ["mga.reflective_plsc.v1", "plsc_point_fit"],
  ].map(([profile, procedure]) => ({
    family: "mga" as const,
    familyCapability: "qpls.multimod.mga_multigroup_v1",
    profile: profile!,
    requiredCell: `${profile}::${procedure}`,
    tableId: "mga_pairwise_comparisons",
  })),
  ...[
    ["fimix.p0_structural.v2", "em_multistart_point"],
    ["fimix.p2_multi_two_way.v2", "pooled_metric_products"],
    ["fimix.p23_all_current.v2", "pooled_metric_three_way_products"],
    ["pos.published.p0_structural.v2", "ten_start_full_refit_point"],
    ["pos.destination_scored.p2_multi_two_way.v2", "ten_start_full_refit_point"],
    ["pos.destination_scored.p23_all_current.v2", "ten_start_full_refit_point"],
    ["pos.common_metric.p2_multi_two_way.v1", "configural_and_pairwise_compositional_gate"],
    ["pos.common_metric.p23_all_current.v1", "configural_and_pairwise_compositional_gate"],
  ].map(([profile, procedure]) => ({
    family: "heterogeneity" as const,
    familyCapability: "qpls.multimod.pls_heterogeneity_v2",
    profile: profile!,
    requiredCell: `${profile}::${procedure}`,
    tableId: "heterogeneity_candidate_criteria",
  })),
  ...[
    ["conditional.multi_two_way_percentile.v2", "explicit_path_target_math"],
    ["conditional.multi_two_way_bca.v2", "explicit_path_target_math"],
    ["conditional.studentized.v2", "nested_studentized"],
    ["conditional.bounded_three_way_percentile.v2", "complete_lower_order_closure"],
    ["conditional.multiple_hoc_percentile.v2", "hoc_dependency_before_products"],
    ["conditional.grouped_percentile.v2", "group_stratified_shared_ledger"],
    ["conditional.case_weighted_percentile.v2", "positive_normalized_case_weights"],
    ["conditional.frequency_weighted_percentile.v2", "count_space_point_equivalence"],
  ].map(([profile, procedure]) => ({
    family: "conditional" as const,
    familyCapability: "qpls.multimod.general_sem_conditional_process_v2",
    profile: profile!,
    requiredCell: `${profile}::${procedure}`,
    tableId: "conditional_process_targets",
  })),
  {
    family: "causal",
    familyCapability: "qpls.multimod.interventional_causal_mediation_v1",
    profile: "interventional.observed_gcomp.v1",
    requiredCell: "interventional.observed_gcomp.v1::observed_equation_point_fit",
    tableId: "interventional_effects",
  },
];

function candidateReceipt(requiredCell: string) {
  return {
    schema_version: 1 as const,
    authority_binding_sha256: "1".repeat(64),
    candidate_commit_sha: "2".repeat(40),
    candidate_version: "2.56.0",
    qualification_plan_sha256: "3".repeat(64),
    gate_binding_sha256: "4".repeat(64),
    capability_index_sha256: "5".repeat(64),
    prepackage_manifest_set_sha256: "6".repeat(64),
    required_profile_cells: [requiredCell],
  };
}

function profileDocument(profile: ProfileFixtureV1): CanonicalResultDocumentV2 {
  const capability = {
    registry_schema_version: 2 as const,
    capability_id: "quickpls.multimod",
    cell_id: profile.familyCapability,
    capability_version: profile.familyCapability,
  };
  const receipt = candidateReceipt(profile.requiredCell);
  const chartId = `${profile.family}_qualification_chart`;
  const mainTable = {
    id: profile.tableId,
    title: `${profile.profile} estimates`,
    description: "Typed MultiMod qualification export fixture.",
    columns: [
      { id: "target_id", label: "Target", data_type: "text" as const, description: "Stable target identity.", role: "label" as const },
      { id: "estimate", label: "Estimate", data_type: "number" as const, description: "Scientific estimate.", role: "estimate" as const, default_precision: 6 },
      { id: "p_value", label: "Probability", data_type: "number" as const, description: "Inferential probability when eligible.", role: "uncertainty" as const, default_precision: 6 },
    ],
    rows: [
      { id: "qualified_target", cells: [{ kind: "text" as const, value: `${profile.profile}:target` }, { kind: "number" as const, value: 0.25 }, { kind: "number" as const, value: 0.04 }] },
      { id: "suppressed_target", cells: [{ kind: "text" as const, value: `${profile.profile}:suppressed` }, { kind: "number" as const, value: -0.1 }, { kind: "missing" as const, reason: "not_estimated" as const }] },
    ],
    footnote_ids: ["suppressed_inference"],
    capability_cells: [capability],
  };
  const provenanceTable = {
    id: "multimod_run_provenance",
    title: "MultiMod run provenance",
    columns: [
      { id: "profile", label: "Profile", data_type: "text" as const, description: "Exact profile.", role: "provenance" as const },
      { id: "qualification", label: "Qualification", data_type: "text" as const, description: "Qualification state.", role: "decision" as const },
      { id: "candidate_qualification_receipt_json", label: "Candidate receipt", data_type: "text" as const, description: "Exact build authority receipt.", role: "provenance" as const },
    ],
    rows: [{
      id: "run",
      cells: [
        { kind: "text" as const, value: profile.profile },
        { kind: "text" as const, value: "release_qualified_candidate" },
        { kind: "text" as const, value: JSON.stringify(receipt) },
      ],
    }],
    footnote_ids: [],
    capability_cells: [capability],
  };
  return {
    schema_version: 2,
    document_id: `result.multimod:${profile.profile}`,
    title: `${profile.profile} qualification export`,
    provenance: {
      run_id: `run:${profile.profile}`,
      project_id: "project:multimod-export-qualification",
      model_id: `model:${profile.profile}`,
      model_digest: DIGEST,
      dataset_id: `dataset:${profile.profile}`,
      dataset_fingerprint: "b".repeat(64),
      recipe_id: `recipe:${profile.profile}`,
      recipe_digest: "c".repeat(64),
      capability_cell: capability,
      method_version: profile.profile,
      engine_version: "qpls-multimod-export-qualification-v1",
      seed: 42,
      workers: 4,
      started_at: "2026-08-24T10:00:00Z",
      completed_at: "2026-08-24T10:01:00Z",
    },
    capability_cells: [capability],
    sections: [{
      id: "multimod_results",
      title: "MultiMod results",
      table_ids: [profile.tableId, "multimod_run_provenance"],
      chart_ids: [chartId],
      capability_cells: [capability],
    }],
    tables: [mainTable, provenanceTable],
    charts: [{
      id: chartId,
      title: `${profile.profile} effect chart`,
      description: "Chart backed by the exact selected canonical estimate table.",
      kind: "interval",
      series: [{
        id: "effect",
        label: "Effect",
        points: [{ x: 1, y: 0.25, lower: 0.05, upper: 0.45, label: "qualified_target" }],
      }],
      source_table_id: profile.tableId,
      display: { show_legend: true, x_axis_label: "Target", y_axis_label: "Estimate" },
    }],
    notices: [{
      id: "candidate_boundary",
      code: "multimod_release_qualified_candidate",
      severity: "information",
      message: "This exact result carries a build-bound candidate receipt.",
      section_ids: ["multimod_results"],
      table_ids: ["multimod_run_provenance"],
    }],
    exclusions: [{
      id: "suppressed_inference_guard",
      title: "Suppressed inference",
      reason: "Ineligible inference is represented as missing and must not reappear in exports.",
    }],
    footnotes: [{ id: "suppressed_inference", text: "Missing inference remains suppressed in every export format." }],
    presentation: {
      default_section_id: "multimod_results",
      default_table_id: profile.tableId,
      precision: 6,
      missing_value_label: "Not estimated",
      chart_defaults: { show_legend: true },
    },
  };
}

function preparedFor(
  document: CanonicalResultDocumentV2,
  profile: ProfileFixtureV1,
  format: CanonicalResultExportFormatV2,
): PreparedCanonicalResultExportV2 {
  const chartId = `${profile.family}_qualification_chart`;
  const chartOnly = format === "svg" || format === "png";
  const rich = format === "json" || format === "html" || format === "pdf";
  const prepared = prepareCanonicalResultExportV2(document, {
    format,
    tableIds: chartOnly ? [] : [profile.tableId, "multimod_run_provenance"],
    chartIds: chartOnly || rich ? [chartId] : [],
  });
  if (!prepared.ok) throw new Error(prepared.errors.join("\n"));
  return prepared.artifact;
}

describe("MultiMod semantic export qualification matrix", () => {
  it("semantically round-trips every admitted profile through every table and chart format", () => {
    const rows: Array<Record<string, unknown>> = [];
    for (const profile of PROFILES) {
      const document = profileDocument(profile);
      for (const format of FORMATS) {
        const artifact = preparedFor(document, profile, format);
        const verification = verifyPreparedCanonicalResultExportV2(document, artifact);
        expect(verification).toMatchObject({
          passed: true,
          exact_semantic_match: true,
          digest_match: true,
          rendered_surface_match: true,
        });
        const semantic = readPreparedCanonicalResultExportSemanticV2(artifact);
        expect(semantic?.publication_qualification).toBe("release_qualified_candidate");
        expect(semantic?.candidate_qualification_receipt?.required_profile_cells).toEqual([profile.requiredCell]);
        const suppressed = semantic?.tables
          .find((table) => table.id === profile.tableId)?.rows
          .find((row) => row.id === "suppressed_target")?.cells[2];
        if (format !== "svg" && format !== "png") {
          expect(suppressed).toEqual({ kind: "missing", reason: "not_estimated" });
        }
        rows.push({
          family: profile.family,
          family_capability: profile.familyCapability,
          profile: profile.profile,
          required_cell: profile.requiredCell,
          format,
          publication_qualification: semantic?.publication_qualification,
          semantic_sha256: artifact.semantic.semantic_sha256,
          exact_semantic_match: verification.exact_semantic_match,
          digest_match: verification.digest_match,
          rendered_surface_match: verification.rendered_surface_match,
          receipt_bound: semantic?.candidate_qualification_receipt?.required_profile_cells[0] === profile.requiredCell,
          suppression_preserved: format === "svg" || format === "png" || suppressed?.kind === "missing",
        });
      }
    }

    const reportPath = process.env.QPLS_MULTIMOD_EXPORT_REPORT;
    if (reportPath) {
      writeFileSync(reportPath, `${JSON.stringify({
        schema_version: 1,
        report_id: "qpls.multimod.semantic-export-qualification.v1",
        status: "passed",
        profile_count: PROFILES.length,
        formats: FORMATS,
        rows,
      }, null, 2)}\n`, "utf8");
    }
    expect(rows).toHaveLength(PROFILES.length * FORMATS.length);
  });

  it("rejects semantic bytes and candidate-receipt tampering without an export fallback", () => {
    const profile = PROFILES[0]!;
    const document = profileDocument(profile);
    const artifact = preparedFor(document, profile, "json");
    if (artifact.format !== "json") throw new Error("JSON fixture returned another format");
    const tampered = {
      ...artifact,
      contents: artifact.contents.replace("0.25", "0.99"),
    } as PreparedCanonicalResultExportV2;
    expect(verifyPreparedCanonicalResultExportV2(document, tampered)).toMatchObject({ passed: false });

    const provenance = document.tables.find((table) => table.id === "multimod_run_provenance")!;
    provenance.rows[0]!.cells[2] = {
      kind: "text",
      value: JSON.stringify({ ...candidateReceipt(profile.requiredCell), required_profile_cells: [] }),
    };
    const blocked = prepareCanonicalResultExportV2(document, {
      format: "json",
      tableIds: [profile.tableId],
      chartIds: [],
    });
    expect(blocked.ok).toBe(false);
  });
});
