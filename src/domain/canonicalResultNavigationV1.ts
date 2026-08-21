import type {
  CanonicalResultChart,
  CanonicalResultDocumentV2,
  CanonicalResultSection,
  CanonicalResultTable,
} from "./canonicalResultDocumentV2";
import type { ResultOverlaySelectionV1 } from "./moderationDiagramProjectionV1";

export type CanonicalResultNavigationGroupIdV1 =
  | "overview"
  | "measurement_model"
  | "structural_model"
  | "effects"
  | "moderation"
  | "three_way_moderation"
  | "higher_order"
  | "moderated_mediation"
  | "cbsem_parameters"
  | "cbsem_fit_identification"
  | "bootstrap"
  | "diagnostics";

export type CanonicalResultNavigationItemKindV1 =
  | "overview"
  | "table"
  | "chart"
  | "diagnostics";

export interface CanonicalResultNavigationItemV1 {
  id: string;
  kind: CanonicalResultNavigationItemKindV1;
  title: string;
  description: string | null;
  sectionId: string | null;
  resourceId: string | null;
  searchText: string;
}

export interface CanonicalResultNavigationGroupV1 {
  id: CanonicalResultNavigationGroupIdV1;
  title: string;
  items: CanonicalResultNavigationItemV1[];
}

export interface CanonicalResultNavigationV1 {
  documentId: string;
  defaultItemId: string;
  groups: CanonicalResultNavigationGroupV1[];
}

const GROUP_DEFINITIONS: ReadonlyArray<{
  id: CanonicalResultNavigationGroupIdV1;
  title: string;
}> = [
  { id: "overview", title: "Overview" },
  { id: "measurement_model", title: "Measurement Model" },
  { id: "structural_model", title: "Structural Model" },
  { id: "effects", title: "Direct, Indirect and Total Effects" },
  { id: "moderation", title: "Moderation and Conditional Effects" },
  { id: "three_way_moderation", title: "Three-Way Moderation" },
  { id: "higher_order", title: "Higher-Order Constructs" },
  { id: "moderated_mediation", title: "Moderated Mediation" },
  { id: "cbsem_parameters", title: "CB-SEM Parameters" },
  { id: "cbsem_fit_identification", title: "CB-SEM Fit and Identification" },
  { id: "bootstrap", title: "Bootstrap Inference" },
  { id: "diagnostics", title: "Diagnostics and Run Details" },
];

const TABLE_CATEGORY_BY_ID: Readonly<Record<string, CanonicalResultNavigationGroupIdV1>> = {
  outer_model: "measurement_model",
  structural_paths: "structural_model",
  r_squared: "structural_model",
  effects: "effects",
  general_sem_specific_indirect_effects: "effects",
  general_sem_aggregate_effects: "effects",
  general_sem_joint_stage_structural_coefficients: "structural_model",
  general_sem_interaction_effects: "moderation",
  general_sem_conditional_slopes: "moderation",
  general_sem_interaction_plots: "moderation",
  general_sem_moderation_gamma_inference: "moderation",
  general_sem_three_way_effect: "three_way_moderation",
  general_sem_three_way_interaction_effects: "three_way_moderation",
  general_sem_three_way_conditional_effects: "three_way_moderation",
  general_sem_three_way_conditional_interaction_effects: "three_way_moderation",
  general_sem_three_way_simple_slopes: "three_way_moderation",
  general_sem_three_way_simple_slope_chart: "three_way_moderation",
  general_sem_moderated_mediation_gamma_inference: "moderated_mediation",
  general_sem_conditional_indirect_effects: "moderated_mediation",
  general_sem_moderated_mediation_indices: "moderated_mediation",
  general_sem_higher_order_stages: "diagnostics",
  general_sem_higher_order_targets: "higher_order",
  cbsem_general_sem_parameters: "cbsem_parameters",
  cbsem_general_sem_fit: "cbsem_fit_identification",
  cbsem_general_sem_identification: "cbsem_fit_identification",
  general_sem_bootstrap_receipt: "bootstrap",
  general_sem_moderation_bootstrap_receipt: "bootstrap",
  general_sem_three_way_bootstrap_receipt: "bootstrap",
  general_sem_three_way_moderation_bootstrap_receipt: "bootstrap",
  general_sem_moderated_mediation_bootstrap_receipt: "bootstrap",
  general_sem_higher_order_bootstrap_receipt: "diagnostics",
  cbsem_recursive_sem_bootstrap_inference: "bootstrap",
  cbsem_recursive_sem_bootstrap_receipt: "bootstrap",
  cbsem_recursive_sem_bootstrap_failures: "bootstrap",
};

const RESEARCHER_FACING_TABLE_TITLE_BY_ID: Readonly<Record<string, string>> = {
  general_sem_higher_order_targets: "Component and structural estimates",
  general_sem_higher_order_stages: "Estimation stages",
  general_sem_higher_order_bootstrap_receipt: "Bootstrap run details",
};

function searchableToken(...values: Array<string | null | undefined>): string {
  return values.filter(Boolean).join(" ").toLowerCase();
}

function firstMatchingCategory(
  token: string,
  resourceId: string,
): CanonicalResultNavigationGroupIdV1 {
  const exact = TABLE_CATEGORY_BY_ID[resourceId];
  if (exact) return exact;
  if (/bootstrap|resampl|replicate|percentile|bca|studentized/.test(token)) return "bootstrap";
  if (/moderated.?mediation|conditional indirect|index of moderated/.test(token)) return "moderated_mediation";
  if (/three.?way|third.?order|x.?w.?z/.test(token)) return "three_way_moderation";
  if (/higher.?order|\bhoc\b|component loading|component weight/.test(token)) return "higher_order";
  if (/interaction|moderation|conditional slope|simple slope/.test(token)) return "moderation";
  if (/indirect effect|total effect|aggregate effect|mediation effect|specific path/.test(token)) return "effects";
  if (/identification|model fit|fit and residual|rmsea|srmr|\bcfi\b|\btli\b/.test(token)) {
    return "cbsem_fit_identification";
  }
  if (/cb.?sem.*parameter|model parameter|standardized parameter|parameter table/.test(token)) {
    return "cbsem_parameters";
  }
  if (/measurement model|outer model|loading|indicator weight|construct score/.test(token)) {
    return "measurement_model";
  }
  if (/structural model|structural path|path coefficient|control effect|r.?squared|inner model/.test(token)) {
    return "structural_model";
  }
  if (/run detail|diagnostic|convergence|execution|receipt|failure|warning|exclusion/.test(token)) {
    return "diagnostics";
  }
  return "overview";
}

function sectionsByResource(document: CanonicalResultDocumentV2): {
  table: Map<string, CanonicalResultSection>;
  chart: Map<string, CanonicalResultSection>;
} {
  const table = new Map<string, CanonicalResultSection>();
  const chart = new Map<string, CanonicalResultSection>();
  for (const section of document.sections) {
    for (const tableId of section.table_ids) if (!table.has(tableId)) table.set(tableId, section);
    for (const chartId of section.chart_ids) if (!chart.has(chartId)) chart.set(chartId, section);
  }
  return { table, chart };
}

function tableItem(
  table: CanonicalResultTable,
  section: CanonicalResultSection | undefined,
): { category: CanonicalResultNavigationGroupIdV1; item: CanonicalResultNavigationItemV1 } {
  const navigationTitle = RESEARCHER_FACING_TABLE_TITLE_BY_ID[table.id] ?? table.title;
  const token = searchableToken(
    table.id,
    table.title,
    navigationTitle,
    table.description,
    section?.id,
    section?.title,
    section?.description,
    ...table.columns.flatMap((column) => [column.label, column.description]),
  );
  return {
    category: firstMatchingCategory(token, table.id),
    item: {
      id: `canonical:table:${table.id}`,
      kind: "table",
      title: navigationTitle,
      description: table.description ?? null,
      sectionId: section?.id ?? null,
      resourceId: table.id,
      searchText: token,
    },
  };
}

function chartItem(
  chart: CanonicalResultChart,
  section: CanonicalResultSection | undefined,
): { category: CanonicalResultNavigationGroupIdV1; item: CanonicalResultNavigationItemV1 } {
  const token = searchableToken(
    chart.id,
    chart.title,
    chart.description,
    section?.id,
    section?.title,
    section?.description,
    ...chart.series.map((series) => series.label),
  );
  return {
    category: firstMatchingCategory(token, chart.id),
    item: {
      id: `canonical:chart:${chart.id}`,
      kind: "chart",
      title: chart.title,
      description: chart.description,
      sectionId: section?.id ?? null,
      resourceId: chart.id,
      searchText: token,
    },
  };
}

export function buildCanonicalResultNavigationV1(
  document: CanonicalResultDocumentV2,
): CanonicalResultNavigationV1 {
  const items = new Map<CanonicalResultNavigationGroupIdV1, CanonicalResultNavigationItemV1[]>(
    GROUP_DEFINITIONS.map((group) => [group.id, []]),
  );
  items.get("overview")!.push({
    id: "canonical:overview",
    kind: "overview",
    title: "Complete verified result",
    description: "All tables and charts in this saved result.",
    sectionId: null,
    resourceId: null,
    searchText: searchableToken(document.title, document.provenance.method_version, "complete verified result"),
  });

  const resources = sectionsByResource(document);
  for (const table of document.tables) {
    const projected = tableItem(table, resources.table.get(table.id));
    items.get(projected.category)!.push(projected.item);
  }
  for (const chart of document.charts) {
    const projected = chartItem(chart, resources.chart.get(chart.id));
    items.get(projected.category)!.push(projected.item);
  }

  if (document.notices.length || document.exclusions.length || document.footnotes.length) {
    items.get("diagnostics")!.push({
      id: "canonical:diagnostics",
      kind: "diagnostics",
      title: "Notices, boundaries and notes",
      description: "Method notices, explicit exclusions and result footnotes.",
      sectionId: null,
      resourceId: null,
      searchText: searchableToken(
        "notices boundaries notes diagnostics exclusions footnotes",
        ...document.notices.flatMap((notice) => [notice.code, notice.message]),
        ...document.exclusions.flatMap((exclusion) => [exclusion.title, exclusion.reason]),
        ...document.footnotes.map((footnote) => footnote.text),
      ),
    });
  }

  const groups = GROUP_DEFINITIONS.flatMap((definition) => {
    const groupItems = items.get(definition.id) ?? [];
    return groupItems.length ? [{ ...definition, items: groupItems }] : [];
  });
  const defaultTableItem = document.presentation.default_table_id
    ? groups.flatMap((group) => group.items).find((item) => (
      item.kind === "table" && item.resourceId === document.presentation.default_table_id
    ))
    : null;
  const defaultSectionItem = document.presentation.default_section_id
    ? groups.flatMap((group) => group.items).find((item) => (
      item.sectionId === document.presentation.default_section_id
    ))
    : null;

  return {
    documentId: document.document_id,
    defaultItemId: defaultTableItem?.id ?? defaultSectionItem?.id ?? "canonical:overview",
    groups,
  };
}

export function filterCanonicalResultNavigationV1(
  navigation: CanonicalResultNavigationV1,
  query: string,
): CanonicalResultNavigationV1 {
  const token = query.trim().toLowerCase();
  if (!token) return navigation;
  return {
    ...navigation,
    groups: navigation.groups.flatMap((group) => {
      const groupMatches = group.title.toLowerCase().includes(token);
      const items = groupMatches
        ? group.items
        : group.items.filter((item) => item.searchText.includes(token));
      return items.length ? [{ ...group, items }] : [];
    }),
  };
}

export function canonicalResultNavigationItemV1(
  navigation: CanonicalResultNavigationV1,
  itemId: string | null | undefined,
): CanonicalResultNavigationItemV1 {
  const allItems = navigation.groups.flatMap((group) => group.items);
  return allItems.find((item) => item.id === itemId)
    ?? allItems.find((item) => item.id === navigation.defaultItemId)
    ?? allItems[0]!;
}

export function canonicalResultDocumentForItemV1(
  document: CanonicalResultDocumentV2,
  item: CanonicalResultNavigationItemV1,
): CanonicalResultDocumentV2 | null {
  if (item.kind === "overview") return document;
  if (item.kind === "diagnostics" || !item.resourceId) return null;

  const selectedTableIds = new Set<string>();
  const selectedChartIds = new Set<string>();
  if (item.kind === "table") selectedTableIds.add(item.resourceId);
  if (item.kind === "chart") {
    selectedChartIds.add(item.resourceId);
    const chart = document.charts.find((candidate) => candidate.id === item.resourceId);
    if (chart?.source_table_id) selectedTableIds.add(chart.source_table_id);
  }
  const tables = document.tables.filter((table) => selectedTableIds.has(table.id));
  const charts = document.charts.filter((chart) => selectedChartIds.has(chart.id));
  const sourceSection = document.sections.find((section) => section.id === item.sectionId)
    ?? document.sections.find((section) => (
      section.table_ids.some((id) => selectedTableIds.has(id))
      || section.chart_ids.some((id) => selectedChartIds.has(id))
    ));
  const fallbackCells = tables.flatMap((table) => table.capability_cells ?? []);
  const section: CanonicalResultSection = sourceSection
    ? {
      ...sourceSection,
      table_ids: sourceSection.table_ids.filter((id) => selectedTableIds.has(id)),
      chart_ids: sourceSection.chart_ids.filter((id) => selectedChartIds.has(id)),
    }
    : {
      id: `selected_${item.kind}`,
      title: item.title,
      description: item.description,
      table_ids: [...selectedTableIds],
      chart_ids: [...selectedChartIds],
      capability_cells: document.capability_cells
        ? (fallbackCells.length ? fallbackCells : [document.provenance.capability_cell])
        : undefined,
    };
  const footnoteIds = new Set(tables.flatMap((table) => table.footnote_ids));
  const notices = document.notices.filter((notice) => (
    notice.section_ids.includes(section.id)
    || notice.table_ids.some((id) => selectedTableIds.has(id))
  ));

  return {
    ...document,
    sections: [section],
    tables,
    charts,
    notices,
    exclusions: [],
    footnotes: document.footnotes.filter((footnote) => footnoteIds.has(footnote.id)),
    presentation: {
      ...document.presentation,
      default_section_id: section.id,
      default_table_id: tables[0]?.id ?? null,
    },
  };
}

function distinctOverlayIdsV1(values: readonly (string | null | undefined)[]): string[] {
  return [...new Set(values.filter((value): value is string => Boolean(value)))];
}

/**
 * Presentation-only diagram focus for one canonical Results selection. The
 * returned IDs are existing scientific identities; no overlay object is ever
 * persisted into SemModelV4.
 */
export function canonicalResultOverlaySelectionV1(
  document: CanonicalResultDocumentV2,
  item: CanonicalResultNavigationItemV1 | null | undefined,
): ResultOverlaySelectionV1 | null {
  if (!item || item.kind === "overview" || item.kind === "diagnostics") return null;
  const results = document.general_sem_results;
  if (!results) return null;
  const resourceId = item.resourceId ?? "";

  if (/three_way|three-way/u.test(resourceId)) {
    const effects = results.three_way_interaction_effects ?? [];
    if (!effects.length) return null;
    const parentInteractions = effects.flatMap((effect) => (
      (results.interaction_effects ?? []).filter((candidate) => (
        candidate.focal_relation_id === effect.focal_relation_id
        && candidate.focal_predictor_id === effect.operand_ids[0]
        && candidate.moderator_id === effect.operand_ids[1]
      ))
    ));
    return {
      kind: "three_way_moderation",
      nodeIds: distinctOverlayIdsV1(effects.flatMap((effect) => [...effect.operand_ids, effect.outcome_id])),
      relationIds: distinctOverlayIdsV1([
        ...effects.flatMap((effect) => [
          effect.focal_relation_id,
          effect.interaction_effect_relation_id,
        ]),
        ...parentInteractions.map((effect) => effect.interaction_effect_relation_id),
      ]),
      interactionTermIds: distinctOverlayIdsV1([
        ...effects.map((effect) => effect.interaction_id),
        ...parentInteractions.map((effect) => effect.interaction_id),
      ]),
      label: effects.length === 1 ? "Three-way moderating effect" : `${effects.length} three-way moderating effects`,
    };
  }

  if (/moderated_mediation|conditional_indirect/u.test(resourceId)) {
    const rows = results.conditional_indirect_effects ?? [];
    const indices = results.moderated_mediation_indices ?? [];
    if (!rows.length && !indices.length) return null;
    return {
      kind: "moderated_mediation",
      nodeIds: distinctOverlayIdsV1([
        ...rows.flatMap((effect) => [effect.x_id, effect.mediator_id, effect.y_id, effect.moderator_id]),
        ...indices.flatMap((effect) => [effect.x_id, effect.mediator_id, effect.y_id, effect.moderator_id]),
      ]),
      relationIds: distinctOverlayIdsV1([
        ...rows.flatMap((effect) => effect.ordered_relation_ids),
        ...indices.flatMap((effect) => effect.ordered_relation_ids),
      ]),
      interactionTermIds: distinctOverlayIdsV1([
        ...rows.map((effect) => effect.interaction_id),
        ...indices.map((effect) => effect.interaction_id),
      ]),
      label: "Moderated mediation path",
    };
  }

  if (/interaction|moderation|conditional_slopes/u.test(resourceId)) {
    const effects = results.interaction_effects ?? [];
    if (!effects.length) return null;
    return {
      kind: "moderation",
      nodeIds: distinctOverlayIdsV1(effects.flatMap((effect) => [
        effect.focal_predictor_id,
        effect.moderator_id,
        effect.outcome_id,
      ])),
      relationIds: distinctOverlayIdsV1(effects.flatMap((effect) => [
        effect.focal_relation_id,
        effect.interaction_effect_relation_id,
      ])),
      interactionTermIds: distinctOverlayIdsV1(effects.map((effect) => effect.interaction_id)),
      label: effects.length === 1 ? "Moderating effect" : `${effects.length} moderating effects`,
    };
  }

  if (/indirect|aggregate_effect|total_effect|mediation|general_sem_bootstrap_receipt/u.test(resourceId)) {
    const paths = results.specific_indirect_effects ?? [];
    if (!paths.length) return null;
    return {
      kind: "mediation",
      nodeIds: distinctOverlayIdsV1(paths.flatMap((effect) => [effect.source_id, effect.target_id])),
      relationIds: distinctOverlayIdsV1(paths.flatMap((effect) => effect.ordered_relation_ids)),
      interactionTermIds: [],
      label: paths.length === 1 ? "Indirect path" : `${paths.length} indirect paths`,
    };
  }
  return null;
}
