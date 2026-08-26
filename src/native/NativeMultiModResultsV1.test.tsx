import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type {
  MultiModResultAttachmentV1,
  MultimodReplicateLedgerSummaryV1,
} from "../domain/multimodContractsV1";
import {
  NativeMultiModResultsV1,
  MULTIMOD_RESULT_DOM_ROW_CAP_V1,
  MULTIMOD_RESULT_VIRTUAL_WINDOW_ROWS_V1,
  nextMultiModResultsTabV1,
  paginateMultiModRowsV1,
  windowMultiModRowsV1,
  type MultiModResultsTabV1,
} from "./NativeMultiModResultsV1";
import {
  NATIVE_MULTIMOD_LABS_ACCESS_V1,
  NATIVE_MULTIMOD_STANDARD_ACCESS_V1,
} from "./nativeMultiModJobV1";

const SHA = {
  recipe: "a".repeat(64),
  config: "b".repeat(64),
  model: "c".repeat(64),
  dataset: "d".repeat(64),
  ledger: "e".repeat(64),
  result: "f".repeat(64),
  identity: "1".repeat(64),
};

function ledger(complete = true): MultimodReplicateLedgerSummaryV1 {
  return complete
    ? {
        requested: 10,
        usable: 10,
        minimum_required: 9,
        usable_fraction: 1,
        complete: true,
        ledger_sha256: SHA.ledger,
        failure_counts: {},
        failures: [],
      }
    : {
        requested: 10,
        usable: 5,
        minimum_required: 9,
        usable_fraction: 0.5,
        complete: false,
        ledger_sha256: SHA.ledger,
        failure_counts: { rank_deficient: 5 },
        failures: [
          {
            replicate_index: 3,
            kind: "rank_deficient",
            stable_code: "multimod.bootstrap.rank_deficient",
            detail: "The resampled design was rank deficient.",
          },
        ],
      };
}

function attachment(
  options: { rows?: number; complete?: boolean } = {},
): MultiModResultAttachmentV1 {
  const rows = options.rows ?? 1;
  const complete = options.complete ?? true;
  return {
    schema_version: 1,
    result_id: "result-mga-v1",
    recipe_id: "00000000-0000-0000-0000-000000000101",
    result_sha256: SHA.result,
    identity_sha256: SHA.identity,
    sidecars: [],
    result: {
      kind: "pls_multigroup_analysis_v1",
      analysis: {
        schema_version: 1,
        provenance: {
          method_version: "qpls.mga.multigroup.v1",
          recipe_id: "00000000-0000-0000-0000-000000000101",
          recipe_analytical_sha256: SHA.recipe,
          config_sha256: SHA.config,
          model_id: "model-multimod",
          model_scientific_sha256: SHA.model,
          dataset_id: "00000000-0000-0000-0000-000000000204",
          dataset_fingerprint: SHA.dataset,
          engine_version: "2.56.0",
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
            label: "Group A",
            complete_cases: 40,
            selected_rows: 40,
            eligible: complete,
            warnings: [],
            blockers: complete ? [] : ["Minimum usable-resample gate failed."],
          },
          {
            group_id: "b",
            label: "Group B",
            complete_cases: 38,
            selected_rows: 38,
            eligible: true,
            warnings: [],
            blockers: [],
          },
        ],
        group_parameters: Array.from({ length: rows }, (_, index) => ({
          group_id: index % 2 ? "b" : "a",
          parameter: {
            target_id: `path-target-${String(index).padStart(3, "0")}`,
            target_kind: "structural_path",
            estimate: complete ? index / 100 : 123456.789,
            standard_error: 0.05,
            p_value: 0.02,
            interval: {
              confidence_level: 0.95,
              lower: 0.1,
              family: "percentile_type7",
              alternative: "greater",
            },
          },
        })),
        micom_pairs: [],
        omnibus: [],
        pairwise: [],
        multiplicity: "holm",
        replicate_ledgers: [ledger(complete)],
        excluded_rows: [],
        sidecars: [],
      },
    },
  };
}

function standardQualifiedAttachment(): MultiModResultAttachmentV1 {
  const result = attachment();
  result.result.analysis.provenance = {
    ...result.result.analysis.provenance,
    qualification: "release_qualified_candidate",
    candidate_qualification_receipt: {
      schema_version: 1,
      authority_binding_sha256: "2".repeat(64),
      candidate_commit_sha: "3".repeat(40),
      candidate_version: "2.56.0",
      qualification_plan_sha256: "4".repeat(64),
      gate_binding_sha256: "5".repeat(64),
      capability_index_sha256: "6".repeat(64),
      prepackage_manifest_set_sha256: "7".repeat(64),
      required_profile_cells: ["mga.general_sem_pls.v1::point_estimation"],
    },
  };
  return result;
}

function heterogeneityAttachment(): MultiModResultAttachmentV1 {
  const base = attachment();
  return {
    ...base,
    result_id: "result-heterogeneity-v2",
    result: {
      kind: "pls_heterogeneity_analysis_v2",
      analysis: {
        schema_version: 2,
        provenance: base.result.analysis.provenance,
        profile: "p0_structural",
        candidates: [
          {
            method: { kind: "pooled_baseline_v1" },
            k: 1,
            state: "eligible",
            converged_starts: 0,
            stable_starts: 0,
            criteria: {},
            class_or_segment_shares: [],
            pooled_parameters: [
              {
                target_id: "pooled:path:x-y",
                target_kind: "structural_path",
                estimate: 0.25,
              },
            ],
            blockers: [],
          },
        ],
        discovery_result_identity_sha256: "9".repeat(64),
        parameters: [],
        contrasts: [],
        sidecars: [],
        descriptive_only: false,
      },
    },
  };
}

function conditionalAttachment(): MultiModResultAttachmentV1 {
  const base = attachment();
  return {
    ...base,
    result_id: "result-conditional-v2",
    result: {
      kind: "general_sem_conditional_process_result_v2",
      analysis: {
        schema_version: 2,
        provenance: base.result.analysis.provenance,
        profile_id: "multi_two_way_percentile",
        targets: [
          {
            target_id: "indirect-at-high-w",
            kind: "conditional_specific_indirect",
            path_id: "x-m-y",
            probe_values: { w: 1 },
            derivative_variables: [],
            estimate: 0.25,
            p_value: 0.01,
            interval: {
              confidence_level: 0.95,
              upper: 0.4,
              family: "percentile_type7",
              alternative: "less",
            },
            usable_replicates: 10,
          },
        ],
        replicate_ledger: ledger(),
        sidecars: [],
        warnings: [],
      },
    },
  };
}

function causalAttachment(): MultiModResultAttachmentV1 {
  const base = attachment();
  return {
    ...base,
    result_id: "result-causal-v1",
    result: {
      kind: "interventional_mediation_result_v1",
      analysis: {
        schema_version: 1,
        provenance: base.result.analysis.provenance,
        interpretation_label: "assumption-dependent interventional estimate",
        identification_assumptions: [
          "Temporal ordering and the adjustment set are defensible.",
        ],
        positivity: [
          {
            variable_id: "treatment",
            observed_minimum: 0,
            observed_maximum: 1,
            requested_value: 1,
            support_count: 50,
            minimum_required_count: 10,
            support_rule: "binary_arm_count",
            supported: true,
          },
        ],
        effects: [
          {
            target_id: "interventional-indirect-x-m-y",
            path_id: "x-m-y",
            estimand: "interventional_indirect_effect",
            estimate: 0.18,
            p_value: 0.04,
            interval: {
              confidence_level: 0.95,
              lower: 0.02,
              upper: 0.31,
              family: "percentile_type7",
              alternative: "two_sided",
            },
          },
        ],
        replicate_ledger: ledger(),
        sidecars: [],
      },
    },
  };
}

describe("NativeMultiModResultsV1", () => {
  it("renders an accessible result tab contract and a bounded first table page", () => {
    const html = renderToStaticMarkup(
      <NativeMultiModResultsV1
        access={NATIVE_MULTIMOD_LABS_ACCESS_V1}
        validatedResult={attachment({ rows: 61 })}
        initialTab="estimates"
      />,
    );

    expect(html).toContain('data-multimod-results="v1"');
    expect(html).toContain('data-scientific-result-ready="true"');
    expect(html.match(/role="tab"/gu)).toHaveLength(8);
    expect(html).toContain('role="tablist"');
    expect(html).toContain('role="tabpanel"');
    expect(html).toContain(
      'aria-labelledby="nd-multimod-results-tab-estimates"',
    );
    expect(html).toContain('role="grid"');
    expect(html).toContain('aria-keyshortcuts="Control+C"');
    expect(html).toContain("Use the arrow keys to move between cells.");
    expect(html).toContain("Showing 1–50 of 61 rows. Page 1 of 2.");
    expect(html).toContain(
      'aria-label="Next page of Group-specific parameters"',
    );
    expect(html).toContain("path-target-024");
    expect(html).not.toContain("path-target-025");
    expect(html).toContain("Not applicable");
  });

  it("withholds estimates and inference for incomplete results while retaining failure evidence", () => {
    const html = renderToStaticMarkup(
      <NativeMultiModResultsV1
        access={NATIVE_MULTIMOD_LABS_ACCESS_V1}
        validatedResult={attachment({ complete: false })}
        initialTab="failures"
      />,
    );

    expect(html).toContain('data-scientific-result-ready="false"');
    expect(html).toContain("Partial scientific output is not displayed.");
    expect(html).toContain(
      "Group Group A is ineligible for multigroup interpretation.",
    );
    expect(html).toMatch(
      /<button(?=[^>]*id="nd-multimod-results-tab-estimates")(?=[^>]*disabled="")[^>]*>/u,
    );
    expect(html).toMatch(
      /<button(?=[^>]*id="nd-multimod-results-tab-inference")(?=[^>]*disabled="")[^>]*>/u,
    );
    expect(html).toContain("multimod.bootstrap.rank_deficient");
    expect(html).toContain("The resampled design was rank deficient.");
    expect(html).not.toContain("123,456.789");
  });

  it("projects each strict result family without substituting another method's semantics", () => {
    const heterogeneity = renderToStaticMarkup(
      <NativeMultiModResultsV1
        access={NATIVE_MULTIMOD_LABS_ACCESS_V1}
        validatedResult={heterogeneityAttachment()}
        initialTab="diagnostics"
      />,
    );
    expect(heterogeneity).toContain("PLS unobserved heterogeneity");
    expect(heterogeneity).toContain("Candidate segmentation diagnostics");
    expect(heterogeneity).toContain("Pooled baseline");

    const conditional = renderToStaticMarkup(
      <NativeMultiModResultsV1
        access={NATIVE_MULTIMOD_LABS_ACCESS_V1}
        validatedResult={conditionalAttachment()}
        initialTab="inference"
      />,
    );
    expect(conditional).toContain("General SEM conditional process");
    expect(conditional).toContain("indirect-at-high-w");
    expect(conditional).toContain("Not applicable");

    const causal = renderToStaticMarkup(
      <NativeMultiModResultsV1
        access={NATIVE_MULTIMOD_LABS_ACCESS_V1}
        validatedResult={causalAttachment()}
        initialTab="estimates"
      />,
    );
    expect(causal).toContain("Interventional causal mediation");
    expect(causal).toContain("assumption-dependent interventional estimate");
    expect(causal).toContain("Interventional Indirect Effect");
  });

  it("labels MICOM strictly as composite invariance", () => {
    const result = attachment();
    if (result.result.kind !== "pls_multigroup_analysis_v1")
      throw new Error("MGA fixture expected");
    result.result.analysis.micom_pairs = [
      {
        left_group_id: "a",
        right_group_id: "b",
        construct_id: "construct-1",
        interpretation: "composite_invariance",
        configural_invariance_confirmed: true,
        compositional_correlation: 0.99,
        compositional_lower_quantile: 0.95,
        compositional_p_value: 0.8,
        compositional_invariance: true,
        partial_invariance: true,
        equal_mean_p_value: 0.4,
        equal_variance_p_value: 0.3,
      },
    ];
    const html = renderToStaticMarkup(
      <NativeMultiModResultsV1
        access={NATIVE_MULTIMOD_LABS_ACCESS_V1}
        validatedResult={result}
        initialTab="diagnostics"
      />,
    );
    expect(html).toContain("Composite Invariance");
    expect(html).toContain("No omnibus MICOM claim is made");
  });

  it("renders no scientific values when the allegedly validated attachment fails local revalidation", () => {
    const invalid = { ...attachment(), result_sha256: "not-a-sha" };
    const html = renderToStaticMarkup(
      <NativeMultiModResultsV1
        access={NATIVE_MULTIMOD_LABS_ACCESS_V1}
        validatedResult={invalid}
      />,
    );

    expect(html).toContain('data-multimod-results="invalid"');
    expect(html).toContain("MultiMod result withheld");
    expect(html).toContain("No scientific values were rendered.");
    expect(html).not.toContain('role="tablist"');
  });

  it("renders qualified Standard results without user-facing Labs labels", () => {
    const html = renderToStaticMarkup(
      <NativeMultiModResultsV1
        access={NATIVE_MULTIMOD_STANDARD_ACCESS_V1}
        validatedResult={standardQualifiedAttachment()}
      />,
    );

    expect(html).toContain("Standard · Release-qualified");
    expect(html).not.toMatch(/Experimental Labs|Labs output|Labs qualification/iu);
  });

  it("uses WAI-ARIA tab navigation order and skips withheld scientific tabs", () => {
    const disabled = new Set<MultiModResultsTabV1>(["estimates", "inference"]);
    expect(
      nextMultiModResultsTabV1("diagnostics", "ArrowRight", disabled),
    ).toBe("exclusions");
    expect(nextMultiModResultsTabV1("eligibility", "ArrowLeft", disabled)).toBe(
      "sidecars",
    );
    expect(nextMultiModResultsTabV1("failures", "Home", disabled)).toBe(
      "eligibility",
    );
    expect(nextMultiModResultsTabV1("failures", "End", disabled)).toBe(
      "sidecars",
    );
    expect(nextMultiModResultsTabV1("failures", "Enter", disabled)).toBeNull();
  });

  it("clamps pagination without dropping or duplicating the final page", () => {
    const rows = Array.from({ length: 101 }, (_, index) => index);
    const page = paginateMultiModRowsV1(rows, 99, 50);
    expect(page.page).toBe(2);
    expect(page.pageCount).toBe(3);
    expect(page.start).toBe(100);
    expect(page.end).toBe(101);
    expect(page.rows).toEqual([100]);
    const capped = paginateMultiModRowsV1(
      Array.from({ length: 250 }, (_, index) => index),
      0,
      1_000,
    );
    expect(capped.rows).toHaveLength(MULTIMOD_RESULT_DOM_ROW_CAP_V1);
    expect(capped.pageCount).toBe(3);
  });

  it("renders a clamped virtual row window within a bounded page", () => {
    const rows = Array.from({ length: 100 }, (_, index) => index);
    const first = windowMultiModRowsV1(rows, 0, 100);
    expect(first.rows).toHaveLength(MULTIMOD_RESULT_VIRTUAL_WINDOW_ROWS_V1);
    expect(first.start).toBe(0);
    expect(first.end).toBe(25);
    const last = windowMultiModRowsV1(rows, 99);
    expect(last.start).toBe(75);
    expect(last.end).toBe(100);
    expect(last.rows[0]).toBe(75);
  });
});
