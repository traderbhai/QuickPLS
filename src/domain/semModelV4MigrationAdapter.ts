import type { Edge } from "@xyflow/react";
import type {
  Dataset,
  NativeCanonicalModelSpec,
  NativeModelPresentation,
  PathEdgeData,
} from "../types";
import {
  compileCbsemPlanV2,
  compilePlsPlanV2,
  convertLegacyBasicModelV4,
  SemModelV4OperationError,
  validateSemModelV4,
  type CompiledCbsemPlanV2,
  type CompiledPlsPlanV2,
  type LegacyBasicModelInterpretationV4,
  type LegacyDisplayCovarianceV4,
  type SemDataBindingV4,
  type SemEndpointV4,
  type SemModelV4,
  type SemParameterV4,
  type SemPresentationV4,
  type SemRelationV4,
} from "./semModelV4";

export const SEM_MODEL_V4_MIGRATION_ADAPTER_VERSION = 1 as const;

export type LegacySemMethodIntentV4 = "pls_sem" | "cbsem" | "method_neutral" | "mixed";
export type ConfirmedLegacyEstimandV4 = Exclude<LegacyBasicModelInterpretationV4, "unspecified">;

export interface SemModelV4MigrationBlocker {
  code: string;
  subject: string;
  message: string;
}

export interface CurrentQuickPlsGraphSnapshotV4 {
  model: NativeCanonicalModelSpec;
  presentation: NativeModelPresentation;
  data_binding: SemDataBindingV4;
}

export interface SemCovarianceLineageV4 {
  lineage_id: string;
  origin: "legacy_presentation" | "authored_presentation" | "authored_scientific";
  source_edge_id: string | null;
  annotation_id: string | null;
  scientific_relation_id: string | null;
  scientific_parameter_id: string | null;
  operation: "presentation_only" | "author_scientific_covariance_v1" | "convert_to_model_covariance_v1";
}

interface SemModelV4MigrationArtifactBase {
  adapter_version: typeof SEM_MODEL_V4_MIGRATION_ADAPTER_VERSION;
  source_method_intent: LegacySemMethodIntentV4;
  source_graph: CurrentQuickPlsGraphSnapshotV4;
  covariance_lineage: SemCovarianceLineageV4[];
}

export interface LegacyEstimandUnspecifiedV4 extends SemModelV4MigrationArtifactBase {
  kind: "legacy_estimand_unspecified";
  automatic_conversion_blocker: SemModelV4MigrationBlocker | null;
}

export interface ConfirmedSemModelV4Migration extends SemModelV4MigrationArtifactBase {
  kind: "sem_model_v4";
  interpretation: ConfirmedLegacyEstimandV4;
  model: SemModelV4;
}

export type SemModelV4MigrationArtifact = LegacyEstimandUnspecifiedV4 | ConfirmedSemModelV4Migration;

export interface CurrentDatasetBindingV4Input extends Pick<Dataset, "id" | "columns" | "kind" | "sampleSize" | "rowCount"> {}

export interface MigrateCurrentQuickPlsGraphV4Input {
  model: NativeCanonicalModelSpec;
  presentation?: NativeModelPresentation | null;
  data_binding: SemDataBindingV4;
  method_intent: LegacySemMethodIntentV4;
}

export interface PresentationCovarianceAuthoringV4 {
  id: string;
  left_construct: string;
  right_construct: string;
  label?: string | null;
}

export interface ScientificCovarianceAuthoringV4 {
  id: string;
  left: SemEndpointV4;
  right: SemEndpointV4;
  label?: string | null;
  start?: number | null;
  lower?: number | null;
  upper?: number | null;
  routing?: string | null;
}

export interface CurrentQuickPlsGraphRoundTripV4 extends CurrentQuickPlsGraphSnapshotV4 {
  scientific_covariances: Array<{
    relation_id: string;
    parameter_id: string;
    left: SemEndpointV4;
    right: SemEndpointV4;
  }>;
}

export class SemModelV4MigrationAdapterError extends Error {
  constructor(
    public readonly code: string,
    public readonly subject: string,
    message: string,
  ) {
    super(message);
    this.name = "SemModelV4MigrationAdapterError";
  }
}

/** Matches the stable construct namespace used by the Rust and TypeScript V4 converters. */
export const semConstructVariableIdV4 = (legacyConstructId: string) => `construct:${legacyConstructId}`;

/** Matches the stable observed-variable namespace used by the Rust and TypeScript V4 converters. */
export const semObservedVariableIdV4 = (sourceColumn: string) => `observed:${sourceColumn}`;

/**
 * Converts the current dataset descriptor into the initial V4 binding understood by
 * this adapter. Indicator column order follows the dataset declaration for matrix
 * inputs; the model remains the authority for which columns are required.
 */
export function currentDatasetToSemDataBindingV4(
  dataset: CurrentDatasetBindingV4Input,
  model: Pick<NativeCanonicalModelSpec, "constructs">,
): SemDataBindingV4 {
  const datasetId = requiredId(dataset.id, "dataset.id");
  const indicatorColumns = model.constructs.flatMap((construct) => construct.indicators);
  const indicatorSet = new Set(indicatorColumns);
  const declaredColumns = dataset.columns.map((column, index) => requiredId(column, `dataset.columns[${index}]`));
  if (new Set(declaredColumns).size !== declaredColumns.length) {
    fail("migration.data_columns_duplicate", datasetId, "Dataset columns must be unique before SEM migration.");
  }
  const missing = [...indicatorSet].filter((indicator) => !declaredColumns.includes(indicator));
  if (missing.length) {
    fail("migration.data_column_missing", missing[0], `The current dataset does not contain required indicator ${missing[0]}.`);
  }

  const kind = dataset.kind ?? "raw";
  if (kind === "raw") {
    return {
      kind: "raw",
      dataset_id: datasetId,
      missing_data: "listwise_deletion",
      weight: null,
      cluster_variable: null,
      strata_variable: null,
    };
  }

  // Matrix row count is the matrix dimension, never a substitute for study N.
  const sampleSize = dataset.sampleSize ?? null;
  if (!Number.isInteger(sampleSize) || Number(sampleSize) < 2) {
    fail("migration.matrix_sample_size_invalid", datasetId, "Matrix input requires an integer sample size of at least two.");
  }
  const variables = declaredColumns.filter((column) => indicatorSet.has(column)).map(semObservedVariableIdV4);
  if (variables.length !== indicatorSet.size) {
    fail("migration.matrix_columns_invalid", datasetId, "Matrix columns must identify each model indicator exactly once.");
  }
  return {
    kind,
    dataset_id: datasetId,
    variables,
    means: null,
    standard_deviations: null,
    sample: {
      sample_size: Number(sampleSize),
      covariance_denominator: "sample_n_minus_one",
      group_sample_sizes: {},
    },
  };
}

/**
 * Creates an immutable, non-destructive migration artifact. PLS and CB-SEM intent
 * can convert automatically; neutral or mixed intent remains explicitly pending.
 */
export function migrateCurrentQuickPlsGraphToSemModelV4(
  input: MigrateCurrentQuickPlsGraphV4Input,
): SemModelV4MigrationArtifact {
  const sourceGraph = captureAndValidateSourceGraph(input);
  const displayCovariances = displayCovariancesFromSource(sourceGraph);
  const base = {
    adapter_version: SEM_MODEL_V4_MIGRATION_ADAPTER_VERSION,
    source_method_intent: input.method_intent,
    source_graph: sourceGraph,
    covariance_lineage: initialCovarianceLineage(displayCovariances),
  } satisfies SemModelV4MigrationArtifactBase;

  const interpretation = automaticInterpretation(input.method_intent);
  if (!interpretation) {
    return deepFreeze({
      ...base,
      kind: "legacy_estimand_unspecified",
      automatic_conversion_blocker: null,
    });
  }

  try {
    return deepFreeze({
      ...base,
      kind: "sem_model_v4",
      interpretation,
      model: convertSourceGraph(sourceGraph, interpretation, displayCovariances),
    });
  } catch (error) {
    if (!(error instanceof SemModelV4OperationError) || !isAutomaticConversionBlocker(error.code)) throw error;
    return deepFreeze({
      ...base,
      kind: "legacy_estimand_unspecified",
      automatic_conversion_blocker: blockerFrom(error),
    });
  }
}

/** Applies explicit factor-versus-composite confirmation to a copied pending artifact. */
export function confirmLegacyEstimandSemModelV4(
  artifact: SemModelV4MigrationArtifact,
  interpretation: ConfirmedLegacyEstimandV4,
): ConfirmedSemModelV4Migration {
  if (artifact.kind !== "legacy_estimand_unspecified") {
    fail("migration.estimand_already_confirmed", artifact.source_graph.model.id, "This model already has explicit estimand semantics.");
  }
  const sourceGraph = serializableClone(artifact.source_graph, "source_graph");
  const displayCovariances = displayCovariancesFromSource(sourceGraph);
  const model = convertSourceGraph(sourceGraph, interpretation, displayCovariances);
  return deepFreeze({
    adapter_version: SEM_MODEL_V4_MIGRATION_ADAPTER_VERSION,
    source_method_intent: artifact.source_method_intent,
    source_graph: sourceGraph,
    covariance_lineage: serializableClone(artifact.covariance_lineage, "covariance_lineage"),
    kind: "sem_model_v4",
    interpretation,
    model,
  });
}

export function requireConfirmedSemModelV4(artifact: SemModelV4MigrationArtifact): SemModelV4 {
  if (artifact.kind !== "sem_model_v4") {
    fail(
      "migration.estimand_confirmation_required",
      artifact.source_graph.model.id,
      "Choose composite or common-factor semantics before compiling this migrated model.",
    );
  }
  return artifact.model;
}

export function compileMigratedPlsPlanV2(artifact: SemModelV4MigrationArtifact): CompiledPlsPlanV2 {
  return compilePlsPlanV2(requireConfirmedSemModelV4(artifact));
}

export function compileMigratedCbsemPlanV2(artifact: SemModelV4MigrationArtifact): CompiledCbsemPlanV2 {
  return compileCbsemPlanV2(requireConfirmedSemModelV4(artifact));
}

/**
 * Adds a display-only covariance. It updates the copied legacy presentation and,
 * when a SemModelV4 already exists, its annotation collection. Scientific identity
 * and compiled estimator plans are intentionally unchanged.
 */
export function authorPresentationCovarianceV4(
  artifact: SemModelV4MigrationArtifact,
  specification: PresentationCovarianceAuthoringV4,
): SemModelV4MigrationArtifact {
  const next = serializableClone(artifact, "migration_artifact");
  validatePresentationCovarianceSpecification(next.source_graph.model, specification);
  const existing = displayCovariancesFromSource(next.source_graph);
  ensureDisplayPairAvailable(existing, specification);
  if ((next.source_graph.presentation.edges ?? []).some((edge) => edge.id === specification.id)) {
    fail("migration.edge_id_duplicate", specification.id, "A presentation edge already uses this id.");
  }
  const edge: Edge = {
    id: specification.id,
    source: specification.left_construct,
    target: specification.right_construct,
    type: "smoothstep",
    label: specification.label ?? "Covariance",
    data: { role: "covariance" } satisfies PathEdgeData,
  };
  next.source_graph.presentation.edges = [...(next.source_graph.presentation.edges ?? []), edge];
  next.covariance_lineage.push({
    lineage_id: stableAdapterId("covariance_lineage", [specification.id]),
    origin: "authored_presentation",
    source_edge_id: specification.id,
    annotation_id: specification.id,
    scientific_relation_id: null,
    scientific_parameter_id: null,
    operation: "presentation_only",
  });

  if (next.kind === "sem_model_v4") {
    next.model.annotations.push({
      kind: "display_only_covariance",
      id: specification.id,
      left: semConstructVariableIdV4(specification.left_construct),
      right: semConstructVariableIdV4(specification.right_construct),
      label: specification.label ?? null,
    });
    assertValidConvertedModel(next.model);
  }
  return deepFreeze(next);
}

/** Adds a scientific covariance relation and free parameter to a confirmed V4 model. */
export function authorScientificCovarianceV4(
  artifact: ConfirmedSemModelV4Migration,
  specification: ScientificCovarianceAuthoringV4,
): ConfirmedSemModelV4Migration {
  return authorScientificCovarianceInternal(artifact, specification, {
    origin: "authored_scientific",
    sourceEdgeId: null,
    annotationId: null,
    operation: "author_scientific_covariance_v1",
  });
}

/**
 * Promotes one legacy display covariance to an executable model covariance. The
 * original annotation and source edge remain intact, and stable lineage points to
 * the added relation and parameter.
 */
export function convertPresentationCovarianceToScientificV4(
  artifact: ConfirmedSemModelV4Migration,
  annotationId: string,
): ConfirmedSemModelV4Migration {
  const annotation = artifact.model.annotations.find((candidate) => (
    candidate.kind === "display_only_covariance" && candidate.id === annotationId
  ));
  if (!annotation || annotation.kind !== "display_only_covariance") {
    fail("migration.display_covariance_unknown", annotationId, "The requested presentation covariance does not exist.");
  }
  const existingLineage = artifact.covariance_lineage.find((entry) => entry.annotation_id === annotationId);
  if (existingLineage?.scientific_relation_id && existingLineage.scientific_parameter_id) {
    const relationExists = artifact.model.relations.some((relation) => relation.id === existingLineage.scientific_relation_id);
    const parameterExists = artifact.model.parameters.some((parameter) => parameter.id === existingLineage.scientific_parameter_id);
    if (!relationExists || !parameterExists) {
      fail("migration.covariance_lineage_broken", annotationId, "Covariance conversion lineage does not match the scientific model.");
    }
    return artifact;
  }
  return authorScientificCovarianceInternal(artifact, {
    id: `from-presentation:${annotationId}`,
    left: { kind: "variable", id: annotation.left },
    right: { kind: "variable", id: annotation.right },
    label: annotation.label ?? null,
    start: 0,
  }, {
    origin: existingLineage?.origin ?? "legacy_presentation",
    sourceEdgeId: existingLineage?.source_edge_id ?? annotationId,
    annotationId,
    operation: "convert_to_model_covariance_v1",
  });
}

/** Restores the exact copied legacy graph while separately exposing V4-only scientific covariances. */
export function roundTripCurrentQuickPlsGraphV4(
  artifact: SemModelV4MigrationArtifact,
): CurrentQuickPlsGraphRoundTripV4 {
  const scientificCovariances = artifact.kind === "sem_model_v4"
    ? artifact.model.relations.flatMap((relation) => relation.kind === "covariance" ? [{
        relation_id: relation.id,
        parameter_id: relation.parameter,
        left: serializableClone(relation.left, `relation.${relation.id}.left`),
        right: serializableClone(relation.right, `relation.${relation.id}.right`),
      }] : [])
    : [];
  return deepFreeze({
    ...serializableClone(artifact.source_graph, "source_graph"),
    scientific_covariances: scientificCovariances,
  });
}

function captureAndValidateSourceGraph(input: MigrateCurrentQuickPlsGraphV4Input): CurrentQuickPlsGraphSnapshotV4 {
  const sourceGraph: CurrentQuickPlsGraphSnapshotV4 = {
    model: serializableClone(input.model, "model"),
    presentation: serializableClone(input.presentation ?? {}, "presentation"),
    data_binding: serializableClone(input.data_binding, "data_binding"),
  };
  validateLegacyModel(sourceGraph.model);
  validateCurrentPresentation(sourceGraph.model, sourceGraph.presentation);
  validateLegacyDataBinding(sourceGraph.model, sourceGraph.data_binding);
  return sourceGraph;
}

function convertSourceGraph(
  sourceGraph: CurrentQuickPlsGraphSnapshotV4,
  interpretation: ConfirmedLegacyEstimandV4,
  displayCovariances: readonly LegacyDisplayCovarianceV4[],
): SemModelV4 {
  const model = convertLegacyBasicModelV4(
    serializableClone(sourceGraph.model, "legacy_model"),
    interpretation,
    serializableClone(displayCovariances, "display_covariances"),
  );
  model.data_binding = serializableClone(sourceGraph.data_binding, "data_binding");
  model.presentation = semPresentationFromCurrent(sourceGraph, model);
  assertValidConvertedModel(model);
  return model;
}

function semPresentationFromCurrent(
  sourceGraph: CurrentQuickPlsGraphSnapshotV4,
  model: SemModelV4,
): SemPresentationV4 {
  const presentation = sourceGraph.presentation;
  const nodes = (presentation.nodes ?? []).map((node) => ({
    variable: semConstructVariableIdV4(node.id),
    x: node.position.x,
    y: node.position.y,
  }));
  const relationByPath = new Map(model.relations.flatMap((relation) => relation.kind === "structural" ? [[
    pathKey(removeConstructNamespace(relation.source), removeConstructNamespace(relation.target)),
    relation.id,
  ] as const] : []));
  const sourceEdgeByPath = new Map((presentation.edges ?? []).flatMap((edge) => (
    isMeasurementEdge(edge) || edgeRole(edge) === "covariance" ? [] : [[pathKey(edge.source, edge.target), edge] as const]
  )));
  const edges = sourceGraph.model.paths.map((path) => {
    const relation = relationByPath.get(pathKey(path.source, path.target));
    if (!relation) fail("migration.presentation_relation_missing", pathKey(path.source, path.target), "A model path has no converted scientific relation.");
    const sourceEdge = sourceEdgeByPath.get(pathKey(path.source, path.target));
    const routing = sourceEdge ? currentEdgeRouting(sourceEdge, presentation) : null;
    return { relation, routing };
  });
  const viewport = presentation.diagramLayout?.diagramViewport;
  return {
    kind: "canvas",
    nodes,
    edges,
    shapes: [],
    images: [],
    lines: [],
    zoom: viewport?.zoom ?? null,
    pan_x: viewport?.x ?? null,
    pan_y: viewport?.y ?? null,
  };
}

function validateLegacyModel(model: NativeCanonicalModelSpec) {
  requiredId(model.id, "model.id");
  if (!model.name.trim()) fail("migration.model_name_empty", model.id, "Model name cannot be empty.");
  const constructIds = new Set<string>();
  const indicatorOwners = new Map<string, string>();
  for (const construct of model.constructs) {
    requiredId(construct.id, "construct.id");
    if (constructIds.has(construct.id)) fail("migration.construct_id_duplicate", construct.id, "Construct ids must be unique.");
    constructIds.add(construct.id);
    if (!construct.name.trim() || !construct.short_name.trim()) fail("migration.construct_label_empty", construct.id, "Construct names and short names cannot be empty.");
    if (!construct.indicators.length) fail("migration.construct_indicators_empty", construct.id, "Every legacy construct needs at least one indicator.");
    const local = new Set<string>();
    for (const indicator of construct.indicators) {
      requiredId(indicator, `construct.${construct.id}.indicator`);
      if (local.has(indicator)) fail("migration.indicator_duplicate", indicator, "An indicator cannot occur twice in one construct.");
      local.add(indicator);
      const owner = indicatorOwners.get(indicator);
      if (owner) fail("migration.indicator_ambiguous", indicator, `Indicator ${indicator} belongs to both ${owner} and ${construct.id}.`);
      indicatorOwners.set(indicator, construct.id);
    }
  }
  const paths = new Set<string>();
  for (const path of model.paths) {
    assertKnownConstructPair(constructIds, path.source, path.target, "migration.path");
    const key = pathKey(path.source, path.target);
    if (paths.has(key)) fail("migration.path_duplicate", key, "Structural paths must be unique.");
    paths.add(key);
  }
  const controls = new Set<string>();
  for (const control of model.controls) {
    assertKnownConstructPair(constructIds, control.source, control.target, "migration.control");
    const key = pathKey(control.source, control.target);
    if (!paths.has(key)) fail("migration.control_path_missing", key, "Every control declaration must reference a structural path.");
    if (controls.has(key)) fail("migration.control_duplicate", key, "Control paths must be unique.");
    controls.add(key);
  }
  for (const higherOrder of model.higher_order_constructs) {
    if (!constructIds.has(higherOrder.id) || higherOrder.components.length < 2 || new Set(higherOrder.components).size !== higherOrder.components.length || higherOrder.components.some((id) => !constructIds.has(id))) {
      fail("migration.higher_order_ambiguous", higherOrder.id, "Higher-order declarations must reference one known output and at least two unique known components.");
    }
  }
  const interactionIds = new Set<string>();
  for (const interaction of model.interactions) {
    if (interactionIds.has(interaction.id)
      || interaction.id !== interaction.product_construct
      || ![interaction.product_construct, interaction.predictor, interaction.moderator, interaction.outcome].every((id) => constructIds.has(id))) {
      fail("migration.interaction_ambiguous", interaction.id, "Interaction declarations must reference unique, known current constructs.");
    }
    interactionIds.add(interaction.id);
  }
}

function validateCurrentPresentation(model: NativeCanonicalModelSpec, presentation: NativeModelPresentation) {
  const constructIds = new Set(model.constructs.map((construct) => construct.id));
  const derivedMeasurements = new Map<string, { source: string; target: string }>(model.constructs.flatMap((construct) => construct.indicators.map((indicator) => {
    const indicatorNode = currentIndicatorNodeId(construct.id, indicator);
    return [`measurement::${construct.id}::${indicator}`, construct.mode === "reflective"
      ? { source: construct.id, target: indicatorNode }
      : { source: indicatorNode, target: construct.id }] as const;
  })));
  const nodeIds = new Set<string>();
  for (const node of presentation.nodes ?? []) {
    if (!constructIds.has(node.id) || nodeIds.has(node.id)) fail("migration.presentation_node_ambiguous", node.id, "Presentation nodes must uniquely reference current constructs.");
    if (!Number.isFinite(node.position?.x) || !Number.isFinite(node.position?.y)) fail("migration.presentation_position_invalid", node.id, "Presentation node coordinates must be finite.");
    nodeIds.add(node.id);
  }
  const edgeIds = new Set<string>();
  const covariancePairs = new Set<string>();
  const structuralPairs = new Set<string>();
  const modelPaths = new Set(model.paths.map((path) => pathKey(path.source, path.target)));
  const controls = new Set(model.controls.map((path) => pathKey(path.source, path.target)));
  for (const edge of presentation.edges ?? []) {
    if (!edge.id.trim() || edgeIds.has(edge.id)) fail("migration.edge_id_duplicate", edge.id, "Presentation edge ids must be non-empty and unique.");
    edgeIds.add(edge.id);
    const role = edgeRole(edge);
    if (isMeasurementEdge(edge)) {
      const expected = derivedMeasurements.get(edge.id);
      if (!expected || role !== undefined || edge.source !== expected.source || edge.target !== expected.target) {
        fail("migration.measurement_edge_ambiguous", edge.id, "Derived measurement edges must exactly match canonical construct indicators.");
      }
      continue;
    }
    if (role !== undefined && role !== "control" && role !== "covariance") {
      fail("migration.edge_role_unknown", edge.id, `Unknown legacy edge role ${String(role)}.`);
    }
    assertKnownConstructPair(constructIds, edge.source, edge.target, "migration.presentation_edge");
    if (role === "covariance") {
      const key = unorderedPathKey(edge.source, edge.target);
      if (covariancePairs.has(key)) fail("migration.display_covariance_duplicate", edge.id, "Display covariance pairs must be unique.");
      covariancePairs.add(key);
      covarianceLabel(edge);
      continue;
    }
    const key = pathKey(edge.source, edge.target);
    if (!modelPaths.has(key)) fail("migration.presentation_path_unknown", edge.id, "A presentation path is not present in the canonical legacy model.");
    if (structuralPairs.has(key)) fail("migration.presentation_path_duplicate", edge.id, "A structural path has more than one presentation edge.");
    structuralPairs.add(key);
    const expectedControl = controls.has(key);
    if ((role === "control") !== expectedControl) fail("migration.presentation_path_role_mismatch", edge.id, "Presentation and canonical control-path semantics disagree.");
  }
  const viewport = presentation.diagramLayout?.diagramViewport;
  if (viewport && ![viewport.x, viewport.y, viewport.zoom].every(Number.isFinite)) {
    fail("migration.presentation_viewport_invalid", model.id, "Presentation viewport values must be finite.");
  }
}

function validateLegacyDataBinding(model: NativeCanonicalModelSpec, binding: SemDataBindingV4) {
  requiredId(binding.dataset_id, "data_binding.dataset_id");
  const observedIds = model.constructs.flatMap((construct) => construct.indicators.map(semObservedVariableIdV4));
  if (binding.kind === "raw") {
    const missing = binding.missing_data;
    const missingIsKnown = typeof missing === "string"
      ? ["listwise_deletion", "pairwise_deletion", "mean_replacement", "full_information_maximum_likelihood"].includes(missing)
      : Number.isInteger(missing.multiple_imputation.imputations) && missing.multiple_imputation.imputations >= 2;
    if (!missingIsKnown) fail("migration.missing_data_invalid", binding.dataset_id, "Missing-data policy is not valid for SemModelV4.");
    if (binding.weight || binding.cluster_variable || binding.strata_variable) {
      fail("migration.data_binding_role_unsupported", binding.dataset_id, "Legacy graph migration requires weight, cluster, and strata variables to be authored explicitly before they are bound.");
    }
    return;
  }
  if (!Number.isInteger(binding.sample.sample_size) || binding.sample.sample_size < 2) {
    fail("migration.matrix_sample_size_invalid", binding.dataset_id, "Matrix input requires an integer sample size of at least two.");
  }
  const expected = new Set(observedIds);
  const actual = new Set(binding.variables);
  if (actual.size !== binding.variables.length || actual.size !== expected.size || [...expected].some((id) => !actual.has(id))) {
    fail("migration.matrix_variables_mismatch", binding.dataset_id, "Matrix bindings must contain each migrated indicator exactly once.");
  }
  for (const [name, values, positive] of [["means", binding.means, false], ["standard_deviations", binding.standard_deviations, true]] as const) {
    if (!values) continue;
    const keys = Object.keys(values);
    if (keys.length !== actual.size || keys.some((key) => !actual.has(key)) || Object.values(values).some((value) => !Number.isFinite(value) || (positive && value <= 0))) {
      fail("migration.matrix_moments_invalid", `${binding.dataset_id}.${name}`, `Matrix ${name} must contain one finite value for each migrated indicator.`);
    }
  }
  if (binding.sample.effective_sample_size != null && (!Number.isFinite(binding.sample.effective_sample_size) || binding.sample.effective_sample_size <= 0)
    || binding.sample.degrees_of_freedom != null && (!Number.isInteger(binding.sample.degrees_of_freedom) || binding.sample.degrees_of_freedom <= 0)
    || Object.keys(binding.sample.group_sample_sizes ?? {}).length) {
    fail("migration.matrix_sample_metadata_invalid", binding.dataset_id, "Single-group matrix metadata requires positive sample values and no group-specific sample counts.");
  }
}

function displayCovariancesFromSource(sourceGraph: CurrentQuickPlsGraphSnapshotV4): LegacyDisplayCovarianceV4[] {
  return (sourceGraph.presentation.edges ?? []).flatMap((edge) => edgeRole(edge) === "covariance" ? [{
    id: edge.id,
    left_construct: edge.source,
    right_construct: edge.target,
    label: covarianceLabel(edge),
  }] : []);
}

function initialCovarianceLineage(displayCovariances: readonly LegacyDisplayCovarianceV4[]): SemCovarianceLineageV4[] {
  return displayCovariances.map((covariance) => ({
    lineage_id: stableAdapterId("covariance_lineage", [covariance.id]),
    origin: "legacy_presentation",
    source_edge_id: covariance.id,
    annotation_id: covariance.id,
    scientific_relation_id: null,
    scientific_parameter_id: null,
    operation: "presentation_only",
  }));
}

function authorScientificCovarianceInternal(
  artifact: ConfirmedSemModelV4Migration,
  specification: ScientificCovarianceAuthoringV4,
  lineage: {
    origin: SemCovarianceLineageV4["origin"];
    sourceEdgeId: string | null;
    annotationId: string | null;
    operation: Extract<SemCovarianceLineageV4["operation"], "author_scientific_covariance_v1" | "convert_to_model_covariance_v1">;
  },
): ConfirmedSemModelV4Migration {
  requiredId(specification.id, "scientific_covariance.id");
  const next = serializableClone(artifact, "migration_artifact");
  const [left, right] = canonicalEndpointPair(specification.left, specification.right);
  const relationId = stableAdapterId("scientific_covariance", [specification.id]);
  const parameterId = stableAdapterId("scientific_covariance_parameter", [specification.id]);
  const existingRelation = next.model.relations.find((relation) => relation.id === relationId);
  const existingParameter = next.model.parameters.find((parameter) => parameter.id === parameterId);
  if (existingRelation || existingParameter) {
    fail("migration.scientific_covariance_id_duplicate", specification.id, "A scientific covariance already uses this stable id.");
  }
  const duplicatePair = next.model.relations.find((relation) => relation.kind === "covariance" && endpointPairKey(relation.left, relation.right) === endpointPairKey(left, right));
  if (duplicatePair) fail("migration.scientific_covariance_duplicate", duplicatePair.id, "A scientific covariance already exists for these endpoints.");
  const relation: SemRelationV4 = { kind: "covariance", id: relationId, left, right, parameter: parameterId };
  const parameter: SemParameterV4 = {
    kind: "free",
    id: parameterId,
    label: specification.label?.trim() || `Cov(${endpointLabel(left)}, ${endpointLabel(right)})`,
    target: { kind: "covariance", left, right },
    start: specification.start ?? 0,
    lower: specification.lower ?? null,
    upper: specification.upper ?? null,
    equality_label: null,
    group_overrides: [],
  };
  next.model.relations.push(relation);
  next.model.parameters.push(parameter);
  if (next.model.presentation.kind === "none") {
    next.model.presentation = { kind: "canvas", nodes: [], edges: [], shapes: [], images: [], lines: [] };
  }
  next.model.presentation.edges.push({ relation: relationId, routing: specification.routing ?? null });
  const lineageEntry: SemCovarianceLineageV4 = {
    lineage_id: stableAdapterId("covariance_lineage", [specification.id]),
    origin: lineage.origin,
    source_edge_id: lineage.sourceEdgeId,
    annotation_id: lineage.annotationId,
    scientific_relation_id: relationId,
    scientific_parameter_id: parameterId,
    operation: lineage.operation,
  };
  const lineageIndex = next.covariance_lineage.findIndex((entry) => lineage.annotationId && entry.annotation_id === lineage.annotationId);
  if (lineageIndex >= 0) {
    const previous = next.covariance_lineage[lineageIndex];
    next.covariance_lineage[lineageIndex] = {
      ...lineageEntry,
      lineage_id: previous.lineage_id,
      origin: previous.origin,
      source_edge_id: previous.source_edge_id,
    };
  }
  else next.covariance_lineage.push(lineageEntry);
  assertValidConvertedModel(next.model);
  return deepFreeze(next);
}

function validatePresentationCovarianceSpecification(model: NativeCanonicalModelSpec, specification: PresentationCovarianceAuthoringV4) {
  requiredId(specification.id, "presentation_covariance.id");
  const constructIds = new Set(model.constructs.map((construct) => construct.id));
  assertKnownConstructPair(constructIds, specification.left_construct, specification.right_construct, "migration.presentation_covariance");
}

function ensureDisplayPairAvailable(
  existing: readonly LegacyDisplayCovarianceV4[],
  specification: PresentationCovarianceAuthoringV4,
) {
  const pair = unorderedPathKey(specification.left_construct, specification.right_construct);
  if (existing.some((candidate) => unorderedPathKey(candidate.left_construct, candidate.right_construct) === pair)) {
    fail("migration.display_covariance_duplicate", specification.id, "A presentation covariance already exists for this construct pair.");
  }
}

function assertValidConvertedModel(model: SemModelV4) {
  const issues = validateSemModelV4(model);
  if (issues.length) {
    throw new SemModelV4OperationError("migration.converted_model_invalid", issues[0].subject ?? "model", issues);
  }
}

function automaticInterpretation(intent: LegacySemMethodIntentV4): ConfirmedLegacyEstimandV4 | null {
  if (intent === "pls_sem") return "pls_composite";
  if (intent === "cbsem") return "cbsem_common_factor";
  return null;
}

function isAutomaticConversionBlocker(code: string) {
  return code === "migration.advanced_semantics" || code === "migration.formative_cbsem";
}

function blockerFrom(error: SemModelV4OperationError): SemModelV4MigrationBlocker {
  return { code: error.code, subject: error.subject, message: error.message };
}

function edgeRole(edge: Edge): unknown {
  const data = edge.data;
  return data && typeof data === "object" && !Array.isArray(data) ? (data as Record<string, unknown>).role : undefined;
}

function covarianceLabel(edge: Edge): string | null {
  if (edge.label == null) return null;
  if (typeof edge.label === "string" || typeof edge.label === "number") return String(edge.label);
  fail("migration.display_covariance_label_ambiguous", edge.id, "Display covariance labels must be plain text.");
}

function isMeasurementEdge(edge: Edge) {
  return edge.id.startsWith("measurement::");
}

function currentIndicatorNodeId(constructId: string, indicator: string) {
  return `indicator::${constructId}::${encodeURIComponent(indicator)}`;
}

function currentEdgeRouting(edge: Edge, presentation: NativeModelPresentation): string | null {
  const configured = presentation.diagramLayout?.edgeLayouts?.[edge.id]?.routing;
  if (configured) return configured;
  const type = edge.type?.toLowerCase();
  if (type === "straight") return "straight";
  if (type === "bezier" || type === "simplebezier") return "curved";
  if (type === "step" || type === "smoothstep") return "orthogonal";
  return null;
}

function assertKnownConstructPair(constructIds: Set<string>, source: string, target: string, codePrefix: string) {
  if (!constructIds.has(source)) fail(`${codePrefix}_source_unknown`, source, `Unknown construct ${source}.`);
  if (!constructIds.has(target)) fail(`${codePrefix}_target_unknown`, target, `Unknown construct ${target}.`);
  if (source === target) fail(`${codePrefix}_self`, source, "Self paths and self covariances are not valid legacy graph relations.");
}

function pathKey(source: string, target: string) {
  return `${source}\u0000${target}`;
}

function unorderedPathKey(left: string, right: string) {
  return [left, right].sort().join("\u0000");
}

function endpointKey(endpoint: SemEndpointV4) {
  return `${endpoint.kind}\u0000${endpoint.id}`;
}

function endpointPairKey(left: SemEndpointV4, right: SemEndpointV4) {
  return [endpointKey(left), endpointKey(right)].sort().join("\u0001");
}

function canonicalEndpointPair(left: SemEndpointV4, right: SemEndpointV4): [SemEndpointV4, SemEndpointV4] {
  const leftCopy = serializableClone(left, "covariance.left");
  const rightCopy = serializableClone(right, "covariance.right");
  return endpointKey(leftCopy) <= endpointKey(rightCopy) ? [leftCopy, rightCopy] : [rightCopy, leftCopy];
}

function endpointLabel(endpoint: SemEndpointV4) {
  if (endpoint.kind === "variable") return endpoint.id;
  if (endpoint.kind === "residual_of") return `residual(${endpoint.id})`;
  return `disturbance(${endpoint.id})`;
}

function removeConstructNamespace(id: string) {
  return id.startsWith("construct:") ? id.slice("construct:".length) : id;
}

function requiredId(value: string, subject: string) {
  if (typeof value !== "string" || !value.trim()) fail("migration.id_empty", subject, `${subject} cannot be empty.`);
  return value;
}

function stableAdapterId(prefix: string, parts: readonly string[]) {
  const encoder = new TextEncoder();
  const encoded = parts.map((part) => [...encoder.encode(part)].map((byte) => byte.toString(16).padStart(2, "0")).join("")).join("_");
  return `${prefix}_${encoded}`;
}

function serializableClone<T>(value: T, subject: string): T {
  try {
    const serialized = JSON.stringify(value);
    if (serialized === undefined) throw new Error("not serializable");
    return JSON.parse(serialized) as T;
  } catch {
    fail("migration.source_not_serializable", subject, `${subject} must be JSON-serializable before migration.`);
  }
}

function deepFreeze<T>(value: T): T {
  if (value && typeof value === "object" && !Object.isFrozen(value)) {
    Object.freeze(value);
    for (const child of Object.values(value as Record<string, unknown>)) deepFreeze(child);
  }
  return value;
}

function fail(code: string, subject: string, message: string): never {
  throw new SemModelV4MigrationAdapterError(code, subject, message);
}
