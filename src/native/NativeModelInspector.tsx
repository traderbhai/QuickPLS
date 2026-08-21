import { useEffect, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";
import type { Edge, Node } from "@xyflow/react";
import {
  type StandardSemModelV4AuthorityRecordV1,
  type StandardSemModelV4EditorIntentV1,
  type StandardSemRelationshipDefinitionV1,
} from "../domain/standardSemModelV4Authority";
import { compareUtf8StringsV1, type SemVariableV4 } from "../domain/semModelV4";
import { capabilityRegistryV2 } from "../domain/capabilityRegistryV2";
import { GENERAL_SEM_CBSEM_POINT_CAPABILITY_CELL_V1 } from "../domain/internalRecipeV4GeneralSemWorkspace";
import { useWorkspace, type StandardSemModelV4AuthorityCommitResult } from "../store";
import {
  NativeSemConstructAuthoringFields,
  NativeSemCovarianceAuthoringFields,
} from "./NativeSemScientificAuthoring";
import { StandardSemModelV4AdvancedEditor } from "./StandardSemModelV4AdvancedEditor";
import {
  nativePathDisplayLabel,
  nativePathLabelPatch,
  nativePathRolePatch,
  type NativePathRole,
} from "./nativePathProperties";
import type { NativePlsReadiness, NativePlsReadinessStatus } from "./nativePlsReadiness";
import type {
  ConstructData,
  Dataset,
  HigherOrderConstructData,
  InteractionData,
  PathEdgeData,
  SemModelV4ConstructAuthoring,
} from "../types";

export type NativeModelInspectorMode = "basic" | "expert";
export type NativeModelInspectorTab = "model" | "parameter" | "appearance" | "data-binding";

export const NATIVE_MODEL_INSPECTOR_TABS: readonly NativeModelInspectorTab[] = [
  "model",
  "parameter",
  "appearance",
  "data-binding",
];

const standardCbsemConstructAuthoringAvailable = capabilityRegistryV2.availability(
  GENERAL_SEM_CBSEM_POINT_CAPABILITY_CELL_V1.capability_id,
  GENERAL_SEM_CBSEM_POINT_CAPABILITY_CELL_V1.cell_id,
  false,
).selectable;

type AuthorityFeedback = { tone: "pending" | "committed" | "blocked" | "stale" | "rejected"; message: string };

function authorityFeedbackFor(result: StandardSemModelV4AuthorityCommitResult): AuthorityFeedback {
  if (result.status === "committed") return { tone: "committed", message: "Committed to the strict Standard model authority." };
  if (result.status === "blocked") return { tone: "blocked", message: `Blocked: ${result.diagnostic.message} ${result.diagnostic.correctiveAction}` };
  if (result.status === "stale") return { tone: "stale", message: "Stale edit ignored because the active model authority changed. Review the current model and retry." };
  const detail = result.error instanceof Error ? result.error.message : String(result.error);
  return { tone: "rejected", message: `Rejected: ${detail}` };
}

function observedVariableForColumn(
  authority: StandardSemModelV4AuthorityRecordV1,
  dataset: Dataset,
  column: string,
): Extract<SemVariableV4, { kind: "observed" }> {
  const existing = authority.model.variables.find((variable): variable is Extract<SemVariableV4, { kind: "observed" }> =>
    variable.kind === "observed" && variable.source_column === column);
  if (existing) return structuredClone(existing);
  const metadata = dataset.columnMetadata?.find((item) => item.name === column);
  return {
    kind: "observed",
    id: `observed:${column}`,
    label: metadata?.label?.trim() || column,
    source_column: column,
    scale: metadata?.scale_type ?? "continuous",
    role: "indicator",
    categories: Object.keys(metadata?.value_labels ?? {}).sort(compareUtf8StringsV1),
    value_labels: { ...(metadata?.value_labels ?? {}) },
    missing_markers: [...new Set((metadata?.missing_markers ?? []).map((value) => value.trim()).filter(Boolean))].sort(compareUtf8StringsV1),
    transformation_lineage: [],
  };
}

function relationshipDefinition(
  authority: StandardSemModelV4AuthorityRecordV1,
  edge: Edge,
  role: NativePathRole,
  labelOverride?: string,
): StandardSemRelationshipDefinitionV1 | null {
  const relation = authority.model.relations.find((item) => item.id === edge.id);
  const annotation = authority.model.annotations.find((item) => item.kind === "display_only_covariance" && item.id === edge.id);
  const parameter = relation ? authority.model.parameters.find((item) => item.id === relation.parameter) : null;
  const label = labelOverride ?? parameter?.label ?? (annotation?.kind === "display_only_covariance" ? annotation.label : null) ?? String(edge.label ?? "Relationship");
  if (role === "covariance") {
    if (annotation?.kind === "display_only_covariance") return { kind: "presentation_only_covariance", left: annotation.left, right: annotation.right, label };
    if (relation?.kind === "covariance") return { kind: "covariance", left: relation.left, right: relation.right, label };
    return { kind: "covariance", left: { kind: "variable", id: edge.source }, right: { kind: "variable", id: edge.target }, label };
  }
  const source = relation?.kind === "structural" ? relation.source : edge.source;
  const target = relation?.kind === "structural" ? relation.target : edge.target;
  return role === "control" ? { kind: "control", source, target, label } : { kind: "structural", source, target, label };
}

function authoredCovarianceDefinition(
  edge: Edge,
  fallback: StandardSemRelationshipDefinitionV1,
): StandardSemRelationshipDefinitionV1 {
  const authored = (edge.data as PathEdgeData | undefined)?.semModelV4?.covariance;
  const label = "label" in fallback ? fallback.label : "Covariance";
  if (authored?.kind === "scientific" && authored.left && authored.right) {
    return { kind: "covariance", left: authored.left, right: authored.right, label };
  }
  if (authored?.kind === "presentation_only") {
    return { kind: "presentation_only_covariance", left: edge.source, right: edge.target, label };
  }
  return fallback;
}

function representationIntent(
  authority: StandardSemModelV4AuthorityRecordV1,
  variableId: string,
  specification: SemModelV4ConstructAuthoring,
  mode: "reflective" | "formative",
): StandardSemModelV4EditorIntentV1 | null {
  if (specification.kind === "legacy_estimand_unspecified") return null;
  if (specification.kind === "composite") return {
    kind: "set_construct_representation",
    variable_id: variableId,
    representation: { kind: "composite", weighting: { kind: mode === "formative" ? "mode_b" : "mode_a" } },
  };
  const indicators = authority.model.relations.flatMap((relation) =>
    relation.kind === "measurement_effect" && relation.construct === variableId
      ? [relation.indicator]
      : []);
  const marker = authority.model.variables.find((variable) =>
    variable.kind === "observed" && variable.source_column === specification.marker_indicator)?.id
    ?? indicators[0];
  if (!marker) return null;
  return {
    kind: "set_construct_representation",
    variable_id: variableId,
    representation: { kind: "common_factor", identification: { kind: "marker_loading", indicator: marker } },
  };
}

export interface NativeModelInspectorPreflightPreview {
  status: NativePlsReadinessStatus;
  headline: string;
  detail: string;
  calculationPlan: string;
}

export function nativeModelInspectorPreflightPreview(
  readiness: NativePlsReadiness,
): NativeModelInspectorPreflightPreview {
  const calculation = readiness.items.find((item) => item.id === "calculation");
  if (readiness.blockers.length) {
    const count = readiness.blockers.length;
    return {
      status: "blocked",
      headline: `${count} thing${count === 1 ? "" : "s"} to fix before calculation`,
      detail: readiness.blockers[0].detail,
      calculationPlan: calculation?.detail ?? "No calculation plan is available until the model setup is complete.",
    };
  }
  if (readiness.warnings.length) {
    const count = readiness.warnings.length;
    return {
      status: "warning",
      headline: `Ready, with ${count} item${count === 1 ? "" : "s"} to review`,
      detail: readiness.warnings[0].detail,
      calculationPlan: calculation?.detail ?? "The current calculation plan does not report additional workload details.",
    };
  }
  return {
    status: "ready",
    headline: "Ready to calculate",
    detail: "All required model, data, runtime, and calculation checks passed.",
    calculationPlan: calculation?.detail ?? "The current calculation plan does not report additional workload details.",
  };
}

const TAB_LABELS: Record<NativeModelInspectorTab, string> = {
  model: "Model",
  parameter: "Parameter",
  appearance: "Appearance",
  "data-binding": "Data Binding",
};

const INTERACTION_METHOD_LABELS: Record<NonNullable<InteractionData["canonicalMethod"]>, string> = {
  two_stage: "Two-stage",
  product_indicator: "Product indicator",
  orthogonalizing: "Orthogonalizing",
};

const HIGHER_ORDER_APPROACH_LABELS: Record<NonNullable<HigherOrderConstructData["canonicalApproach"]>, string> = {
  repeated_indicators: "Repeated indicators",
  extended_repeated_indicators: "Extended repeated indicators",
  embedded_two_stage: "Embedded two-stage",
  disjoint_two_stage: "Disjoint two-stage",
  hybrid: "Hybrid",
};

const HIGHER_ORDER_MEASUREMENT_LABELS: Record<NonNullable<HigherOrderConstructData["measurementType"]>, string> = {
  reflective_reflective: "Reflective–reflective higher-order construct",
  reflective_formative: "Reflective–formative higher-order construct",
  formative_reflective: "Formative–reflective higher-order construct",
  formative_formative: "Formative–formative higher-order construct",
};

function interactionMethodLabel(interaction: InteractionData): string {
  return interaction.canonicalMethod
    ? INTERACTION_METHOD_LABELS[interaction.canonicalMethod]
    : "Two-stage product score";
}

function interactionOperands(interaction: InteractionData): readonly string[] {
  return interaction.kind === "interaction_v2"
    ? interaction.operands
    : [interaction.predictor, interaction.moderator];
}

function interactionRequiredPaths(
  interactionId: string,
  interaction: InteractionData,
): Array<{ source: string; target: string; relationId?: string }> {
  const operands = interactionOperands(interaction);
  const paths: Array<{ source: string; target: string; relationId?: string }> = [
    {
      source: operands[0]!,
      target: interaction.outcome,
      ...(interaction.focalRelationId ? { relationId: interaction.focalRelationId } : {}),
    },
    { source: interactionId, target: interaction.outcome },
  ];
  if (interaction.kind !== "interaction_v2" || interaction.hierarchyPolicy !== "none") {
    paths.push(...operands.slice(1).map((operand) => ({ source: operand, target: interaction.outcome })));
  }
  return paths;
}

function higherOrderApproachLabel(higherOrder: HigherOrderConstructData): string {
  if (higherOrder.canonicalApproach) return HIGHER_ORDER_APPROACH_LABELS[higherOrder.canonicalApproach];
  if (higherOrder.method === "repeated_indicators") return "Repeated indicators";
  if (higherOrder.method === "hybrid") return "Hybrid";
  return "Disjoint two-stage";
}

function higherOrderMeasurementLabel(higherOrder: HigherOrderConstructData): string {
  return higherOrder.measurementType
    ? HIGHER_ORDER_MEASUREMENT_LABELS[higherOrder.measurementType]
    : "Reflective–reflective higher-order construct";
}

export function nextNativeModelInspectorTab(
  current: NativeModelInspectorTab,
  key: string,
): NativeModelInspectorTab {
  const currentIndex = Math.max(0, NATIVE_MODEL_INSPECTOR_TABS.indexOf(current));
  if (key === "Home") return NATIVE_MODEL_INSPECTOR_TABS[0];
  if (key === "End") return NATIVE_MODEL_INSPECTOR_TABS[NATIVE_MODEL_INSPECTOR_TABS.length - 1];
  if (key === "ArrowLeft") {
    return NATIVE_MODEL_INSPECTOR_TABS[(currentIndex - 1 + NATIVE_MODEL_INSPECTOR_TABS.length) % NATIVE_MODEL_INSPECTOR_TABS.length];
  }
  if (key === "ArrowRight") {
    return NATIVE_MODEL_INSPECTOR_TABS[(currentIndex + 1) % NATIVE_MODEL_INSPECTOR_TABS.length];
  }
  return current;
}

function CommitTextInput({
  id,
  value,
  onCommit,
  maxLength,
  allowEmpty = false,
}: {
  id?: string;
  value: string;
  onCommit: (value: string) => void;
  maxLength?: number;
  allowEmpty?: boolean;
}) {
  const [draft, setDraft] = useState(value);
  useEffect(() => setDraft(value), [value]);
  const commit = () => {
    const next = draft.trim();
    if ((next || allowEmpty) && next !== value) onCommit(next);
    else setDraft(value);
  };
  return <input
    id={id}
    type="text"
    value={draft}
    maxLength={maxLength}
    onChange={(event) => setDraft(event.target.value)}
    onBlur={commit}
    onKeyDown={(event) => {
      if (event.key === "Enter") {
        event.preventDefault();
        event.currentTarget.blur();
      } else if (event.key === "Escape") {
        setDraft(value);
        event.currentTarget.blur();
      }
    }}
  />;
}

export interface NativeModelInspectorProps {
  initialMode?: NativeModelInspectorMode;
  initialTab?: NativeModelInspectorTab;
  nodesOverride?: Array<Node<ConstructData>>;
  selectedNodeIdOverride?: string | null;
  selectedEdgeIdOverride?: string | null;
  experimentalLabsEnabledOverride?: boolean;
  strictAuthorityOverride?: StandardSemModelV4AuthorityRecordV1 | null;
  readiness?: NativePlsReadiness;
}

export function NativeModelInspector({
  initialMode = "basic",
  initialTab = "model",
  nodesOverride,
  selectedNodeIdOverride,
  selectedEdgeIdOverride,
  experimentalLabsEnabledOverride,
  strictAuthorityOverride,
  readiness,
}: NativeModelInspectorProps = {}) {
  const dataset = useWorkspace((state) => state.dataset);
  const groupingVariable = useWorkspace((state) => state.analysisSettings.groupColumn?.trim() ?? "");
  const storeNodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const storeSelectedNodeId = useWorkspace((state) => state.selectedNodeId);
  const storeSelectedEdgeId = useWorkspace((state) => state.selectedEdgeId);
  const storeExperimentalSemAuthoringEnabled = useWorkspace((state) => state.uiPreferences.experimentalLabsEnabled);
  const generalSemPublicationPending = useWorkspace((state) => state.generalSemPublicationPending);
  const diagramLayout = useWorkspace((state) => state.diagramLayout);
  const activeModelId = useWorkspace((state) => state.activeModelId);
  const storeStrictAuthority = useWorkspace((state) => state.activeModelId
    ? state.standardSemModelV4Authorities[state.activeModelId] ?? null
    : null);
  const strictAuthority = strictAuthorityOverride === undefined ? storeStrictAuthority : strictAuthorityOverride;
  const commitStandardIntent = useWorkspace((state) => state.commitStandardSemModelV4Intent);
  const updateConstruct = useWorkspace((state) => state.updateConstruct);
  const updateEdge = useWorkspace((state) => state.updateEdge);
  const setConstructEstimandV4 = useWorkspace((state) => state.setConstructEstimandV4);
  const assignIndicator = useWorkspace((state) => state.assignIndicator);
  const unassignIndicator = useWorkspace((state) => state.unassignIndicator);
  const reverseSelectedPath = useWorkspace((state) => state.reverseSelectedPath);
  const setSelectedPathRouting = useWorkspace((state) => state.setSelectedPathRouting);
  const removeSelection = useWorkspace((state) => state.removeSelection);

  const selectedNodeId = selectedNodeIdOverride === undefined ? storeSelectedNodeId : selectedNodeIdOverride;
  const selectedEdgeId = selectedEdgeIdOverride === undefined ? storeSelectedEdgeId : selectedEdgeIdOverride;
  const experimentalSemAuthoringEnabled = experimentalLabsEnabledOverride ?? storeExperimentalSemAuthoringEnabled;
  const constructRepresentationAuthoringEnabled = experimentalSemAuthoringEnabled
    || standardCbsemConstructAuthoringAvailable;
  const nodes = nodesOverride ?? storeNodes;
  const selected = nodes.find((node) => node.id === selectedNodeId);
  const selectedInteraction = selected?.data.semantic === "interaction"
    ? selected.data.interaction
    : undefined;
  const selectedPath = edges.find((edge) => edge.id === selectedEdgeId && !edge.id.startsWith("measurement::"));
  const source = nodes.find((node) => node.id === selectedPath?.source)?.data.label ?? selectedPath?.source ?? "";
  const target = nodes.find((node) => node.id === selectedPath?.target)?.data.label ?? selectedPath?.target ?? "";
  const routing = diagramLayout.edgeLayouts[selectedPath?.id ?? ""]?.routing ?? "straight";
  const pathRole: NativePathRole = selectedPath?.data?.role === "control" || selectedPath?.data?.role === "covariance"
    ? selectedPath.data.role
    : "structural";
  const selectedPathSupportsModeration = selectedPath ? nodes.some((node) => {
    const interaction = node.data.semantic === "interaction" ? node.data.interaction : undefined;
    return interaction && interactionRequiredPaths(node.id, interaction)
      .some((required) => selectedPath.source === required.source
        && selectedPath.target === required.target
        && (!required.relationId || selectedPath.id === required.relationId));
  }) : false;
  const assignedIndicators = new Set(nodes.flatMap((node) => node.data.indicators));
  const availableIndicators = dataset.columns.filter((column) => column !== groupingVariable && !assignedIndicators.has(column));

  const [mode, setMode] = useState<NativeModelInspectorMode>(initialMode);
  const [activeTab, setActiveTab] = useState<NativeModelInspectorTab>(initialTab);
  const [authorityFeedback, setAuthorityFeedback] = useState<AuthorityFeedback | null>(null);

  useEffect(() => setAuthorityFeedback(null), [activeModelId, selectedNodeId, selectedEdgeId]);
  const commitAuthority = async (intent: StandardSemModelV4EditorIntentV1) => {
    setAuthorityFeedback({ tone: "pending", message: "Committing strict Standard model edit…" });
    const result = await commitStandardIntent(intent);
    setAuthorityFeedback(authorityFeedbackFor(result));
    return result;
  };

  useEffect(() => setActiveTab("model"), [selectedNodeId, selectedEdgeId]);
  useEffect(() => {
    const showModelEditor = () => setActiveTab("model");
    window.addEventListener("quickpls:model-inspector-show-editor", showModelEditor);
    return () => window.removeEventListener("quickpls:model-inspector-show-editor", showModelEditor);
  }, []);

  const selectTab = (tab: NativeModelInspectorTab, moveFocus = false) => {
    setActiveTab(tab);
    if (moveFocus) window.setTimeout(() => document.getElementById(`nd-model-inspector-${tab}-tab`)?.focus(), 0);
  };
  const onTabKeyDown = (event: ReactKeyboardEvent<HTMLButtonElement>) => {
    if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    selectTab(nextNativeModelInspectorTab(activeTab, event.key), true);
  };
  const setPathRole = (role: NativePathRole) => {
    if (!selectedPath) return;
    if (strictAuthority) {
      const definition = relationshipDefinition(strictAuthority, selectedPath, role);
      if (definition) void commitAuthority({ kind: "replace_relationship", relationship_id: selectedPath.id, definition });
      return;
    }
    updateEdge(selectedPath.id, nativePathRolePatch(selectedPath, role));
  };

  const renameConstruct = (label: string) => {
    if (strictAuthority && selected) void commitAuthority({ kind: "rename_construct", variable_id: selected.id, label });
    else if (selected) updateConstruct(selected.id, { label });
  };
  const renamePath = (label: string) => {
    if (!selectedPath) return;
    if (strictAuthority) {
      const definition = relationshipDefinition(strictAuthority, selectedPath, pathRole, label);
      if (definition) void commitAuthority({ kind: "replace_relationship", relationship_id: selectedPath.id, definition });
    } else updateEdge(selectedPath.id, nativePathLabelPatch(selectedPath, pathRole, label));
  };
  const setMeasurementMode = (nextMode: "reflective" | "formative") => {
    if (!selected) return;
    if (!strictAuthority) {
      updateConstruct(selected.id, { mode: nextMode });
      return;
    }
    const current = strictAuthority.model.variables.find((variable) => variable.id === selected.id);
    const representation = nextMode === "formative"
      ? { kind: "composite" as const, weighting: { kind: "mode_b" as const } }
      : current?.kind === "common_factor"
        ? { kind: "common_factor" as const, identification: current.identification }
        : { kind: "composite" as const, weighting: { kind: "mode_a" as const } };
    void commitAuthority({ kind: "set_construct_representation", variable_id: selected.id, representation });
  };
  const setScientificRepresentation = (specification: SemModelV4ConstructAuthoring) => {
    if (!selected) return;
    if (!strictAuthority) {
      setConstructEstimandV4(selected.id, specification);
      return;
    }
    const intent = representationIntent(strictAuthority, selected.id, specification, selected.data.mode);
    if (intent) void commitAuthority(intent);
    else setAuthorityFeedback({ tone: "rejected", message: "Rejected: a strict Standard model cannot clear its explicit scientific representation." });
  };
  const assignDatasetIndicator = (column: string) => {
    if (!selected || !column) return;
    if (!strictAuthority) {
      assignIndicator(selected.id, column);
      return;
    }
    void commitAuthority({
      kind: "assign_indicators",
      construct_id: selected.id,
      indicators: [observedVariableForColumn(strictAuthority, dataset, column)],
    });
  };
  const removeDatasetIndicator = (column: string) => {
    if (!selected) return;
    if (!strictAuthority) {
      unassignIndicator(selected.id, column);
      return;
    }
    const observed = strictAuthority.model.variables.find((variable) => variable.kind === "observed" && variable.source_column === column);
    if (!observed || observed.kind !== "observed") {
      setAuthorityFeedback({ tone: "rejected", message: `Rejected: ${column} is not present in the strict authority.` });
      return;
    }
    const construct = strictAuthority.model.variables.find((variable) => variable.id === selected.id);
    const replacementRelation = strictAuthority.model.relations.find((relation) =>
      relation.kind === "measurement_effect" && relation.construct === selected.id && relation.indicator !== observed.id);
    const replacement = construct?.kind === "common_factor" && construct.identification.kind === "marker_loading" && construct.identification.indicator === observed.id
      && replacementRelation?.kind === "measurement_effect"
      ? replacementRelation.indicator
      : null;
    void commitAuthority({ kind: "remove_indicator", construct_id: selected.id, observed_id: observed.id, replacement_marker: replacement });
  };
  const reversePath = () => {
    if (!selectedPath) return;
    if (!strictAuthority) {
      reverseSelectedPath();
      return;
    }
    const current = relationshipDefinition(strictAuthority, selectedPath, pathRole);
    if (!current || current.kind === "covariance" || current.kind === "presentation_only_covariance") {
      setAuthorityFeedback({ tone: "rejected", message: "Rejected: covariance relationships do not have a direction to reverse." });
      return;
    }
    void commitAuthority({ kind: "replace_relationship", relationship_id: selectedPath.id, definition: { ...current, source: current.target, target: current.source } });
  };
  const deleteSelection = () => {
    if (!strictAuthority) {
      removeSelection();
      return;
    }
    if (selected) void commitAuthority({ kind: "delete_construct", variable_id: selected.id });
    else if (selectedPath) void commitAuthority({ kind: "delete_relationship", relationship_id: selectedPath.id });
  };

  const heading = selected ? "Construct" : selectedPath ? "Path" : "Model properties";
  const panelId = `nd-model-inspector-${activeTab}-panel`;
  const tabId = `nd-model-inspector-${activeTab}-tab`;
  const preflight = readiness ? nativeModelInspectorPreflightPreview(readiness) : null;
  const assignedIndicatorCount = nodes.reduce((count, node) => count + node.data.indicators.length, 0);

  return <aside className="nd-properties nd-model-inspector" aria-labelledby="nd-model-inspector-heading">
    <header className="nd-pane-title"><strong id="nd-model-inspector-heading">{heading}</strong></header>
    <div className="nd-inspector-mode" role="group" aria-label="Inspector mode">
      {(["basic", "expert"] as const).map((candidate) => <button
        key={candidate}
        type="button"
        aria-pressed={mode === candidate}
        onClick={() => setMode(candidate)}
      >{candidate === "basic" ? "Basic" : "Expert"}</button>)}
    </div>
    <div className="nd-inspector-tabs" role="tablist" aria-label="Model inspector sections">
      {NATIVE_MODEL_INSPECTOR_TABS.map((tab) => <button
        id={`nd-model-inspector-${tab}-tab`}
        key={tab}
        type="button"
        role="tab"
        aria-selected={activeTab === tab}
        aria-controls={`nd-model-inspector-${tab}-panel`}
        tabIndex={activeTab === tab ? 0 : -1}
        onClick={() => selectTab(tab)}
        onKeyDown={onTabKeyDown}
      >{TAB_LABELS[tab]}</button>)}
    </div>
    {generalSemPublicationPending ? <p className="nd-property-note" role="status">Calculation-ready project publication is in progress. Model and presentation editing are temporarily locked.</p> : null}
    <fieldset disabled={generalSemPublicationPending} style={{ border: 0, margin: 0, minInlineSize: 0, padding: 0 }}>
    <section id={panelId} className="nd-inspector-panel" role="tabpanel" aria-labelledby={tabId} tabIndex={0}>
      {activeTab === "model" ? <form className="nd-property-form" onSubmit={(event) => event.preventDefault()}>
        {selected ? <>
          <label>Name<CommitTextInput id="nd-model-construct-name" value={selected.data.label} onCommit={renameConstruct} /></label>
          {selected.data.semantic !== "interaction" ? <label>Short name<CommitTextInput value={selected.data.shortName} maxLength={12} onCommit={(shortName) => updateConstruct(selected.id, { shortName })} /></label> : null}
          {strictAuthority ? <p className="nd-property-note">The short name is projected presentation metadata; strict scientific edits use the authority controls below.</p> : null}
          {selectedInteraction ? <dl className="nd-property-list">
            {selectedInteraction.kind === "interaction_v2" ? <>
              <div><dt>Focal predictor</dt><dd>{nodes.find((node) => node.id === selectedInteraction.operands[0])?.data.label ?? selectedInteraction.operands[0]}</dd></div>
              <div><dt>Moderators (authored order)</dt><dd>{selectedInteraction.operands.slice(1).map((operand) => nodes.find((node) => node.id === operand)?.data.label ?? operand).join(" × ")}</dd></div>
              <div><dt>Hierarchy policy</dt><dd>{selectedInteraction.hierarchyPolicy}</dd></div>
            </> : <>
              <div><dt>Predictor</dt><dd>{nodes.find((node) => node.id === selectedInteraction.predictor)?.data.label ?? selectedInteraction.predictor}</dd></div>
              <div><dt>Moderator</dt><dd>{nodes.find((node) => node.id === selectedInteraction.moderator)?.data.label ?? selectedInteraction.moderator}</dd></div>
            </>}
            <div><dt>Outcome</dt><dd>{nodes.find((node) => node.id === selectedInteraction.outcome)?.data.label ?? selectedInteraction.outcome}</dd></div>
          </dl> : selected.data.semantic === "higher_order" && selected.data.higherOrder ? <dl className="nd-property-list">
            <div><dt>Type</dt><dd>{higherOrderMeasurementLabel(selected.data.higherOrder)}</dd></div>
            <div><dt>Method</dt><dd>{higherOrderApproachLabel(selected.data.higherOrder)}</dd></div>
            <div><dt>Components</dt><dd>{selected.data.higherOrder.components.map((component) => nodes.find((node) => node.id === component)?.data.label ?? component).join(", ")}</dd></div>
            <div><dt>Indicators</dt><dd>Generated component scores</dd></div>
          </dl> : <dl className="nd-property-list"><div><dt>Object</dt><dd>Construct</dd></div><div><dt>Indicators</dt><dd>{selected.data.indicators.length}</dd></div></dl>}
        </> : selectedPath ? <>
          <dl className="nd-property-list"><div><dt>Source</dt><dd>{source}</dd></div><div><dt>Target</dt><dd>{target}</dd></div></dl>
          <label>Label<CommitTextInput id="nd-model-path-label" allowEmpty value={nativePathDisplayLabel(selectedPath, pathRole)} onCommit={renamePath} /></label>
          {selectedPathSupportsModeration ? <p className="nd-property-note">This relationship is required by the current moderating effect.</p> : null}
        </> : <>
          <dl className="nd-property-list"><div><dt>Constructs</dt><dd>{nodes.length}</dd></div><div><dt>Relationships</dt><dd>{edges.filter((edge) => !edge.id.startsWith("measurement::")).length}</dd></div><div><dt>Dataset</dt><dd>{dataset.name}</dd></div></dl>
          <p className="nd-property-note">Select a construct or relationship to edit it. Press C to create a construct, Enter to edit a selection, and Delete to remove it.</p>
        </>}
        {preflight && readiness ? <section className={`nd-model-preflight ${preflight.status}`} aria-labelledby="nd-model-preflight-heading">
          <header><strong id="nd-model-preflight-heading">Model preflight</strong><span>{preflight.status === "blocked" ? "Needs attention" : preflight.status === "warning" ? "Review" : "Ready"}</span></header>
          <p className="nd-model-preflight-summary" role="status" aria-live="polite" aria-atomic="true"><strong>{preflight.headline}</strong>{preflight.detail}</p>
          <div className="nd-model-workload" aria-labelledby="nd-model-workload-heading">
            <strong id="nd-model-workload-heading">Workload preview</strong>
            <p>{preflight.calculationPlan}</p>
            <dl><div><dt>Cases</dt><dd>{(dataset.rowCount ?? dataset.rows.length).toLocaleString("en-US")}</dd></div><div><dt>Constructs</dt><dd>{nodes.length}</dd></div><div><dt>Assigned indicators</dt><dd>{assignedIndicatorCount}</dd></div></dl>
          </div>
          {mode === "expert" ? <details><summary>All preflight checks ({readiness.items.length})</summary><ul>{readiness.items.map((item) => <li key={item.id} className={item.status}><strong>{item.label}: {item.status === "blocked" ? "Fix required" : item.status === "warning" ? "Review" : "Ready"}</strong><span>{item.detail}</span></li>)}</ul></details> : null}
        </section> : null}
        {mode === "expert" && strictAuthority ? <StandardSemModelV4AdvancedEditor
          authority={strictAuthority}
          commit={commitAuthority}
        /> : null}
      </form> : null}

      {activeTab === "parameter" ? <form className="nd-property-form" onSubmit={(event) => event.preventDefault()}>
        {selectedInteraction ? <>
          <dl className="nd-property-list"><div><dt>Parameter</dt><dd>{interactionMethodLabel(selectedInteraction)}</dd></div><div><dt>Manifest indicators</dt><dd>Not applicable</dd></div></dl>
          <p className="nd-property-note">The interaction parameter is derived from its ordered operand scores.</p>
        </> : selected?.data.semantic === "higher_order" && selected.data.higherOrder ? <>
          <dl className="nd-property-list"><div><dt>Method</dt><dd>{higherOrderApproachLabel(selected.data.higherOrder)}</dd></div><div><dt>Indicators</dt><dd>Generated component scores</dd></div></dl>
          <p className="nd-property-note">The higher-order construct remains indicator-free in the editable model.</p>
        </> : selected ? <>
          <fieldset><legend>Measurement model</legend><label><input type="radio" checked={selected.data.mode === "reflective"} onChange={() => setMeasurementMode("reflective")} />Reflective</label><label><input type="radio" checked={selected.data.mode === "formative"} onChange={() => setMeasurementMode("formative")} />Formative</label></fieldset>
          {mode === "expert" ? <dl className="nd-property-list"><div><dt>Stable construct ID</dt><dd>{selected.id}</dd></div><div><dt>Bound indicators</dt><dd>{selected.data.indicators.length}</dd></div></dl> : null}
          {mode === "expert" && constructRepresentationAuthoringEnabled ? <NativeSemConstructAuthoringFields node={selected} onCommit={setScientificRepresentation} /> : constructRepresentationAuthoringEnabled ? <p className="nd-property-note">Switch to Expert to edit factor/composite representation and identification.</p> : null}
        </> : selectedPath ? <>
          <label>Relationship type<select value={pathRole} disabled={selectedPathSupportsModeration} aria-describedby={selectedPathSupportsModeration ? "nd-moderation-path-lock" : undefined} onChange={(event) => setPathRole(event.target.value as NativePathRole)}><option value="structural">Structural path</option><option value="control">Control path</option><option value="covariance">Covariance</option></select></label>
          {mode === "expert" ? <dl className="nd-property-list"><div><dt>Stable relationship ID</dt><dd>{selectedPath.id}</dd></div><div><dt>Endpoint IDs</dt><dd>{selectedPath.source} → {selectedPath.target}</dd></div></dl> : null}
          {selectedPathSupportsModeration ? <p id="nd-moderation-path-lock" className="nd-property-note">The current moderating effect requires this relationship type.</p> : null}
          {mode === "expert" && experimentalSemAuthoringEnabled && pathRole === "covariance" ? <NativeSemCovarianceAuthoringFields edge={selectedPath} nodes={nodes} edges={edges} onCommit={(authoredEdge) => {
            if (!strictAuthority) updateEdge(selectedPath.id, { data: authoredEdge.data });
            else {
              const current = relationshipDefinition(strictAuthority, selectedPath, "covariance");
              if (current) void commitAuthority({ kind: "replace_relationship", relationship_id: selectedPath.id, definition: authoredCovarianceDefinition(authoredEdge, current) });
            }
          }} /> : experimentalSemAuthoringEnabled && pathRole === "covariance" ? <p className="nd-property-note">Switch to Expert to define the scientific covariance use and endpoints.</p> : null}
        </> : <div className="nd-pane-empty">Select a construct or relationship to inspect its parameters.</div>}
      </form> : null}

      {activeTab === "appearance" ? <form className="nd-property-form" onSubmit={(event) => event.preventDefault()}>
        {selected ? <>
          <dl className="nd-property-list"><div><dt>Canvas X</dt><dd>{Math.round(selected.position.x)}</dd></div><div><dt>Canvas Y</dt><dd>{Math.round(selected.position.y)}</dd></div></dl>
          <p className="nd-property-note">Move the construct with the canvas, keyboard focus, or Arrange command. Its saved position is presentation-only.</p>
        </> : selectedPath ? <>
          <label>Routing<select value={routing} onChange={(event) => setSelectedPathRouting(event.target.value === "orthogonal" ? "smoothstep" : event.target.value === "curved" ? "default" : "straight")}><option value="straight">Straight</option><option value="curved">Curved</option><option value="orthogonal">Orthogonal</option></select></label>
          <p className="nd-property-note">Routing changes presentation only; it does not change the scientific relationship.</p>
        </> : <div className="nd-pane-empty">Select an object to inspect its presentation settings.</div>}
      </form> : null}

      {activeTab === "data-binding" ? <form className="nd-property-form" onSubmit={(event) => event.preventDefault()}>
        {selected && selected.data.semantic !== "interaction" && selected.data.semantic !== "higher_order" ? <>
          <label>Assign dataset variable<select value="" onChange={(event) => assignDatasetIndicator(event.target.value)}><option value="">Choose variable…</option>{availableIndicators.map((indicator) => <option key={indicator} value={indicator}>{indicator}</option>)}</select></label>
          <div className="nd-binding-list" aria-label={`Variables bound to ${selected.data.label}`}>
            {selected.data.indicators.map((indicator) => <div key={indicator}><span>{indicator}</span><button type="button" aria-label={`Remove ${indicator} from ${selected.data.label}`} onClick={() => removeDatasetIndicator(indicator)}>Remove</button></div>)}
            {!selected.data.indicators.length ? <p>No variables are bound to this construct.</p> : null}
          </div>
        </> : selected ? <p className="nd-property-note">{selected.data.semantic === "interaction" ? "Generated interactions do not accept manifest indicators." : "Higher-order constructs use generated component scores and do not accept manifest indicators."}</p> : selectedPath ? <p className="nd-property-note">Relationships do not bind directly to dataset variables.</p> : <dl className="nd-property-list"><div><dt>Dataset</dt><dd>{dataset.name}</dd></div><div><dt>Input</dt><dd>{dataset.kind}</dd></div><div><dt>Variables</dt><dd>{dataset.columns.length}</dd></div><div><dt>Cases</dt><dd>{dataset.rows.length}</dd></div></dl>}
      </form> : null}
    </section>
    {selected || selectedPath ? <div className="nd-property-actions nd-inspector-object-actions">
      {selectedPath ? <button type="button" disabled={selectedPathSupportsModeration} onClick={reversePath}>Reverse</button> : null}
      <button type="button" className="danger" onClick={deleteSelection}>{selectedPathSupportsModeration ? "Delete relationship and interaction" : selected?.data.semantic === "interaction" ? "Delete interaction" : selected?.data.semantic === "higher_order" ? "Delete higher-order construct" : selected ? "Delete construct" : "Delete relationship"}</button>
    </div> : null}
    </fieldset>
    {authorityFeedback ? <p className={`nd-authority-feedback ${authorityFeedback.tone}`} role={authorityFeedback.tone === "blocked" || authorityFeedback.tone === "rejected" ? "alert" : "status"} aria-live="polite">{authorityFeedback.message}</p> : null}
  </aside>;
}
