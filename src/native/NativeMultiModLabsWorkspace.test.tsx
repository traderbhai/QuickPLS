import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { MultiModResultAttachmentV1 } from "../domain/multimodContractsV1";
import type { SemModelV4 } from "../domain/semModelV4";
import {
  NativeMultiModLabsWorkspace,
  nativeHeterogeneityDiscoveryLockSourceV2,
  nativeMultiModModelInventoryV1,
  type NativeMultiModGroupingColumnV1,
} from "./NativeMultiModLabsWorkspace";

function modelFixture(): SemModelV4 {
  return {
    schema_version: 4,
    id: "model-multimod",
    name: "MultiMod fixture",
    variables: [
      {
        kind: "observed",
        id: "x",
        label: "Treatment",
        source_column: "x",
        scale: "binary",
        role: "structural",
        categories: ["0", "1"],
        value_labels: {},
        missing_markers: [],
        transformation_lineage: [],
      },
      {
        kind: "observed",
        id: "m",
        label: "Mediator",
        source_column: "m",
        scale: "continuous",
        role: "structural",
        categories: [],
        value_labels: {},
        missing_markers: [],
        transformation_lineage: [],
      },
      {
        kind: "observed",
        id: "y",
        label: "Outcome",
        source_column: "y",
        scale: "continuous",
        role: "structural",
        categories: [],
        value_labels: {},
        missing_markers: [],
        transformation_lineage: [],
      },
      {
        kind: "observed",
        id: "w",
        label: "Moderator",
        source_column: "w",
        scale: "continuous",
        role: "structural",
        categories: [],
        value_labels: {},
        missing_markers: [],
        transformation_lineage: [],
      },
      { kind: "derived", id: "xw", label: "Treatment × Moderator" },
    ],
    relations: [
      {
        kind: "structural",
        id: "rel-x-m",
        source: "x",
        target: "m",
        parameter: "beta-x-m",
      },
      {
        kind: "structural",
        id: "rel-m-y",
        source: "m",
        target: "y",
        parameter: "beta-m-y",
      },
      {
        kind: "structural",
        id: "rel-xw-m",
        source: "xw",
        target: "m",
        parameter: "gamma-xw-m",
      },
    ],
    parameters: [
      {
        kind: "free",
        id: "beta-x-m",
        label: "X to M",
        target: { kind: "regression", source: "x", target: "m" },
      },
      {
        kind: "free",
        id: "beta-m-y",
        label: "M to Y",
        target: { kind: "regression", source: "m", target: "y" },
      },
      {
        kind: "free",
        id: "gamma-xw-m",
        label: "XW to M",
        target: { kind: "regression", source: "xw", target: "m" },
      },
    ],
    constraints: [],
    derived_terms: [
      {
        kind: "interaction_v2",
        id: "interaction-x-w-m",
        output: "xw",
        operands: ["x", "w"],
        focal_relation: "rel-x-m",
        method: "two_stage",
        hierarchy_policy: "strong",
      },
    ],
    group: { kind: "single_group" },
    data_binding: {
      kind: "raw",
      dataset_id: "fixture",
      missing_data: "listwise_deletion",
      weight: null,
    },
    annotations: [],
    presentation: { kind: "none" },
  };
}

const groupingColumns: NativeMultiModGroupingColumnV1[] = [
  {
    column: "sector",
    label: "Sector",
    usedAsIndicator: false,
    groups: [
      {
        groupId: "public",
        label: "Public",
        value: { kind: "text", value: "public" },
        completeCases: 60,
      },
      {
        groupId: "private",
        label: "Private",
        value: { kind: "text", value: "private" },
        completeCases: 55,
      },
      {
        groupId: "nonprofit",
        label: "Nonprofit",
        value: { kind: "text", value: "nonprofit" },
        completeCases: 40,
      },
    ],
  },
];

function validatedResultFixture(): MultiModResultAttachmentV1 {
  return {
    schema_version: 1,
    result_id: "workspace-result",
    recipe_id: "00000000-0000-0000-0000-000000000101",
    result_sha256: "f".repeat(64),
    identity_sha256: "1".repeat(64),
    sidecars: [],
    result: {
      kind: "pls_multigroup_analysis_v1",
      analysis: {
        schema_version: 1,
        provenance: {
          method_version: "qpls.mga.multigroup.v1",
          recipe_id: "00000000-0000-0000-0000-000000000101",
          recipe_analytical_sha256: "a".repeat(64),
          config_sha256: "b".repeat(64),
          model_id: "model-multimod",
          model_scientific_sha256: "c".repeat(64),
          dataset_id: "00000000-0000-0000-0000-000000000204",
          dataset_fingerprint: "d".repeat(64),
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
            label: "A",
            complete_cases: 20,
            selected_rows: 20,
            eligible: true,
            warnings: [],
            blockers: [],
          },
          {
            group_id: "b",
            label: "B",
            complete_cases: 20,
            selected_rows: 20,
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
}

function heterogeneityDiscoveryFixture(): MultiModResultAttachmentV1 {
  const base = validatedResultFixture();
  const segmentation = (
    ["fimix_pls_v2", "pls_pos_published_v2"] as const
  ).flatMap((algorithm) =>
    [2, 3].map((k) => ({
      method: { kind: "segmentation" as const, algorithm },
      k,
      state: "converged_stable" as const,
      converged_starts: 3,
      stable_starts: 2,
      criteria: {},
      class_or_segment_shares: Array.from({ length: k }, () => 1 / k),
      pooled_parameters: [],
      blockers: [],
    })),
  );
  return {
    ...base,
    result_id: "workspace-heterogeneity-discovery",
    identity_sha256: "8".repeat(64),
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
                target_id: "pooled:path:x-m",
                family: "structural_path",
                estimate: 0.2,
                standardized: true,
              },
            ],
            blockers: [],
          },
          ...segmentation,
        ],
        discovery_result_identity_sha256: "7".repeat(64),
        parameters: [],
        contrasts: [],
        sidecars: [],
        descriptive_only: false,
      },
    },
  };
}

describe("Native MultiMod Labs workspace", () => {
  it("derives indirect paths and authored interaction identities without inventing paths", () => {
    const inventory = nativeMultiModModelInventoryV1(modelFixture());
    expect(inventory.paths).toContainEqual(
      expect.objectContaining({
        orderedRelationIds: ["rel-x-m", "rel-m-y"],
        orderedVariableIds: ["x", "m", "y"],
      }),
    );
    expect(inventory.interactions).toEqual([
      expect.objectContaining({
        id: "interaction-x-w-m",
        moderatorIds: ["w"],
        order: 2,
      }),
    ]);
    expect(
      inventory.mgaParameters.map((parameter) => parameter.id),
    ).toEqual(
      expect.arrayContaining([
        "beta-x-m",
        "beta-m-y",
        "rel-xw-m",
        "simple_slope:interaction-x-w-m:w:minus_1",
        "simple_slope:interaction-x-w-m:w:zero",
        "simple_slope:interaction-x-w-m:w:plus_1",
      ]),
    );
    expect(
      inventory.mgaParameters.map((parameter) => parameter.id),
    ).not.toContain("gamma-xw-m");
  });

  it("exposes the ARIA tab contract and keeps native execution disabled until wired", () => {
    const html = renderToStaticMarkup(
      <NativeMultiModLabsWorkspace
        model={modelFixture()}
        caseCount={155}
        groupingColumns={groupingColumns}
      />,
    );
    expect(html).toContain("Experimental Labs · Unqualified");
    expect(html.match(/role="tab"/gu)).toHaveLength(4);
    expect(html).toContain('role="tablist"');
    expect(html).toContain('aria-selected="true"');
    expect(html).not.toContain('data-multimod-results="v1"');
    expect(html).toMatch(/<button class="primary" type="button" disabled=""/u);
    expect(html).toContain(
      "The MultiMod native runner command is not connected in this build.",
    );
  });

  it("never preselects a conditional indirect path", () => {
    const html = renderToStaticMarkup(
      <NativeMultiModLabsWorkspace
        model={modelFixture()}
        caseCount={155}
        groupingColumns={groupingColumns}
        initialTab="conditional"
      />,
    );
    const pathInventory =
      html.match(
        /<fieldset class="nd-multimod-option-list"><legend>1\. Explicit indirect paths<\/legend>(.*?)<\/fieldset>/u,
      )?.[1] ?? "";
    expect(pathInventory).toContain("rel-x-m");
    expect(pathInventory).not.toContain('checked=""');
    expect(html).toContain("general_sem_conditional_process_v2.path_count");
  });

  it("derives inference locks only from a complete validated discovery table", () => {
    const source = nativeHeterogeneityDiscoveryLockSourceV2(
      heterogeneityDiscoveryFixture(),
    );
    expect(source).toMatchObject({
      ready: true,
      discoveryResultIdentitySha256: "7".repeat(64),
      candidateK: [2, 3],
      algorithms: ["fimix_pls_v2", "pls_pos_published_v2"],
    });
    if (source.ready) expect(source.selectable).toHaveLength(4);

    const incomplete = heterogeneityDiscoveryFixture();
    if (incomplete.result.kind === "pls_heterogeneity_analysis_v2")
      incomplete.result.analysis.candidates =
        incomplete.result.analysis.candidates.filter(
          (candidate) =>
            !(
              candidate.method.kind === "segmentation" &&
              candidate.method.algorithm === "pls_pos_published_v2" &&
              candidate.k === 3
            ),
        );
    expect(nativeHeterogeneityDiscoveryLockSourceV2(incomplete)).toMatchObject({
      ready: false,
    });
    expect(
      nativeHeterogeneityDiscoveryLockSourceV2(validatedResultFixture()),
    ).toMatchObject({ ready: false });
  });

  it("labels the causal module as assumption-dependent before any configuration", () => {
    const html = renderToStaticMarkup(
      <NativeMultiModLabsWorkspace
        model={modelFixture()}
        caseCount={155}
        groupingColumns={groupingColumns}
        initialTab="causal"
      />,
    );
    expect(html).toContain("assumption-dependent interventional estimate");
    expect(html).toContain("QuickPLS never says causality is established");
    expect(html).toContain("Explicit observed-data equations");
    expect(html).toContain(
      "QuickPLS will not infer an indirect path or hide its equation terms",
    );
    expect(html).toContain("Required nonempty adjustment set");
    expect(html).toContain(
      "interventional_causal_mediation_v1.adjustment_set_missing",
    );
    expect(html).toMatch(
      /<button type="button" disabled="">.*Stage Recipe V4<\/button>/u,
    );
  });

  it("renders results only when a strict validated attachment is supplied", () => {
    const html = renderToStaticMarkup(
      <NativeMultiModLabsWorkspace
        model={modelFixture()}
        caseCount={155}
        groupingColumns={groupingColumns}
        validatedResult={validatedResultFixture()}
      />,
    );
    expect(html).toContain('data-multimod-results="v1"');
    expect(html).toContain("PLS multigroup analysis");
    expect(html).toContain("Result-level gates passed.");
  });
});
