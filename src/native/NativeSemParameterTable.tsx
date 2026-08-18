import type { Edge, Node } from "@xyflow/react";
import { AlertTriangle, ArrowUpRight, CheckCircle2, Pencil } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import {
  projectNativeWorkbenchSemParameterTableV4,
  projectSemModelV4ParameterTable,
  type SemParameterTableRowV4,
  type SemParameterTableSourceV4,
} from "../domain/semParameterTableV4";
import {
  adaptAuthoredNativeWorkbenchToSemModelV4,
  type AuthoredNativeWorkbenchToSemModelV4Input,
  type NativeWorkbenchObservedSemanticsV4,
  type NativeWorkbenchSemModelV4Trace,
} from "../domain/nativeWorkbenchSemModelV4Adapter";
import type { StandardSemModelV4EditorIntentV1 } from "../domain/standardSemModelV4Authority";
import { compareUtf8StringsV1, type SemDataBindingV4, type SemModelV4, type SemParameterV4, type SemVariableV4 } from "../domain/semModelV4";
import {
  nativeSemLatentMeanEntryV4,
  nativeSemLatentMeanParameterIdV4,
  nativeSemObservedInterceptEntryV4,
  nativeSemObservedInterceptParameterIdV4,
  nativeSemOrdinalThresholdEntriesV4,
  parameterEntryFromSemParameterV4,
  withNativeSemFactorIdentificationV4,
  withNativeSemParameterEntriesOnConstructV4,
  withNativeSemParameterEntryOnConstructV4,
  withNativeSemParameterEntryOnEdgeV4,
} from "../domain/semModelV4ParameterAuthoring";
import { useWorkspace, type StandardSemModelV4AuthorityCommitResult } from "../store";
import type {
  ConstructData,
  Dataset,
  SemModelV4ParameterAuthoringEntry,
  SemModelV4ParameterAuthoringSpecification,
} from "../types";
import {
  NativeSemParameterEditor,
  NativeSemVariableEditor,
  type NativeSemParameterEditPolicy,
  type NativeSemVariableAuthoringDraft,
} from "./NativeSemParameterEditor";

const SECTION_LABELS: Readonly<Record<SemParameterTableRowV4["section"], string>> = {
  diagnostic: "Decisions needed",
  variable: "Variables",
  relation: "Relations",
  parameter: "Parameters",
  constraint: "Constraints",
  derived_term: "Derived terms",
  group: "Groups",
  annotation: "Annotations",
  presentation: "Presentation objects",
};

export interface NativeSemParameterTableProps {
  modelName: string;
  onShowCanvas: () => void;
}

export function NativeSemParameterTable({ modelName, onShowCanvas }: NativeSemParameterTableProps) {
  const activeModelId = useWorkspace((state) => state.activeModelId);
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const diagramLayout = useWorkspace((state) => state.diagramLayout);
  const dataset = useWorkspace((state) => state.dataset);
  const setSelectedNode = useWorkspace((state) => state.setSelectedNode);
  const setSelectedEdge = useWorkspace((state) => state.setSelectedEdge);
  const updateConstruct = useWorkspace((state) => state.updateConstruct);
  const updateEdge = useWorkspace((state) => state.updateEdge);
  const strictAuthority = useWorkspace((state) => state.activeModelId
    ? state.standardSemModelV4Authorities[state.activeModelId] ?? null
    : null);
  const commitStandardIntent = useWorkspace((state) => state.commitStandardSemModelV4Intent);
  const [editingRowId, setEditingRowId] = useState<string | null>(null);
  const [editError, setEditError] = useState<string | null>(null);
  const [authorityStatus, setAuthorityStatus] = useState<string | null>(null);

  const authoringInput = useMemo<AuthoredNativeWorkbenchToSemModelV4Input>(() => ({
    model_id: activeModelId ?? "",
    model_name: modelName,
    nodes,
    edges,
    diagram_layout: diagramLayout,
    data_binding: semDataBindingForParameterTable(dataset),
    group: { kind: "single_group" },
    observed_semantics: observedSemanticsForParameterTable(dataset, nodes.flatMap((node) => node.data.indicators)),
  }), [activeModelId, dataset, diagramLayout, edges, modelName, nodes]);
  const strictTrace = useMemo(() => strictAuthority ? strictAuthorityTrace(strictAuthority.model, nodes, edges) : null, [edges, nodes, strictAuthority]);
  const projection = useMemo(() => strictAuthority && strictTrace
    ? projectSemModelV4ParameterTable(strictAuthority.model, strictTrace)
    : projectNativeWorkbenchSemParameterTableV4(authoringInput), [authoringInput, strictAuthority, strictTrace]);
  const adapted = useMemo(() => adaptAuthoredNativeWorkbenchToSemModelV4(authoringInput), [authoringInput]);
  const editingRow = useMemo(() => projection.rows.find((row) => row.id === editingRowId) ?? null, [editingRowId, projection.rows]);
  const editingModel = strictAuthority?.model ?? (adapted.ok ? adapted.model : null);
  const editingParameter = editingRow?.section === "parameter" && editingRow.parameter_id && editingModel
    ? editingModel.parameters.find((parameter) => parameter.id === editingRow.parameter_id) ?? null
    : null;
  const editingVariable = editingRow?.section === "variable" && editingRow.sem_id && editingModel
    ? editingModel.variables.find((variable) => variable.id === editingRow.sem_id) ?? null
    : null;

  useEffect(() => {
    if (editingRowId && !editingRow) setEditingRowId(null);
  }, [editingRow, editingRowId]);

  const groups = useMemo(() => {
    const output: Array<{ section: SemParameterTableRowV4["section"]; rows: readonly SemParameterTableRowV4[] }> = [];
    for (const row of projection.rows) {
      const current = output.at(-1);
      if (current?.section === row.section) current.rows = [...current.rows, row];
      else output.push({ section: row.section, rows: [row] });
    }
    return output;
  }, [projection.rows]);

  const focusSource = (source: SemParameterTableSourceV4) => {
    let eventName: "quickpls:focus-construct" | "quickpls:focus-edge" | null = null;
    let sourceId = source.id;
    if (source.kind === "construct") {
      setSelectedNode(source.id);
      eventName = "quickpls:focus-construct";
    } else if (source.kind === "edge") {
      setSelectedEdge(source.id);
      eventName = "quickpls:focus-edge";
    } else if (source.kind === "indicator") {
      const owner = nodes.find((node) => node.data.indicators.includes(source.id));
      if (!owner) return;
      sourceId = owner.id;
      setSelectedNode(owner.id);
      eventName = "quickpls:focus-construct";
    } else return;
    onShowCanvas();
    window.setTimeout(() => {
      window.dispatchEvent(new CustomEvent(eventName!, { detail: { id: sourceId } }));
    }, 0);
  };

  const beginEdit = (row: SemParameterTableRowV4) => {
    setEditError(null);
    setEditingRowId(row.id);
  };

  const closeEditor = () => {
    const triggerId = editingRowId ? editButtonId(editingRowId) : null;
    setEditingRowId(null);
    setEditError(null);
    if (triggerId) window.setTimeout(() => document.getElementById(triggerId)?.focus(), 0);
  };

  const commitStrictIntent = async (intent: StandardSemModelV4EditorIntentV1) => {
    setEditError(null);
    setAuthorityStatus("Committing strict Standard parameter edit…");
    const result = await commitStandardIntent(intent);
    const message = strictCommitMessage(result);
    setAuthorityStatus(message);
    if (result.status === "committed") closeEditor();
    else setEditError(message);
  };

  const commitParameter = (parameter: Exclude<SemParameterV4, { kind: "derived" }>, specification: SemModelV4ParameterAuthoringSpecification | null) => {
    if (strictAuthority) {
      void commitStrictIntent(specification === null
        ? { kind: "restore_parameter", parameter_id: parameter.id }
        : { kind: "set_parameter_specification", parameter_id: parameter.id, specification, label: parameter.label });
      return;
    }
    const owner = parameterOwner(parameter, editingRow?.source ?? null, nodes, edges);
    if (!owner) {
      setEditError("This parameter no longer has an editable source. Reopen the Parameter Table after correcting the model object.");
      return;
    }
    try {
      const entry = specification === null ? null : { ...parameterEntryFromSemParameterV4(parameter), specification };
      if (owner.kind === "construct") {
        const next = withNativeSemParameterEntryOnConstructV4(owner.node, entry, parameter.id);
        const candidate = adaptAuthoredNativeWorkbenchToSemModelV4({
          ...authoringInput,
          nodes: nodes.map((node) => node.id === next.id ? next : node),
        });
        if (!candidate.ok) {
          setEditError(adapterDiagnosticMessage(candidate.diagnostics));
          return;
        }
        updateConstruct(owner.node.id, { semModelV4: next.data.semModelV4 });
      } else {
        const next = withNativeSemParameterEntryOnEdgeV4(owner.edge, entry, parameter.id);
        const candidate = adaptAuthoredNativeWorkbenchToSemModelV4({
          ...authoringInput,
          edges: edges.map((edge) => edge.id === next.id ? next : edge),
        });
        if (!candidate.ok) {
          setEditError(adapterDiagnosticMessage(candidate.diagnostics));
          return;
        }
        updateEdge(owner.edge.id, { data: next.data });
      }
      closeEditor();
    } catch (error) {
      setEditError(authoringErrorMessage(error));
    }
  };

  const commitVariable = (variable: Extract<SemVariableV4, { kind: "common_factor" | "observed" }>, draft: NativeSemVariableAuthoringDraft) => {
    if (strictAuthority) {
      const intents: StandardSemModelV4EditorIntentV1[] = [];
      if (variable.kind === "common_factor" && draft.kind === "common_factor") {
        if (JSON.stringify(variable.identification) !== JSON.stringify(draft.identification)) intents.push({
          kind: "set_factor_identification",
          variable_id: variable.id,
          identification: draft.identification,
        });
        if ((variable.mean_policy.kind !== "fixed_zero") !== draft.estimate_latent_mean) intents.push({
          kind: "set_latent_mean",
          variable_id: variable.id,
          estimated: draft.estimate_latent_mean,
        });
      } else if (variable.kind === "observed" && draft.kind === "observed") {
        const hasIntercept = strictAuthority.model.parameters.some((parameter) => parameter.target.kind === "intercept" && parameter.target.variable === variable.id);
        const hasThresholds = strictAuthority.model.parameters.some((parameter) => parameter.target.kind === "threshold" && parameter.target.variable === variable.id);
        if (hasIntercept !== draft.estimate_intercept) intents.push({ kind: "set_observed_intercept", variable_id: variable.id, estimated: draft.estimate_intercept });
        if (hasThresholds !== draft.estimate_thresholds) intents.push({ kind: "set_ordinal_thresholds", variable_id: variable.id, estimated: draft.estimate_thresholds });
      }
      if (intents.length !== 1) {
        setEditError(intents.length === 0
          ? "No strict authority change was selected."
          : "Apply one strict variable decision at a time so the editor emits one atomic authority intent.");
        return;
      }
      void commitStrictIntent(intents[0]);
      return;
    }
    const owner = variableOwner(variable, nodes);
    if (!owner) {
      setEditError("This variable no longer has an editable construct. Restore its data binding and try again.");
      return;
    }
    try {
      let next = owner;
      const entries: SemModelV4ParameterAuthoringEntry[] = [];
      const removeIds: string[] = [];
      if (variable.kind === "common_factor" && draft.kind === "common_factor") {
        next = withNativeSemFactorIdentificationV4(next, draft.identification);
        if (draft.estimate_latent_mean) {
          if (!hasStoredParameter(next, nativeSemLatentMeanParameterIdV4(owner.id))) entries.push(nativeSemLatentMeanEntryV4(owner.id));
        } else removeIds.push(nativeSemLatentMeanParameterIdV4(owner.id));
      } else if (variable.kind === "observed" && draft.kind === "observed") {
        const indicator = variable.source_column;
        if (draft.estimate_intercept) {
          if (!hasStoredParameter(next, nativeSemObservedInterceptParameterIdV4(indicator))) entries.push(nativeSemObservedInterceptEntryV4(indicator));
        } else removeIds.push(nativeSemObservedInterceptParameterIdV4(indicator));
        const thresholdIds = thresholdParameterIds(next, variable.id);
        if (draft.estimate_thresholds) entries.push(...nativeSemOrdinalThresholdEntriesV4(indicator, variable.categories.length)
          .filter((entry) => !hasStoredParameter(next, entry.parameter_id)));
        else removeIds.push(...thresholdIds);
      } else throw new Error("The variable editor changed while it was open. Reopen it and try again.");
      next = withNativeSemParameterEntriesOnConstructV4(next, entries, removeIds);
      const candidate = adaptAuthoredNativeWorkbenchToSemModelV4({
        ...authoringInput,
        nodes: nodes.map((node) => node.id === next.id ? next : node),
      });
      if (!candidate.ok) {
        setEditError(adapterDiagnosticMessage(candidate.diagnostics));
        return;
      }
      updateConstruct(next.id, { semModelV4: next.data.semModelV4 });
      closeEditor();
    } catch (error) {
      setEditError(authoringErrorMessage(error));
    }
  };

  return <section
    id="nd-model-parameter-panel"
    className="nd-sem-parameter-pane"
    role="tabpanel"
    aria-labelledby="nd-model-parameter-tab"
    tabIndex={0}
  >
    <header className="nd-sem-parameter-header">
      <div>
        <h3>Parameter Table <span className="nd-experimental-chip">Experimental</span></h3>
        <p>Review and edit the variables, relationships, parameters, and visual-only objects in the experimental SEM model. These edits are not sent to the current calculation engine.</p>
      </div>
      <dl aria-label="Parameter table summary">
        <div><dt>Scientific</dt><dd>{projection.counts.scientific}</dd></div>
        <div><dt>Presentation</dt><dd>{projection.counts.presentation}</dd></div>
        <div><dt>Needs attention</dt><dd>{projection.counts.unresolved}</dd></div>
      </dl>
    </header>
    <div className={`nd-sem-parameter-status ${projection.status === "ready" ? "ready" : "attention"}`} role="status" aria-live="polite">
      {projection.status === "ready" ? <CheckCircle2 size={14} aria-hidden="true" /> : <AlertTriangle size={14} aria-hidden="true" />}
      <span>{projection.status === "ready"
        ? "The table reflects the complete authored SEM model."
        : "Complete the listed model decisions before a scientific parameter table can be created."}</span>
    </div>
    {authorityStatus ? <p className="nd-authority-feedback" role="status" aria-live="polite">{authorityStatus}</p> : null}
    {editError ? <p className="nd-sem-editor-errors" role="alert">{editError}</p> : null}
    {editingParameter && editingParameter.kind !== "derived" ? <NativeSemParameterEditor
      key={editingParameter.id}
      parameter={editingParameter}
      canRestore={hasAuthoredParameter(editingParameter.id, nodes, edges)}
      policy={parameterEditPolicy(editingParameter, editingModel?.variables ?? [])}
      onApply={(specification) => commitParameter(editingParameter, specification)}
      onRestore={() => commitParameter(editingParameter, null)}
      onClose={closeEditor}
    /> : editingVariable && (editingVariable.kind === "common_factor" || editingVariable.kind === "observed") ? <NativeSemVariableEditor
      key={editingVariable.id}
      variable={editingVariable}
      indicators={variableOwner(editingVariable, nodes)?.data.indicators ?? []}
      hasLatentMean={editingVariable.kind === "common_factor" && editingVariable.mean_policy.kind !== "fixed_zero"}
      hasIntercept={Boolean(editingModel?.parameters.some((parameter) => parameter.target.kind === "intercept" && parameter.target.variable === editingVariable.id))}
      hasThresholds={Boolean(editingModel?.parameters.some((parameter) => parameter.target.kind === "threshold" && parameter.target.variable === editingVariable.id))}
      onApply={(draft) => commitVariable(editingVariable, draft)}
      onClose={closeEditor}
    /> : null}
    <div className="nd-sem-parameter-scroll" tabIndex={0} aria-label="Scrollable parameter table">
      <table className="nd-sem-parameter-table">
        <caption className="nd-sr-only">SEM objects for {modelName}; source IDs link scientific rows back to the model canvas.</caption>
        <thead><tr><th scope="col">Object</th><th scope="col">Use</th><th scope="col">Name</th><th scope="col">Specification</th><th scope="col">Source ID</th><th scope="col">Edit</th></tr></thead>
        {groups.map((group) => <tbody key={group.section} aria-label={SECTION_LABELS[group.section]}>
          <tr className="nd-sem-section-row"><th scope="rowgroup" colSpan={6}>{SECTION_LABELS[group.section]} <span>{group.rows.length}</span></th></tr>
          {group.rows.map((row) => <tr key={row.id} className={row.classification === "unresolved" ? "needs-attention" : undefined}>
            <td>{humanToken(row.object_kind)}</td>
            <td><span className={`nd-sem-use nd-sem-use-${row.classification}`}>{classificationLabel(row.classification)}</span></td>
            <th scope="row">{row.label}</th>
            <td>{row.specification}</td>
            <td>{isFocusableSource(row.source, nodes)
              ? <button type="button" onClick={() => focusSource(row.source)} aria-label={`Show ${sourceLabel(row.source)} on the model canvas`}>
                <code>{row.source.id}</code><ArrowUpRight size={12} aria-hidden="true" />
              </button>
              : <code>{row.source.id}</code>}</td>
            <td>{isEditableRow(row, editingModel?.parameters ?? [], editingModel?.variables ?? [])
              ? <button id={editButtonId(row.id)} type="button" className="nd-sem-edit-button" onClick={() => beginEdit(row)} aria-label={`Edit ${row.label}`} aria-expanded={editingRowId === row.id} aria-controls="nd-sem-parameter-editor">
                <Pencil size={12} aria-hidden="true" /> Edit
              </button>
              : <span className="nd-sem-not-editable">—</span>}</td>
          </tr>)}
        </tbody>)}
      </table>
    </div>
  </section>;
}

export function semDataBindingForParameterTable(dataset: Dataset): SemDataBindingV4 {
  if (dataset.kind === "covariance" || dataset.kind === "correlation") return {
    kind: dataset.kind,
    dataset_id: dataset.id,
    variables: [...dataset.columns],
    means: null,
    standard_deviations: null,
    sample: {
      sample_size: dataset.sampleSize ?? 0,
      covariance_denominator: "sample_n_minus_one",
    },
  };
  return {
    kind: "raw",
    dataset_id: dataset.id,
    missing_data: "listwise_deletion",
    weight: null,
    cluster_variable: null,
    strata_variable: null,
  };
}

export function observedSemanticsForParameterTable(
  dataset: Dataset,
  usedIndicators: readonly string[],
): Record<string, NativeWorkbenchObservedSemanticsV4> {
  const used = new Set(usedIndicators);
  const metadata = new Map((dataset.columnMetadata ?? []).map((column) => [column.name, column]));
  return Object.fromEntries([...used].sort().flatMap((column) => {
    const details = metadata.get(column);
    if (!details) return [];
    return [[column, {
      label: details.label?.trim() || column,
      scale: details.scale_type,
      role: "indicator" as const,
      categories: Object.keys(details.value_labels).sort(),
      value_labels: { ...details.value_labels },
      missing_markers: [...new Set(details.missing_markers
        .map((marker) => marker.trim())
        .filter(Boolean))].sort(compareUtf8StringsV1),
      transformation_lineage: [],
    } satisfies NativeWorkbenchObservedSemanticsV4]];
  }));
}

function isFocusableSource(source: SemParameterTableSourceV4, nodes: readonly { id: string; data: { indicators: string[] } }[]): boolean {
  if (source.kind === "construct" || source.kind === "edge") return true;
  return source.kind === "indicator" && nodes.some((node) => node.data.indicators.includes(source.id));
}

function classificationLabel(classification: SemParameterTableRowV4["classification"]): string {
  if (classification === "scientific") return "Scientific";
  if (classification === "presentation") return "Presentation only";
  return "Decision needed";
}

function sourceLabel(source: SemParameterTableSourceV4): string {
  if (source.kind === "edge") return `relationship ${source.id}`;
  if (source.kind === "construct") return `construct ${source.id}`;
  if (source.kind === "indicator") return `indicator ${source.id}`;
  return source.id;
}

type ParameterOwner =
  | { kind: "construct"; node: Node<ConstructData> }
  | { kind: "edge"; edge: Edge };

function parameterOwner(
  parameter: SemParameterV4,
  source: SemParameterTableSourceV4 | null,
  nodes: readonly Node<ConstructData>[],
  edges: readonly Edge[],
): ParameterOwner | null {
  if (source?.kind === "edge") {
    const edge = edges.find((candidate) => candidate.id === source.id);
    return edge ? { kind: "edge", edge } : null;
  }
  if (source?.kind === "construct") {
    const node = nodes.find((candidate) => candidate.id === source.id);
    return node ? { kind: "construct", node } : null;
  }
  if (source?.kind === "indicator") {
    const node = nodes.find((candidate) => candidate.data.indicators.includes(source.id));
    return node ? { kind: "construct", node } : null;
  }
  const target = parameter.target;
  const constructId = target.kind === "loading" ? constructSource(target.construct)
    : target.kind === "weight" ? constructSource(target.composite)
      : target.kind === "variance" ? variableOwnerId(target.endpoint.id, nodes)
        : target.kind === "intercept" || target.kind === "mean" || target.kind === "threshold" ? variableOwnerId(target.variable, nodes)
          : null;
  const node = constructId ? nodes.find((candidate) => candidate.id === constructId) : null;
  return node ? { kind: "construct", node } : null;
}

function variableOwner(
  variable: Extract<SemVariableV4, { kind: "common_factor" | "observed" }>,
  nodes: readonly Node<ConstructData>[],
): Node<ConstructData> | null {
  const id = variable.kind === "common_factor"
    ? constructSource(variable.id)
    : nodes.find((node) => node.data.indicators.includes(variable.source_column))?.id ?? null;
  return id ? nodes.find((node) => node.id === id) ?? null : null;
}

function variableOwnerId(variableId: string, nodes: readonly Node<ConstructData>[]): string | null {
  const construct = constructSource(variableId);
  if (construct) return construct;
  const indicator = variableId.startsWith("observed:") ? variableId.slice("observed:".length) : variableId;
  return nodes.find((node) => node.data.indicators.includes(indicator))?.id ?? null;
}

function constructSource(variableId: string): string | null {
  return variableId.startsWith("construct:") ? variableId.slice("construct:".length) : null;
}

function hasAuthoredParameter(parameterId: string, nodes: readonly Node<ConstructData>[], edges: readonly Edge[]): boolean {
  if (nodes.some((node) => node.data.semModelV4?.parameters?.some((entry) => entry.parameter_id === parameterId))) return true;
  return edges.some((edge) => {
    const data = edge.data as { semModelV4?: { parameters?: SemModelV4ParameterAuthoringEntry[] }; semModelV4ParameterAuthoring?: { parameters?: SemModelV4ParameterAuthoringEntry[] } } | undefined;
    return data?.semModelV4?.parameters?.some((entry) => entry.parameter_id === parameterId)
      || data?.semModelV4ParameterAuthoring?.parameters?.some((entry) => entry.parameter_id === parameterId);
  });
}

function thresholdParameterIds(node: Node<ConstructData>, variableId: string): string[] {
  return (node.data.semModelV4?.parameters ?? []).flatMap((entry) => entry.target.kind === "threshold" && entry.target.variable === variableId
    ? [entry.parameter_id]
    : []);
}

function hasStoredParameter(node: Node<ConstructData>, parameterId: string): boolean {
  return Boolean(node.data.semModelV4?.parameters?.some((entry) => entry.parameter_id === parameterId));
}

function parameterEditPolicy(
  parameter: Exclude<SemParameterV4, { kind: "derived" }>,
  variables: readonly SemVariableV4[],
): NativeSemParameterEditPolicy {
  if (parameter.target.kind === "loading") {
    const target = parameter.target;
    const factor = variables.find((variable) => variable.id === target.construct);
    if (factor?.kind === "common_factor" && factor.identification.kind === "marker_loading" && factor.identification.indicator === target.indicator) return {
      managedMessage: "This loading is fixed at 1 by marker-loading identification. Change the factor identification from its Variable row to edit it.",
    };
    if (factor?.kind === "common_factor" && factor.identification.kind === "effects_coding") return {
      freeOnly: true,
    };
  }
  if (parameter.target.kind === "variance" && parameter.target.endpoint.kind === "variable") {
    const endpoint = parameter.target.endpoint;
    const factor = variables.find((variable) => variable.id === endpoint.id);
    if (factor?.kind === "common_factor" && factor.identification.kind === "fixed_variance") return {
      managedMessage: "This factor variance is fixed at 1 by fixed-variance identification. Change the factor identification from its Variable row to edit it.",
    };
  }
  if (parameter.target.kind === "mean") return { freeOnly: true };
  return {};
}

function isEditableRow(
  row: SemParameterTableRowV4,
  parameters: readonly SemParameterV4[],
  variables: readonly SemVariableV4[],
): boolean {
  if (row.section === "parameter" && row.parameter_id) return parameters.some((parameter) => parameter.id === row.parameter_id && parameter.kind !== "derived");
  if (row.section !== "variable" || !row.sem_id) return false;
  return variables.some((variable) => variable.id === row.sem_id && (variable.kind === "common_factor" || variable.kind === "observed"));
}

function authoringErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message.trim()) return error.message;
  return "The parameter edit could not be applied. Reopen the row and try again.";
}

function editButtonId(rowId: string): string {
  return `nd-sem-edit-${encodeURIComponent(rowId)}`;
}

function adapterDiagnosticMessage(diagnostics: readonly { message: string; corrective_action: string }[]): string {
  const first = diagnostics[0];
  return first ? `${first.message} ${first.corrective_action}` : "The parameter edit is not compatible with this model.";
}

function humanToken(value: string): string {
  const spaced = value.replaceAll("_", " ");
  return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}

function strictAuthorityTrace(
  model: SemModelV4,
  nodes: readonly Node<ConstructData>[],
  edges: readonly Edge[],
): NativeWorkbenchSemModelV4Trace {
  const edgeIds = new Set(edges.map((edge) => edge.id));
  return {
    construct_variables: Object.fromEntries(nodes.map((node) => [node.id, node.id])),
    indicator_variables: Object.fromEntries(model.variables.flatMap((variable) => variable.kind === "observed"
      ? [[variable.source_column, variable.id] as const]
      : [])),
    edge_objects: Object.fromEntries([
      ...model.relations.flatMap((relation) => relation.kind !== "measurement_effect" && relation.kind !== "measurement_causal" && edgeIds.has(relation.id)
        ? [[relation.id, { kind: "scientific_relation" as const, sem_id: relation.id, parameter_id: relation.parameter }] as const]
        : []),
      ...model.annotations.flatMap((annotation) => annotation.kind === "display_only_covariance" && edgeIds.has(annotation.id)
        ? [[annotation.id, { kind: "presentation_annotation" as const, sem_id: annotation.id }] as const]
        : []),
    ]),
  };
}

function strictCommitMessage(result: StandardSemModelV4AuthorityCommitResult): string {
  if (result.status === "committed") return "Committed to the strict Standard model authority.";
  if (result.status === "blocked") return `Blocked: ${result.diagnostic.message} ${result.diagnostic.correctiveAction}`;
  if (result.status === "stale") return "Stale edit ignored because the active authority changed.";
  return `Rejected: ${result.error instanceof Error ? result.error.message : String(result.error)}`;
}
