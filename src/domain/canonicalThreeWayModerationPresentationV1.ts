import {
  capabilityCellReferenceIdentityV2,
  type CanonicalResultCell,
  type CanonicalResultChart,
  type CanonicalResultColumn,
  type CanonicalResultDocumentV2,
  type CanonicalResultSection,
  type CanonicalResultTable,
  type CapabilityCellReferenceV2,
} from "./canonicalResultDocumentV2";
import type {
  CanonicalGeneralSemEstimateV1,
  CanonicalGeneralSemResultTraceV1,
  CanonicalThreeWayModeratorProbeKindV1,
  CanonicalThreeWayModerationBootstrapReceiptV1,
} from "./canonicalGeneralSemResultsV1";

export const CANONICAL_THREE_WAY_INTERACTION_TABLE_ID_V1 = "general_sem_three_way_effect" as const;
export const CANONICAL_THREE_WAY_CONDITIONAL_TABLE_ID_V1 = "general_sem_three_way_conditional_effects" as const;
export const CANONICAL_THREE_WAY_SIMPLE_SLOPE_TABLE_ID_V1 = "general_sem_three_way_simple_slopes" as const;
export const CANONICAL_THREE_WAY_SIMPLE_SLOPE_CHART_ID_V1 = "general_sem_three_way_simple_slope_chart" as const;
export const CANONICAL_THREE_WAY_BOOTSTRAP_RECEIPT_TABLE_ID_V1 = "general_sem_three_way_bootstrap_receipt" as const;
export const CANONICAL_THREE_WAY_SECTION_ID_V1 = "general_sem_three_way_moderation" as const;

function textCell(value: string): CanonicalResultCell {
  return { kind: "text", value };
}

function numberCell(value: number): CanonicalResultCell {
  return { kind: "number", value };
}

function optionalNumberCell(value: number | null | undefined): CanonicalResultCell {
  return value == null
    ? { kind: "missing", reason: "not_estimated" }
    : numberCell(value);
}

function probeKindLabel(kind: CanonicalThreeWayModeratorProbeKindV1): string {
  return kind === "binary_zero_one" ? "Binary 0/1" : "Continuous standardized";
}

function probeValueLabel(kind: CanonicalThreeWayModeratorProbeKindV1, value: number): string {
  if (kind === "binary_zero_one") return Object.is(value, -0) ? "0" : String(value);
  if (value === -1) return "−1 SD";
  if (Object.is(value, -0) || value === 0) return "Mean";
  if (value === 1) return "+1 SD";
  return String(value);
}

function probeAssignmentLabel(
  moderatorId: string,
  kind: CanonicalThreeWayModeratorProbeKindV1,
  value: number,
): string {
  const label = probeValueLabel(kind, value);
  return kind === "binary_zero_one"
    ? `${moderatorId} (binary) = ${label}`
    : `${moderatorId} = ${label}`;
}

function column(
  id: string,
  label: string,
  dataType: CanonicalResultColumn["data_type"],
  description: string,
  role: CanonicalResultColumn["role"],
  defaultPrecision?: number,
): CanonicalResultColumn {
  return {
    id,
    label,
    data_type: dataType,
    description,
    role,
    ...(defaultPrecision == null ? {} : { default_precision: defaultPrecision }),
  };
}

const ESTIMATE_COLUMNS: CanonicalResultColumn[] = [
  column("estimate", "Estimate", "number", "Original-sample estimate.", "estimate", 6),
  column("bootstrap_mean", "Bootstrap mean", "number", "Mean estimate across usable bootstrap samples.", "uncertainty", 6),
  column("standard_error", "Standard error", "number", "Sample standard deviation across usable bootstrap estimates.", "uncertainty", 6),
  column("ci_lower", "CI lower", "number", "Lower endpoint of the requested percentile confidence interval.", "uncertainty", 6),
  column("ci_upper", "CI upper", "number", "Upper endpoint of the requested percentile confidence interval.", "uncertainty", 6),
  column("p_value", "p value", "number", "Deterministic two-sided null-centered bootstrap p value.", "decision", 6),
];

function estimateCells(estimate: CanonicalGeneralSemEstimateV1): CanonicalResultCell[] {
  return [
    numberCell(estimate.estimate),
    optionalNumberCell(estimate.bootstrap_mean),
    optionalNumberCell(estimate.standard_error),
    optionalNumberCell(estimate.lower),
    optionalNumberCell(estimate.upper),
    optionalNumberCell(estimate.p_value),
  ];
}

function sortedCapabilityCells(
  values: readonly (CapabilityCellReferenceV2 | null | undefined)[],
): CapabilityCellReferenceV2[] {
  return [...new Map(values
    .filter((value): value is CapabilityCellReferenceV2 => value != null)
    .map((value) => [capabilityCellReferenceIdentityV2(value), value])).entries()]
    .sort(([left], [right]) => left < right ? -1 : left > right ? 1 : 0)
    .map(([, value]) => value);
}

function tableCapabilities(
  document: CanonicalResultDocumentV2,
  traces: readonly CanonicalGeneralSemResultTraceV1[],
  receipt?: CanonicalThreeWayModerationBootstrapReceiptV1 | null,
): CapabilityCellReferenceV2[] | undefined {
  if (!document.capability_cells) return undefined;
  const capabilities = sortedCapabilityCells([
    ...traces.map((trace) => trace.capability_cell),
    receipt?.capability_cell,
  ]);
  return capabilities.length ? capabilities : [document.provenance.capability_cell];
}

function interactionTable(document: CanonicalResultDocumentV2): CanonicalResultTable | null {
  const rows = document.general_sem_results?.three_way_interaction_effects ?? [];
  if (!rows.length) return null;
  return {
    id: CANONICAL_THREE_WAY_INTERACTION_TABLE_ID_V1,
    title: "Three-way interaction effect",
    description: "Joint strong-hierarchy estimate for the authored predictor and two moderators.",
    columns: [
      column("predictor", "Predictor", "text", "First authored interaction operand.", "label"),
      column("first_moderator", "Moderator 1", "text", "First moderator in authored order.", "label"),
      column("second_moderator", "Moderator 2", "text", "Second moderator in authored order.", "label"),
      column("outcome", "Outcome", "text", "Outcome of the moderated focal relationship.", "label"),
      ...ESTIMATE_COLUMNS,
    ],
    rows: rows.map((row) => ({
      id: row.effect_id,
      cells: [
        textCell(row.operand_ids[0]),
        textCell(row.operand_ids[1]),
        textCell(row.operand_ids[2]),
        textCell(row.outcome_id),
        ...estimateCells(row.scientific_rescaled_delta),
      ],
    })),
    footnote_ids: [],
    capability_cells: tableCapabilities(document, rows.map((row) => row.trace)),
  };
}

function conditionalInteractionTable(document: CanonicalResultDocumentV2): CanonicalResultTable | null {
  const rows = document.general_sem_results?.three_way_conditional_interaction_effects ?? [];
  if (!rows.length) return null;
  return {
    id: CANONICAL_THREE_WAY_CONDITIONAL_TABLE_ID_V1,
    title: "Conditional two-way interaction effects",
    description: "Conditional X × W interaction effects across fixed probes of the second moderator.",
    columns: [
      column("first_moderator", "Moderator 1", "text", "Moderator in the conditional two-way interaction.", "label"),
      column("second_moderator", "Moderator 2", "text", "Moderator defining the probe values.", "label"),
      column("probe_kind", "Probe type", "text", "Continuous standardized probes or actual binary 0/1 categories.", "diagnostic"),
      column("second_moderator_value", "Moderator 2 value", "number", "Fixed standardized probe or actual binary category.", "label", 4),
      ...ESTIMATE_COLUMNS,
    ],
    rows: [...rows]
      .sort((left, right) => left.second_moderator_probe_index - right.second_moderator_probe_index)
      .map((row) => ({
        id: row.effect_id,
        cells: [
          textCell(row.first_moderator_id),
          textCell(row.second_moderator_id),
          textCell(probeKindLabel(row.second_moderator_probe_kind)),
          numberCell(row.second_moderator_value),
          ...estimateCells(row.value),
        ],
      })),
    footnote_ids: [],
    capability_cells: tableCapabilities(document, rows.map((row) => row.trace)),
  };
}

function simpleSlopeTable(document: CanonicalResultDocumentV2): CanonicalResultTable | null {
  const rows = document.general_sem_results?.three_way_simple_slopes ?? [];
  if (!rows.length) return null;
  const predictorId = document.general_sem_results?.three_way_interaction_effects?.[0]?.operand_ids[0] ?? "Predictor";
  return {
    id: CANONICAL_THREE_WAY_SIMPLE_SLOPE_TABLE_ID_V1,
    title: "Simple slopes",
    description: "Simple slope of the focal predictor across the two-moderator probe grid.",
    columns: [
      column("predictor", "Predictor", "text", "Focal predictor whose simple slope is reported.", "label"),
      column("first_moderator", "Moderator 1", "text", "First moderator.", "label"),
      column("first_probe_kind", "Moderator 1 probe type", "text", "Continuous standardized probes or actual binary 0/1 categories.", "diagnostic"),
      column("first_moderator_value", "Moderator 1 value", "number", "Fixed standardized probe or actual binary category.", "label", 4),
      column("second_moderator", "Moderator 2", "text", "Second moderator.", "label"),
      column("second_probe_kind", "Moderator 2 probe type", "text", "Continuous standardized probes or actual binary 0/1 categories.", "diagnostic"),
      column("second_moderator_value", "Moderator 2 value", "number", "Fixed standardized probe or actual binary category.", "label", 4),
      ...ESTIMATE_COLUMNS,
    ],
    rows: [...rows]
      .sort((left, right) => (
        left.second_probe_index - right.second_probe_index
        || left.first_probe_index - right.first_probe_index
      ))
      .map((row) => ({
        id: row.effect_id,
        cells: [
          textCell(predictorId),
          textCell(row.first_moderator_id),
          textCell(probeKindLabel(row.first_moderator_probe_kind)),
          numberCell(row.first_moderator_value),
          textCell(row.second_moderator_id),
          textCell(probeKindLabel(row.second_moderator_probe_kind)),
          numberCell(row.second_moderator_value),
          ...estimateCells(row.value),
        ],
      })),
    footnote_ids: [],
    capability_cells: tableCapabilities(document, rows.map((row) => row.trace)),
  };
}

function receiptTable(document: CanonicalResultDocumentV2): CanonicalResultTable | null {
  const receipt = document.general_sem_results?.three_way_moderation_bootstrap_receipt;
  if (!receipt) return null;
  return {
    id: CANONICAL_THREE_WAY_BOOTSTRAP_RECEIPT_TABLE_ID_V1,
    title: "Three-way moderation bootstrap run details",
    description: "Shared full-model case-bootstrap traversal used by every three-way target.",
    columns: [
      column("field", "Field", "text", "Bootstrap setting or execution receipt field.", "label"),
      column("value", "Value", "text", "Recorded value.", "provenance"),
    ],
    rows: [
      ["method", "Method version", receipt.method_version],
      ["requested", "Requested replicates", String(receipt.resamples_requested)],
      ["usable", "Usable replicates", String(receipt.resamples_usable)],
      ["confidence", "Confidence level", String(receipt.confidence_level)],
      ["seed", "Seed", receipt.seed],
      ["workers", "Workers", String(receipt.workers)],
      ["shared_ledger", "One shared replicate ledger", receipt.all_three_way_targets_share_one_replicate_ledger ? "Yes" : "No"],
      ["complete_refit", "Complete model refit per replicate", receipt.complete_model_reestimated_per_replicate ? "Yes" : "No"],
      ["stage_one_refit", "Shared stage one refit per replicate", receipt.shared_stage_one_reestimated_per_replicate ? "Yes" : "No"],
      ["score_sign_alignment", "Score signs aligned before products", receipt.score_vectors_sign_aligned_before_products ? "Yes" : "No"],
      ["products_recomputed", "All hierarchy products recomputed", receipt.all_lower_order_and_three_way_products_recomputed_per_replicate ? "Yes" : "No"],
      ["joint_stage_refit", "Joint stage two refit per replicate", receipt.joint_stage_two_reestimated_per_replicate ? "Yes" : "No"],
      ["point_contract", "Joint point contract validated per replicate", receipt.complete_joint_point_contract_validated_per_replicate ? "Yes" : "No"],
    ].map(([id, label, value]) => ({
      id,
      cells: [textCell(label), textCell(value)],
    })),
    footnote_ids: [],
    capability_cells: tableCapabilities(document, [], receipt),
  };
}

function simpleSlopeChart(document: CanonicalResultDocumentV2): CanonicalResultChart | null {
  const rows = document.general_sem_results?.three_way_simple_slopes ?? [];
  if (!rows.length) return null;
  const firstModerator = rows[0].first_moderator_id;
  const secondModerator = rows[0].second_moderator_id;
  const grouped = new Map<number, typeof rows>();
  for (const row of rows) {
    const group = grouped.get(row.second_probe_index) ?? [];
    grouped.set(row.second_probe_index, [...group, row]);
  }
  return {
    id: CANONICAL_THREE_WAY_SIMPLE_SLOPE_CHART_ID_V1,
    title: "Simple slopes across moderator probes",
    description: `Two-dimensional plot of the focal predictor's simple slope by ${firstModerator}; each line is one fixed value of ${secondModerator}. Exact values and intervals remain available in the adjacent table.`,
    kind: "line",
    series: [...grouped.entries()]
      .sort(([left], [right]) => left - right)
      .map(([probeIndex, group]) => ({
        id: `second_moderator_probe_${probeIndex}`,
        label: probeAssignmentLabel(
          secondModerator,
          group[0].second_moderator_probe_kind,
          group[0].second_moderator_value,
        ),
        group: secondModerator,
        points: [...group]
          .sort((left, right) => left.first_probe_index - right.first_probe_index)
          .map((row) => ({
            x: probeValueLabel(row.first_moderator_probe_kind, row.first_moderator_value),
            y: row.value.estimate,
            lower: row.value.lower,
            upper: row.value.upper,
            label: `${probeAssignmentLabel(firstModerator, row.first_moderator_probe_kind, row.first_moderator_value)}; ${probeAssignmentLabel(secondModerator, row.second_moderator_probe_kind, row.second_moderator_value)}`,
          })),
      })),
    source_table_id: CANONICAL_THREE_WAY_SIMPLE_SLOPE_TABLE_ID_V1,
    display: {
      show_legend: true,
      show_values: false,
      x_axis_label: firstModerator,
      y_axis_label: "Simple slope of focal predictor",
    },
  };
}

/**
 * Adds researcher-facing resources derived from the strict typed three-way
 * payload. This is an immutable presentation/export projection; the archived
 * scientific document and its identities remain untouched.
 */
export function canonicalThreeWayModerationPresentationV1(
  document: CanonicalResultDocumentV2,
): CanonicalResultDocumentV2 {
  const results = document.general_sem_results;
  if (!results?.three_way_interaction_effects?.length) return document;

  const tableCandidates = [
    interactionTable(document),
    conditionalInteractionTable(document),
    simpleSlopeTable(document),
    receiptTable(document),
  ].filter((table): table is CanonicalResultTable => table != null);
  const existingTableIds = new Set(document.tables.map((table) => table.id));
  const addedTables = tableCandidates.filter((table) => !existingTableIds.has(table.id));
  const chartCandidate = simpleSlopeChart(document);
  const existingChart = chartCandidate
    ? document.charts.find((chart) => chart.id === chartCandidate.id)
    : null;
  const chartPresentationChanged = Boolean(chartCandidate
    && (!existingChart || JSON.stringify(existingChart) !== JSON.stringify(chartCandidate)));
  const projectedCharts = !chartCandidate || !chartPresentationChanged
    ? document.charts
    : existingChart
      ? document.charts.map((chart) => chart.id === chartCandidate.id ? chartCandidate : chart)
      : [...document.charts, chartCandidate];
  const existingSection = document.sections.find((section) => section.id === CANONICAL_THREE_WAY_SECTION_ID_V1);
  const sectionAlreadyComplete = Boolean(existingSection
    && tableCandidates.every((table) => existingSection.table_ids.includes(table.id))
    && (!chartCandidate || existingSection.chart_ids.includes(chartCandidate.id)));
  if (!addedTables.length && !chartPresentationChanged && sectionAlreadyComplete) return document;

  const relevantTableIds = tableCandidates
    .map((table) => table.id)
    .filter((id) => existingTableIds.has(id) || addedTables.some((table) => table.id === id));
  const relevantChartIds = [CANONICAL_THREE_WAY_SIMPLE_SLOPE_CHART_ID_V1]
    .filter((id) => projectedCharts.some((chart) => chart.id === id));
  const sectionCapabilities = tableCapabilities(
    document,
    [
      ...(results.three_way_interaction_effects ?? []).map((row) => row.trace),
      ...(results.three_way_conditional_interaction_effects ?? []).map((row) => row.trace),
      ...(results.three_way_simple_slopes ?? []).map((row) => row.trace),
    ],
    results.three_way_moderation_bootstrap_receipt,
  );
  const projectedSection: CanonicalResultSection = existingSection
    ? {
      ...existingSection,
      table_ids: [...new Set([...existingSection.table_ids, ...relevantTableIds])],
      chart_ids: [...new Set([...existingSection.chart_ids, ...relevantChartIds])],
      capability_cells: sectionCapabilities ?? existingSection.capability_cells,
    }
    : {
      id: CANONICAL_THREE_WAY_SECTION_ID_V1,
      title: "Three-Way Moderation",
      description: "Three-way interaction, conditional interaction effects, and simple slopes.",
      table_ids: relevantTableIds,
      chart_ids: relevantChartIds,
      capability_cells: sectionCapabilities,
    };

  return {
    ...document,
    sections: existingSection
      ? document.sections.map((section) => section.id === projectedSection.id ? projectedSection : section)
      : [...document.sections, projectedSection],
    tables: [...document.tables, ...addedTables],
    charts: projectedCharts,
  };
}
