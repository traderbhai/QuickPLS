import type { CanonicalResultCell, CanonicalResultColumn, CanonicalResultDocumentV2, CanonicalResultTable } from "./canonicalResultDocumentV2";
import type { ResultTable } from "./resultTables";
import type { AnalysisModelSnapshot, ConstructData, InteractionData } from "../types";

export type AuthoredResultIdentityKind =
  | "construct"
  | "indicator"
  | "relation"
  | "mediation"
  | "interaction"
  | "higher_order"
  | "canonical_target";

export type AuthoredResultIdentityModel = Pick<AnalysisModelSnapshot, "nodes" | "edges">;

export interface AuthoredInteractionIdentity {
  operands?: readonly string[];
  predictor?: string;
  moderator?: string;
  moderators?: readonly string[];
  outcome?: string;
}

export interface AuthoredRelationIdentity {
  id?: string | null;
  source?: string | null;
  target?: string | null;
}

export interface AuthoredResultIdentityResolver {
  construct: (identity: string) => string;
  indicator: (identity: string, constructId?: string | null) => string;
  relation: (identity: string | AuthoredRelationIdentity) => string;
  mediation: (identity: string | readonly string[]) => string;
  interaction: (identity: string | AuthoredInteractionIdentity) => string;
  higherOrder: (identity: string, componentId?: string | null) => string;
  canonicalTarget: (identity: string) => string;
  text: (value: string) => string;
}

interface InteractionRecord {
  nodeId: string;
  termId: string;
  operands: string[];
  outcome: string;
}

interface HigherOrderRecord {
  id: string;
  components: string[];
}

const TECHNICAL_ID = /^(?:__qpls_|generated:|derived:|interaction:|relation:|effect:|target:|parameter:|term:|path:|plot:|series:|equation:)/iu;
const HASH_ID = /^(?:sha256:)?[a-f0-9]{40,}$/iu;

function hasText(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function normalized(value: string): string {
  return value.trim().normalize("NFKC").toLocaleLowerCase();
}

function sentenceStart(value: string): string {
  return value.length ? `${value[0]!.toUpperCase()}${value.slice(1)}` : value;
}

function compactIdentityToken(value: string): string {
  const trimmed = value.trim();
  const tail = trimmed.includes(":") ? trimmed.slice(trimmed.lastIndexOf(":") + 1) : trimmed;
  return tail
    .replace(/^__qpls_(?:interaction|hoc)_/u, "")
    .replace(/_by_/gu, " × ")
    .replace(/_to_/gu, " → ")
    .replace(/[_.-]+/gu, " ")
    .replace(/\s+/gu, " ")
    .trim();
}

function friendlyFallback(value: string, kind: AuthoredResultIdentityKind): string {
  const trimmed = value.trim();
  if (!trimmed) return kind === "indicator" ? "Saved indicator" : "Saved model element";
  if (HASH_ID.test(trimmed)) return kind === "canonical_target" ? "Saved result target" : "Saved model element";
  const compact = compactIdentityToken(trimmed);
  if (!compact || TECHNICAL_ID.test(trimmed)) {
    if (kind === "interaction") return "Moderating effect";
    if (kind === "higher_order") return "Higher-order construct";
    if (kind === "relation") return "Model relationship";
    if (kind === "indicator") return "Saved indicator";
    if (kind === "canonical_target") return "Saved result target";
    return "Saved construct";
  }
  return sentenceStart(compact);
}

function interactionParts(interaction: InteractionData): string[] {
  return interaction.kind === "interaction_v2"
    ? [...interaction.operands]
    : [interaction.predictor, interaction.moderator];
}

function modelInteractionRecords(model?: AuthoredResultIdentityModel | null): InteractionRecord[] {
  return (model?.nodes ?? []).flatMap((node) => {
    const interaction = node.data.semantic === "interaction" ? node.data.interaction : undefined;
    if (!interaction) return [];
    return [{
      nodeId: node.id,
      termId: interaction.termId?.trim() || node.id,
      operands: interactionParts(interaction),
      outcome: interaction.outcome,
    }];
  });
}

function modelHigherOrderRecords(model?: AuthoredResultIdentityModel | null): HigherOrderRecord[] {
  return (model?.nodes ?? []).flatMap((node) => {
    const declaration = node.data.semantic === "higher_order" ? node.data.higherOrder : undefined;
    return declaration ? [{ id: node.id, components: [...declaration.components] }] : [];
  });
}

function authoredConstructLabels(model?: AuthoredResultIdentityModel | null): Map<string, string> {
  const nodesById = new Map<string, Array<{ data: ConstructData }>>();
  for (const node of model?.nodes ?? []) {
    if (!hasText(node.id)) continue;
    nodesById.set(node.id, [...(nodesById.get(node.id) ?? []), node]);
  }

  const candidate = new Map<string, { label: string; shortName: string | null }>();
  for (const [id, nodes] of nodesById) {
    const labels = new Set(nodes.map((node) => node.data.label?.trim()).filter(hasText));
    if (labels.size !== 1) continue;
    const shortNames = new Set(nodes.map((node) => node.data.shortName?.trim()).filter(hasText));
    candidate.set(id, {
      label: [...labels][0]!,
      shortName: shortNames.size === 1 ? [...shortNames][0]! : null,
    });
  }

  const idsByDisplay = new Map<string, string[]>();
  for (const [id, entry] of candidate) {
    const key = normalized(entry.label);
    idsByDisplay.set(key, [...(idsByDisplay.get(key) ?? []), id]);
  }

  const result = new Map<string, string>();
  for (const [id, entry] of candidate) {
    const collisions = idsByDisplay.get(normalized(entry.label)) ?? [id];
    if (collisions.length === 1) {
      result.set(id, entry.label);
      continue;
    }
    const shortNameUnique = entry.shortName
      && collisions.filter((candidateId) => normalized(candidate.get(candidateId)?.shortName ?? "") === normalized(entry.shortName!)).length === 1;
    const ordinal = collisions.indexOf(id) + 1;
    result.set(id, `${entry.label} (${shortNameUnique ? entry.shortName : ordinal})`);
  }
  return result;
}

function splitPathIdentity(value: string): string[] {
  if (value.includes("→")) return value.split("→").map((part) => part.trim()).filter(Boolean);
  if (value.includes("->")) return value.split("->").map((part) => part.trim()).filter(Boolean);
  if (value.includes("_via_")) return value.split("_via_").map((part) => part.trim()).filter(Boolean);
  return [];
}

/**
 * Builds a presentation-only identity resolver from an immutable model
 * snapshot. Missing or historical metadata is converted to a readable fallback
 * instead of leaking generated implementation identities into normal Results.
 */
export function createAuthoredResultIdentityResolver(
  model?: AuthoredResultIdentityModel | null,
): AuthoredResultIdentityResolver {
  const constructLabels = authoredConstructLabels(model);
  const authoredDisplayKeys = new Set([...constructLabels.values()].map(normalized));
  const interactions = modelInteractionRecords(model);
  const higherOrders = modelHigherOrderRecords(model);
  const interactionByIdentity = new Map<string, InteractionRecord>();
  for (const interaction of interactions) {
    interactionByIdentity.set(interaction.nodeId, interaction);
    interactionByIdentity.set(interaction.termId, interaction);
  }
  const higherOrderById = new Map(higherOrders.map((record) => [record.id, record]));
  const relationByIdentity = new Map<string, { source: string; target: string }>();
  for (const edge of model?.edges ?? []) {
    const relation = { source: edge.source, target: edge.target };
    relationByIdentity.set(edge.id, relation);
    const authorityId = (edge.data as { standardSemV4Authority?: { authorityObjectId?: string } } | undefined)
      ?.standardSemV4Authority?.authorityObjectId;
    if (hasText(authorityId)) relationByIdentity.set(authorityId, relation);
  }

  const construct = (identity: string): string => {
    const id = identity.trim();
    const authored = constructLabels.get(id);
    if (authored) return authored;
    const interaction = interactionByIdentity.get(id);
    if (interaction) return interactionLabel(interaction);
    if (higherOrderById.has(id) || id.startsWith("__qpls_hoc_")) return friendlyFallback(id, "higher_order");
    if (id.startsWith("__qpls_interaction_")) return friendlyFallback(id, "interaction");
    const fallback = friendlyFallback(id, "construct");
    return authoredDisplayKeys.has(normalized(fallback))
      ? `${fallback} (unmatched saved construct)`
      : fallback;
  };

  const interactionLabel = (record: InteractionRecord): string => {
    const operands = record.operands.map(construct);
    const expression = operands.length ? operands.join(" × ") : "Moderating effect";
    return hasText(record.outcome) ? `${expression} → ${construct(record.outcome)}` : expression;
  };

  const indicator = (identity: string, constructId?: string | null): string => {
    const id = identity.trim();
    for (const higherOrder of higherOrders) {
      for (const componentId of higherOrder.components) {
        if (id === `__qpls_hoc_${higherOrder.id}_${componentId}`) {
          return `${construct(componentId)} score for ${construct(higherOrder.id)}`;
        }
      }
    }
    if (id.startsWith("__qpls_interaction_")) {
      const matching = interactions.find((record) => record.nodeId === constructId || record.termId === constructId);
      return matching ? `${interactionLabel(matching)} product indicator` : "Moderating-effect product indicator";
    }
    return friendlyFallback(id, "indicator");
  };

  const relation = (identity: string | AuthoredRelationIdentity): string => {
    const record = typeof identity === "string" ? relationByIdentity.get(identity.trim()) : undefined;
    const source = typeof identity === "string" ? record?.source : identity.source ?? undefined;
    const target = typeof identity === "string" ? record?.target : identity.target ?? undefined;
    if (hasText(source) && hasText(target)) return `${construct(source)} → ${construct(target)}`;
    const raw = typeof identity === "string" ? identity.trim() : identity.id?.trim() ?? "";
    const path = splitPathIdentity(raw.replace(/^relation:/u, ""));
    return path.length >= 2 ? path.map(construct).join(" → ") : friendlyFallback(raw, "relation");
  };

  const mediation = (identity: string | readonly string[]): string => {
    const path = Array.isArray(identity) ? [...identity] : splitPathIdentity(identity as string);
    if (path.length >= 2) return path.map(construct).join(" → ");
    return friendlyFallback(String(identity), "mediation");
  };

  const interaction = (identity: string | AuthoredInteractionIdentity): string => {
    if (typeof identity === "string") {
      const raw = identity.trim();
      const record = interactionByIdentity.get(raw)
        ?? interactionByIdentity.get(raw.replace(/^(?:interaction_effect|conditional_effect|simple_slope):/u, ""));
      if (record) return interactionLabel(record);
      const compact = raw
        .replace(/^(?:interaction_effect|conditional_effect|simple_slope|interaction):/u, "")
        .replace(/(?::\d+)+$/u, "");
      const processLike = compact.match(/^(.+?)(?:->|_to_)(.+?)@(.+?)(?:\|(.+))?$/u);
      if (processLike) {
        return interaction({
          predictor: processLike[1],
          moderators: [processLike[3], processLike[4]].filter(hasText),
          outcome: processLike[2],
        });
      }
      const operands = compact.split(/_by_|\u00d7/u).map((part) => part.trim()).filter(Boolean);
      return operands.length >= 2 ? operands.map(construct).join(" × ") : friendlyFallback(raw, "interaction");
    }
    const operands = identity.operands?.length
      ? [...identity.operands]
      : [identity.predictor, ...(identity.moderators ?? (identity.moderator ? [identity.moderator] : []))].filter(hasText);
    const expression = operands.length ? operands.map(construct).join(" × ") : "Moderating effect";
    return hasText(identity.outcome) ? `${expression} → ${construct(identity.outcome)}` : expression;
  };

  const higherOrder = (identity: string, componentId?: string | null): string => {
    const higherOrderLabel = construct(identity);
    return hasText(componentId)
      ? `${construct(componentId)} component of ${higherOrderLabel}`
      : higherOrderLabel;
  };

  const canonicalTarget = (identity: string): string => {
    const raw = identity.trim();
    if (!raw) return "Saved result target";
    if (relationByIdentity.has(raw)) return relation(raw);
    if (interactionByIdentity.has(raw)) return interaction(raw);
    const [kind, ...parts] = raw.split(":");
    const payload = parts.join(":");
    if (kind === "hoc_component") {
      const [higherOrderId, componentId] = parts.length > 1 ? parts : [parts[0], undefined];
      return componentId
        ? higherOrder(higherOrderId!, componentId)
        : `${construct(higherOrderId ?? "")} component relationship`;
    }
    if (kind === "hoc_path") return relation(payload);
    if (kind === "interaction") return interaction(raw);
    if (kind === "parameter") {
      const [parameterKind, lhs, rhs] = parts;
      if (parameterKind === "loading" && lhs && rhs) return `${indicator(rhs, lhs)} ← ${construct(lhs)}`;
      if (parameterKind === "structural_path" && lhs && rhs) return relation({ source: rhs, target: lhs });
      if (parameterKind === "latent_covariance" && lhs && rhs) return `${construct(lhs)} ↔ ${construct(rhs)}`;
      if (parameterKind === "latent_variance" && lhs) return `Variance: ${construct(lhs)}`;
      if (parameterKind === "residual_variance" && lhs) return `Residual variance: ${indicator(lhs)}`;
      if (parameterKind === "residual_covariance" && lhs && rhs) return `${indicator(lhs)} ↔ ${indicator(rhs)}`;
      return parameterKind ? sentenceStart(parameterKind.replaceAll("_", " ")) : "Saved parameter";
    }
    if (kind === "interaction_effect") return /^\d+(?::\d+)*$/u.test(payload) ? "Interaction effect" : `Interaction effect: ${interaction(payload)}`;
    if (kind === "conditional_effect") return /^\d+(?::\d+)*$/u.test(payload) ? "Conditional effect" : `Conditional effect: ${interaction(payload)}`;
    if (kind === "simple_slope") return /^\d+(?::\d+)*$/u.test(payload) ? "Simple slope" : `Simple slope: ${interaction(payload)}`;
    if (/indirect|mediation/u.test(kind)) return mediation(payload);
    if (/relation|path/u.test(kind)) return relation(payload);
    return friendlyFallback(raw, "canonical_target");
  };

  const text = (value: string): string => {
    const raw = value.trim();
    if (!raw) return raw;
    if (raw.includes(";")) return raw.split(";").map((part) => text(part)).join("; ");
    if (constructLabels.has(raw)) return construct(raw);
    if (interactionByIdentity.has(raw)) return interaction(raw);
    if (relationByIdentity.has(raw)) return relation(raw);
    if (TECHNICAL_ID.test(raw) || HASH_ID.test(raw)) return canonicalTarget(raw);
    const path = splitPathIdentity(raw);
    if (path.length >= 2) return mediation(path);
    const probe = raw.match(/^(.+?)\s*=\s*(.+)$/u);
    if (probe && (constructLabels.has(probe[1]!.trim()) || interactionByIdentity.has(probe[1]!.trim()))) {
      const rawProbe = probe[2]!.trim();
      const displayedProbe = rawProbe === "-1" ? "−1 SD" : rawProbe === "0" ? "mean" : rawProbe === "1" ? "+1 SD" : rawProbe;
      return `${construct(probe[1]!.trim())} = ${displayedProbe}`;
    }
    let displayed = raw;
    for (const record of interactions) {
      const label = interactionLabel(record);
      displayed = displayed.replaceAll(record.nodeId, label).replaceAll(record.termId, label);
    }
    for (const record of higherOrders) displayed = displayed.replaceAll(record.id, construct(record.id));
    for (const [id, label] of [...constructLabels.entries()].sort(([left], [right]) => right.length - left.length)) {
      displayed = displayed.replaceAll(id, label);
    }
    return displayed
      .replace(/\bconstruct:[A-Za-z0-9_.-]+/gu, (token) => construct(token))
      .replace(/\brelation:[A-Za-z0-9_.:-]+/gu, (token) => relation(token))
      .replace(/\binteraction:[A-Za-z0-9_.:-]+/gu, (token) => interaction(token))
      .replace(/__qpls_interaction_[A-Za-z0-9_.:-]+/gu, "Moderating effect")
      .replace(/__qpls_hoc_[A-Za-z0-9_.:-]+/gu, "Higher-order construct")
      .replace(/(?:sha256:)?[a-f0-9]{40,}/giu, "Saved result target");
  };

  return { construct, indicator, relation, mediation, interaction, higherOrder, canonicalTarget, text };
}

function nativeResultIdentityCell(
  value: string,
  column: string,
  resolver: AuthoredResultIdentityResolver,
): string {
  const heading = column.trim().toLocaleLowerCase();
  if (!value.trim()) return value;
  if (/^(?:path id|path|relationship|relation|indirect path|conditional indirect path|moderated edge)$/u.test(heading)) {
    return resolver.mediation(value.replace(/^path:/u, ""));
  }
  if (/moderation|interaction/u.test(heading)) return resolver.interaction(value);
  if (/parameter/u.test(heading)) return resolver.text(value);
  if (/indicator/u.test(heading)) return resolver.indicator(value);
  if (/construct|predictor|moderator|mediator|outcome|source|target|from|to|equation/u.test(heading)) {
    return resolver.construct(value);
  }
  if (/^effect id$/u.test(heading)) {
    if (/^indirect:/u.test(value)) return "Conditional indirect effect";
    if (/^slope:/u.test(value)) return "Conditional effect";
    if (/^index:/u.test(value)) return "Moderated-mediation index";
    return resolver.canonicalTarget(value);
  }
  if (/^plot id$/u.test(heading)) return "Conditional outcome plot";
  if (/^series id$/u.test(heading)) {
    const index = value.match(/^series:(\d+)/u)?.[1];
    return index == null ? "Conditional series" : `Conditional series ${Number(index) + 1}`;
  }
  return resolver.text(value);
}

/**
 * Creates a display-only projection of a legacy string table. Technical
 * identities remain untouched in explicitly diagnostic/Run Details tables.
 */
export function authoredResultTablePresentation(
  table: ResultTable,
  resolver: AuthoredResultIdentityResolver,
): ResultTable {
  if (/(?:run details?|diagnostic|provenance|receipt|ledger|failures?|accounting|scope|settings|successful.+refits?|delete.one|refit standard errors?|validation plan)/iu.test(`${table.id} ${table.title}`)) {
    return table;
  }
  const columns = table.columns.map((column) => column.replace(/\s+ID$/iu, ""));
  const rows = table.rows.map((row) => row.map((cell, index) => (
    nativeResultIdentityCell(cell, table.columns[index] ?? "", resolver)
  )));
  const unchanged = columns.every((column, index) => column === table.columns[index])
    && rows.every((row, rowIndex) => row.every((cell, columnIndex) => cell === table.rows[rowIndex]?.[columnIndex]));
  return unchanged ? table : { ...table, columns, rows };
}

function isDiagnosticCanonicalTable(table: CanonicalResultTable): boolean {
  return /(?:details?|scope|receipt|diagnostic|provenance|ledger|failures?|accounting)/iu.test(`${table.id} ${table.title}`);
}

function authoredColumnLabel(column: CanonicalResultColumn): string {
  const id = column.id.toLocaleLowerCase();
  if (id === "relation_id") return "Relationship";
  if (id === "effect_id") return "Effect";
  if (id === "interaction_id") return "Moderating effect";
  if (id === "parameter_id") return "Parameter";
  if (id === "source_table_id") return "Source result";
  if (id.endsWith("_id") && /construct|predictor|moderator|mediator|outcome|component|source|target/u.test(id)) {
    return column.label.replace(/\s+ID$/iu, "");
  }
  return column.label;
}

function authoredCanonicalCell(
  cell: CanonicalResultCell,
  column: CanonicalResultColumn,
  resolver: AuthoredResultIdentityResolver,
  tableTitles: ReadonlyMap<string, string>,
): CanonicalResultCell {
  if (cell.kind !== "text") return cell;
  const id = column.id.toLocaleLowerCase();
  let value = cell.value;
  if (id === "source_table_id" || id === "source_table") value = tableTitles.get(value) ?? "Saved result table";
  else if (id === "interaction_id") value = resolver.interaction(value);
  else if (/relation_id|effect_id|parameter_id|canonical_target|target_identity/u.test(id)) value = resolver.canonicalTarget(value);
  else if (/indicator/u.test(id)) value = resolver.indicator(value);
  else if (/path|mediation|indirect/u.test(id)) value = resolver.mediation(value);
  else if (/construct|predictor|moderator|mediator|outcome|component|source|target|left|right/u.test(id)) value = resolver.construct(value);
  else value = resolver.text(value);
  return value === cell.value ? cell : { ...cell, value };
}

function researcherFacingText(
  value: string,
  resolver: AuthoredResultIdentityResolver,
  sourceTableId?: string | null,
  tableTitles?: ReadonlyMap<string, string>,
): string {
  let result = value
    .replace(/resident canonical result/giu, "saved result")
    .replace(/canonical source table/giu, "source table")
    .replace(/canonical row order/giu, "saved row order");
  if (sourceTableId) {
    result = result.replaceAll(sourceTableId, tableTitles?.get(sourceTableId) ?? "saved result table");
  }
  const interaction = result.match(/^Interaction\s+([^\s]+)$/iu);
  if (interaction) return `Moderating effect: ${resolver.interaction(interaction[1]!)}`;
  return resolver.text(result);
}

/**
 * Returns an immutable researcher-facing projection. Stable table, row, chart,
 * and provenance identities remain unchanged for navigation, export, and Run
 * Details; only visible labels and descriptions are projected.
 */
export function authoredCanonicalResultPresentation(
  document: CanonicalResultDocumentV2,
  resolver: AuthoredResultIdentityResolver,
): CanonicalResultDocumentV2 {
  const tableTitles = new Map(document.tables.map((table) => [table.id, table.title]));
  const tables = document.tables.map((table) => {
    if (isDiagnosticCanonicalTable(table)) return table;
    return {
      ...table,
      description: table.description
        ? researcherFacingText(table.description, resolver, null, tableTitles)
        : table.description,
      columns: table.columns.map((column) => ({ ...column, label: authoredColumnLabel(column) })),
      rows: table.rows.map((row) => ({
        ...row,
        cells: row.cells.map((cell, index) => {
          const column = table.columns[index];
          return column ? authoredCanonicalCell(cell, column, resolver, tableTitles) : cell;
        }),
      })),
    };
  });

  const charts = document.charts.map((chart) => {
    const higherOrder = /higher[_ -]?order|hoc/iu.test(`${chart.id} ${chart.title} ${chart.source_table_id ?? ""}`);
    return {
      ...chart,
      title: researcherFacingText(chart.title, resolver, chart.source_table_id, tableTitles),
      description: researcherFacingText(chart.description, resolver, chart.source_table_id, tableTitles),
      series: chart.series.map((series) => ({
        ...series,
        label: resolver.text(series.label),
        group: series.group ? resolver.text(series.group) : series.group,
        points: series.points.map((point) => {
          const label = point.label ? resolver.text(point.label) : point.label;
          return {
            ...point,
            x: higherOrder && label ? label : typeof point.x === "string" ? resolver.text(point.x) : point.x,
            label,
          };
        }),
      })),
      display: {
        ...chart.display,
        x_axis_label: higherOrder
          ? "Higher-order relationship"
          : chart.display.x_axis_label ? resolver.text(chart.display.x_axis_label) : chart.display.x_axis_label,
        y_axis_label: chart.display.y_axis_label ? resolver.text(chart.display.y_axis_label) : chart.display.y_axis_label,
      },
    };
  });

  return {
    ...document,
    sections: document.sections.map((section) => ({
      ...section,
      title: resolver.text(section.title),
      description: section.description
        ? researcherFacingText(section.description, resolver, null, tableTitles)
        : section.description,
    })),
    tables,
    charts,
    notices: document.notices.map((notice) => ({
      ...notice,
      message: researcherFacingText(notice.message, resolver, null, tableTitles),
    })),
    exclusions: document.exclusions.map((exclusion) => ({
      ...exclusion,
      title: resolver.text(exclusion.title),
      reason: researcherFacingText(exclusion.reason, resolver, null, tableTitles),
    })),
    footnotes: document.footnotes.map((footnote) => ({
      ...footnote,
      text: researcherFacingText(footnote.text, resolver, null, tableTitles),
    })),
  };
}
