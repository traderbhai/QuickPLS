import type { Edge, Node } from "@xyflow/react";
import { useMemo, useState } from "react";
import type { HigherOrderConstructionApproachV4, HigherOrderMeasurementTypeV4 } from "../domain/semModelV4";
import type { ConstructData } from "../types";
import {
  DEFAULT_NATIVE_HIGHER_ORDER_APPROACH,
  DEFAULT_NATIVE_HIGHER_ORDER_MEASUREMENT_TYPE,
  NATIVE_HIGHER_ORDER_APPROACH_LABELS,
  type NativeHigherOrderConceptualDirection,
  type NativeHigherOrderDraft,
  type NativeHigherOrderDraftIssue,
  type NativeHigherOrderEditableApproach,
  isNativeStructuralEdge,
  nativeHigherOrderApproachOptions,
  nativeHigherOrderComponentCandidates,
  nativeHigherOrderConceptualDirection,
  nativeHigherOrderCreationBlocker,
  nativeHigherOrderDraftIssues,
  nativeHigherOrderMeasurementCode,
  nativeHigherOrderMeasurementType,
  nativeHigherOrderSelectedComponentMode,
  nativeHigherOrderSuggestedShortName,
} from "./nativeHigherOrder";

export type NativeHigherOrderDialogRequest =
  | { kind: "create"; selectedComponentIds?: readonly string[]; requireInitialPath?: boolean }
  | { kind: "edit"; constructId: string };

export type NativeHigherOrderDialogSubmission =
  | { kind: "create"; draft: NativeHigherOrderDraft }
  | {
      kind: "edit";
      constructId: string;
      outputId: string;
      termId: string;
      draft: NativeHigherOrderDraft;
    };

export type NativeHigherOrderDialogCommitResult =
  | { status: "applied"; constructId: string }
  | { status: "blocked"; detail: string };

export interface NativeHigherOrderDialogProps {
  nodes: readonly Node<ConstructData>[];
  edges: readonly Edge[];
  request: NativeHigherOrderDialogRequest;
  commit: (submission: NativeHigherOrderDialogSubmission) => NativeHigherOrderDialogCommitResult;
  close: () => void;
}

function canonicalApproach(node: Node<ConstructData> | null): HigherOrderConstructionApproachV4 {
  const declaration = node?.data.higherOrder;
  if (declaration?.canonicalApproach) return declaration.canonicalApproach;
  if (declaration?.method === "repeated_indicators") return "repeated_indicators";
  if (declaration?.method === "hybrid") return "hybrid";
  return DEFAULT_NATIVE_HIGHER_ORDER_APPROACH;
}

function canonicalMeasurementType(node: Node<ConstructData> | null): HigherOrderMeasurementTypeV4 {
  return node?.data.higherOrder?.measurementType ?? DEFAULT_NATIVE_HIGHER_ORDER_MEASUREMENT_TYPE;
}

function issueFor(
  issues: readonly NativeHigherOrderDraftIssue[],
  field: NativeHigherOrderDraftIssue["field"],
): string | null {
  return issues.find((issue) => issue.field === field)?.message ?? null;
}

function FieldError({ message }: { message: string | null }) {
  return message ? <div className="nd-form-error" role="alert">{message}</div> : null;
}

function ReadOnlyHigherOrder({ node, close }: { node: Node<ConstructData>; close: () => void }) {
  const declaration = node.data.higherOrder!;
  const measurementLabel = declaration.measurementType
    ? nativeHigherOrderMeasurementCode(declaration.measurementType)
    : "Legacy";
  return <div className="nd-dialog-form nd-higher-order-dialog">
    <dl className="nd-property-list nd-hoc-summary">
      <div><dt>Name</dt><dd>{node.data.label}</dd></div>
      <div><dt>Type</dt><dd>{measurementLabel}</dd></div>
      <div><dt>Approach</dt><dd>Hybrid</dd></div>
      <div><dt>Dimensions</dt><dd>{declaration.components.length}</dd></div>
    </dl>
    <p className="nd-dialog-note">This legacy hybrid HOC is readable but compatibility-only.</p>
    <footer><button type="button" onClick={close}>Close</button></footer>
  </div>;
}

export default function NativeHigherOrderDialog({ nodes, edges, request, commit, close }: NativeHigherOrderDialogProps) {
  const editingNode = useMemo(() => {
    if (request.kind !== "edit") return null;
    return nodes.find((node) => node.id === request.constructId)
      ?? nodes.find((node) => node.data.semantic === "higher_order" && node.data.higherOrder?.id === request.constructId)
      ?? null;
  }, [nodes, request]);
  const editingDeclaration = editingNode?.data.semantic === "higher_order"
    ? editingNode.data.higherOrder ?? null
    : null;
  const editingOutputId = editingNode?.id;
  const editingApproach = canonicalApproach(editingNode);
  const editingMeasurementType = canonicalMeasurementType(editingNode);
  const editingHigherOrderId = request.kind === "edit" ? editingOutputId : undefined;
  const baseCandidates = useMemo(
    () => nativeHigherOrderComponentCandidates(nodes, editingHigherOrderId),
    [editingHigherOrderId, nodes],
  );
  const candidateIds = useMemo(
    () => new Set(baseCandidates.filter((option) => option.eligible).map((option) => option.id)),
    [baseCandidates],
  );
  const requestedComponents = request.kind === "create"
    ? [...new Set(request.selectedComponentIds ?? [])].filter((id) => candidateIds.has(id))
    : editingDeclaration?.components ?? [];
  const initialPathConstruct = nodes.find((node) => (
    !node.data.semantic
    && node.data.indicators.length > 0
    && !requestedComponents.includes(node.id)
  ))?.id ?? "";

  const [name, setName] = useState(editingNode?.data.label ?? "Higher-order construct");
  const [shortNameOverride, setShortNameOverride] = useState<string | null>(
    request.kind === "edit" ? editingNode?.data.shortName ?? "" : null,
  );
  const [components, setComponents] = useState<string[]>([...requestedComponents]);
  const [conceptualDirection, setConceptualDirection] = useState<NativeHigherOrderConceptualDirection>(
    nativeHigherOrderConceptualDirection(editingMeasurementType),
  );
  const [approachOverride, setApproachOverride] = useState<NativeHigherOrderEditableApproach | null>(
    editingApproach === "hybrid" ? null : request.kind === "edit" ? editingApproach : null,
  );
  const [pathDirection, setPathDirection] = useState<"hoc_to_construct" | "construct_to_hoc">("hoc_to_construct");
  const [pathConstructId, setPathConstructId] = useState(initialPathConstruct);
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [submitted, setSubmitted] = useState(false);
  const [creationError, setCreationError] = useState<string | null>(null);

  if (request.kind === "edit" && (!editingNode || !editingDeclaration)) {
    return <div className="nd-dialog-form nd-higher-order-dialog">
      <div className="nd-form-error" role="alert">This higher-order construct is no longer available.</div>
      <footer><button type="button" onClick={close}>Close</button></footer>
    </div>;
  }
  if (request.kind === "edit" && editingNode && editingDeclaration && editingApproach === "hybrid") {
    return <ReadOnlyHigherOrder node={editingNode} close={close} />;
  }

  const selectedComponentMode = nativeHigherOrderSelectedComponentMode(components, nodes);
  const selectedModes = new Set(components
    .map((id) => nodes.find((node) => node.id === id)?.data.mode)
    .filter((mode): mode is ConstructData["mode"] => mode === "reflective" || mode === "formative"));
  const mixedComponentModes = selectedModes.size > 1;
  const measurementType = selectedComponentMode
    ? nativeHigherOrderMeasurementType(selectedComponentMode, conceptualDirection)
    : nativeHigherOrderMeasurementType("reflective", conceptualDirection);
  const structuralEdges = edges.filter(isNativeStructuralEdge);
  const hocIsEndogenous = request.kind === "create"
    ? request.requireInitialPath === true && pathConstructId
      ? pathDirection === "construct_to_hoc"
      : null
    : structuralEdges.some((edge) => edge.target === editingOutputId)
      ? true
      : structuralEdges.some((edge) => edge.source === editingOutputId)
        ? false
        : null;
  const approachOptions = nativeHigherOrderApproachOptions({
    nodes,
    edges,
    components,
    measurementType: selectedComponentMode ? measurementType : null,
    hocIsEndogenous,
    editingHigherOrderId,
  });
  const recommendedApproach = approachOptions.find((option) => option.recommended)?.approach ?? null;
  const approach = approachOverride ?? recommendedApproach ?? DEFAULT_NATIVE_HIGHER_ORDER_APPROACH;
  const selectedApproach = approachOptions.find((option) => option.approach === approach) ?? null;
  const shortName = shortNameOverride ?? nativeHigherOrderSuggestedShortName(name, nodes, editingHigherOrderId);
  const requireInitialPath = request.kind === "create" && request.requireInitialPath === true;
  const draft: NativeHigherOrderDraft = {
    name,
    shortName,
    components,
    approach,
    measurementType,
    ...(requireInitialPath && pathConstructId
      ? { initialPath: { direction: pathDirection, constructId: pathConstructId } }
      : {}),
  };
  const issues = nativeHigherOrderDraftIssues(draft, nodes, edges, {
    editingHigherOrderId,
    requireInitialPath,
    hocIsEndogenous,
  });
  const visibleIssues = submitted ? issues : [];
  const blocker = request.kind === "create" ? nativeHigherOrderCreationBlocker(nodes, edges) : null;
  const pathOptions = nodes.filter((node) => (
    !node.data.semantic
    && node.data.indicators.length > 0
    && !components.includes(node.id)
  ));
  const dimensionLabel = selectedComponentMode === "reflective"
    ? "Reflective (Mode A)"
    : selectedComponentMode === "formative"
      ? "Formative (Mode B)"
      : "Choose dimensions";

  return <form
    className="nd-dialog-form nd-higher-order-dialog"
    onSubmit={(event) => {
      event.preventDefault();
      setSubmitted(true);
      setCreationError(null);
      if (issues.some((issue) => issue.field === "approach") || !selectedApproach?.valid) setAdvancedOpen(true);
      if (blocker || issues.length || !selectedApproach?.valid) return;
      const result = request.kind === "create"
        ? commit({ kind: "create", draft })
        : commit({
            kind: "edit",
            constructId: request.constructId,
            outputId: editingNode!.id,
            termId: editingDeclaration!.id,
            draft,
          });
      if (result.status === "applied") close();
      else setCreationError(result.detail);
    }}
  >
    <label htmlFor="nd-hoc-name">Name
      <input
        id="nd-hoc-name"
        autoFocus
        value={name}
        aria-invalid={submitted && Boolean(issueFor(issues, "name"))}
        onChange={(event) => {
          setName(event.target.value);
          setSubmitted(false);
          setCreationError(null);
        }}
      />
    </label>
    <FieldError message={issueFor(visibleIssues, "name")} />

    <fieldset className="nd-project-mode-options">
      <legend>Conceptual direction</legend>
      <label>
        <input
          type="radio"
          name="nd-hoc-direction"
          value="hoc_explains_components"
          checked={conceptualDirection === "hoc_explains_components"}
          onChange={() => {
            setConceptualDirection("hoc_explains_components");
            setSubmitted(false);
            setCreationError(null);
          }}
        />
        <span>HOC explains its dimensions</span>
      </label>
      <label>
        <input
          type="radio"
          name="nd-hoc-direction"
          value="components_form_hoc"
          checked={conceptualDirection === "components_form_hoc"}
          onChange={() => {
            setConceptualDirection("components_form_hoc");
            setSubmitted(false);
            setCreationError(null);
          }}
        />
        <span>Dimensions form the HOC</span>
      </label>
    </fieldset>
    <FieldError message={issueFor(visibleIssues, "direction")} />

    <fieldset className="nd-hoc-components">
      <legend>Dimensions</legend>
      {baseCandidates.map((option) => {
        const checked = components.includes(option.id);
        const modeMismatch = Boolean(selectedComponentMode && option.mode !== selectedComponentMode);
        const selectionLocked = !checked && (mixedComponentModes || modeMismatch);
        const disabled = !checked && (!option.eligible || selectionLocked);
        const reason = option.reason
          ?? (selectionLocked
            ? `Choose ${option.mode === "reflective" ? "Mode A" : "Mode B"} dimensions in a separate HOC.`
            : null);
        return <label key={option.id} className={disabled ? "checkbox-row disabled" : "checkbox-row"}>
          <input
            type="checkbox"
            checked={checked}
            disabled={disabled || Boolean(blocker)}
            onChange={(event) => {
              setSubmitted(false);
              setCreationError(null);
              setComponents((current) => event.target.checked
                ? [...current, option.id]
                : current.filter((component) => component !== option.id));
            }}
          />
          <span>
            {option.label} [{option.shortName}]
            <small>{option.mode === "reflective" ? "Mode A" : "Mode B"}{reason ? ` · ${reason}` : ""}</small>
          </span>
        </label>;
      })}
    </fieldset>
    <FieldError message={issueFor(visibleIssues, "components")} />

    {requireInitialPath ? <fieldset className="nd-hoc-initial-path">
      <legend>Initial model path</legend>
      <label htmlFor="nd-hoc-path-direction">Direction
        <select id="nd-hoc-path-direction" value={pathDirection} onChange={(event) => {
          setPathDirection(event.target.value as "hoc_to_construct" | "construct_to_hoc");
          setSubmitted(false);
          setCreationError(null);
        }}>
          <option value="hoc_to_construct">HOC → construct</option>
          <option value="construct_to_hoc">Construct → HOC</option>
        </select>
      </label>
      <label htmlFor="nd-hoc-path-construct">Construct
        <select id="nd-hoc-path-construct" value={pathConstructId} onChange={(event) => {
          setPathConstructId(event.target.value);
          setSubmitted(false);
          setCreationError(null);
        }}>
          <option value="">Choose a construct</option>
          {pathOptions.map((node) => <option key={node.id} value={node.id}>{node.data.label} [{node.data.shortName}]</option>)}
        </select>
      </label>
    </fieldset> : null}
    <FieldError message={issueFor(visibleIssues, "initial_path")} />

    <dl className="nd-property-list nd-hoc-summary">
      <div><dt>Type</dt><dd>{selectedComponentMode ? nativeHigherOrderMeasurementCode(measurementType) : "Choose dimensions"}</dd></div>
      <div><dt>Approach</dt><dd>{NATIVE_HIGHER_ORDER_APPROACH_LABELS[approach]}{selectedApproach?.recommended ? " (Recommended)" : ""}</dd></div>
      <div><dt>Dimensions</dt><dd>{dimensionLabel}</dd></div>
    </dl>

    <details open={advancedOpen} onToggle={(event) => setAdvancedOpen(event.currentTarget.open)}>
      <summary>Advanced</summary>
      <label htmlFor="nd-hoc-short-name">Legacy short code
        <input
          id="nd-hoc-short-name"
          value={shortName}
          maxLength={12}
          onChange={(event) => {
            setShortNameOverride(event.target.value);
            setSubmitted(false);
            setCreationError(null);
          }}
        />
      </label>
      <label htmlFor="nd-hoc-approach">Construction approach
        <select
          id="nd-hoc-approach"
          value={approach}
          onChange={(event) => {
            setApproachOverride(event.target.value as NativeHigherOrderEditableApproach);
            setSubmitted(false);
            setCreationError(null);
          }}
        >
          {approachOptions.filter((option) => option.valid || option.approach === approach).map((option) => <option key={option.approach} value={option.approach} disabled={!option.valid}>
            {option.label}{option.recommended ? " (Recommended)" : ""}
          </option>)}
        </select>
      </label>
      <FieldError message={!selectedApproach?.valid ? selectedApproach?.reason ?? "Choose a valid construction approach." : issueFor(visibleIssues, "approach")} />
    </details>

    {blocker || creationError ? <div className="nd-form-error" role="alert">{blocker ?? creationError}</div> : null}
    <footer>
      <button type="button" onClick={close}>Cancel</button>
      <button className="primary" type="submit" disabled={Boolean(blocker)}>{request.kind === "edit" ? "Save" : "Create"}</button>
    </footer>
  </form>;
}
