import type { Edge, Node } from "@xyflow/react";
import { useMemo, useState } from "react";
import type { AddHigherOrderConstructResult } from "../store";
import type { ConstructData } from "../types";
import type {
  HigherOrderConstructionApproachV4,
  HigherOrderMeasurementTypeV4,
} from "../domain/semModelV4";
import {
  DEFAULT_NATIVE_HIGHER_ORDER_APPROACH,
  DEFAULT_NATIVE_HIGHER_ORDER_MEASUREMENT_TYPE,
  NATIVE_HIGHER_ORDER_SCOPE_LABEL,
  type NativeHigherOrderDraft,
  nativeHigherOrderComponentOptions,
  nativeHigherOrderCreationBlocker,
  nativeHigherOrderDraftProblems,
} from "./nativeHigherOrder";

export interface NativeHigherOrderDialogProps {
  nodes: readonly Node<ConstructData>[];
  edges: readonly Edge[];
  selectedComponentIds?: readonly string[];
  requireInitialPath?: boolean;
  create: (draft: NativeHigherOrderDraft) => AddHigherOrderConstructResult;
  close: () => void;
}

export default function NativeHigherOrderDialog({
  nodes,
  edges,
  selectedComponentIds = [],
  requireInitialPath = false,
  create,
  close,
}: NativeHigherOrderDialogProps) {
  const [approach, setApproach] = useState<Exclude<HigherOrderConstructionApproachV4, "hybrid">>(DEFAULT_NATIVE_HIGHER_ORDER_APPROACH);
  const [measurementType, setMeasurementType] = useState<HigherOrderMeasurementTypeV4>(DEFAULT_NATIVE_HIGHER_ORDER_MEASUREMENT_TYPE);
  const options = useMemo(
    () => nativeHigherOrderComponentOptions(nodes, edges, approach, measurementType),
    [approach, edges, measurementType, nodes],
  );
  const eligibleIds = useMemo(() => new Set(options.filter((option) => option.eligible).map((option) => option.id)), [options]);
  const initialComponents = [...new Set(selectedComponentIds)].filter((id) => eligibleIds.has(id));
  const initialPathConstruct = nodes.find((node) => (
    !node.data.semantic
    && node.data.indicators.length > 0
    && !initialComponents.includes(node.id)
  ))?.id ?? "";
  const [name, setName] = useState("Higher-order construct");
  const [shortName, setShortName] = useState("HOC");
  const [components, setComponents] = useState(initialComponents);
  const [pathDirection, setPathDirection] = useState<"hoc_to_construct" | "construct_to_hoc">("hoc_to_construct");
  const [pathConstructId, setPathConstructId] = useState(initialPathConstruct);
  const [submitted, setSubmitted] = useState(false);
  const [creationError, setCreationError] = useState<string | null>(null);
  const blocker = nativeHigherOrderCreationBlocker(nodes, edges, approach, measurementType);
  const pathOptions = nodes.filter((node) => (
    !node.data.semantic
    && node.data.indicators.length > 0
    && !components.includes(node.id)
  ));
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
  const problems = [
    ...nativeHigherOrderDraftProblems(draft, nodes, edges),
    ...(requireInitialPath && !pathConstructId
      ? ["Choose an ordinary measured construct for the initial structural path."]
      : []),
  ];
  const visibleProblems = submitted ? problems : [];

  return <form
    className="nd-dialog-form nd-higher-order-dialog"
    onSubmit={(event) => {
      event.preventDefault();
      setSubmitted(true);
      setCreationError(null);
      if (blocker || problems.length) return;
      const result = create(draft);
      if (result.status === "created") close();
      else setCreationError(result.detail);
    }}
  >
    <div className="nd-form-grid">
      <label htmlFor="nd-hoc-name">Name
        <input id="nd-hoc-name" autoFocus value={name} onChange={(event) => { setName(event.target.value); setSubmitted(false); }} />
      </label>
      <label htmlFor="nd-hoc-short-name">Short name
        <input id="nd-hoc-short-name" maxLength={12} value={shortName} onChange={(event) => { setShortName(event.target.value); setSubmitted(false); }} />
      </label>
      <label htmlFor="nd-hoc-approach">Approach
        <select
          id="nd-hoc-approach"
          value={approach}
          onChange={(event) => {
            setApproach(event.target.value as Exclude<HigherOrderConstructionApproachV4, "hybrid">);
            setSubmitted(false);
          }}
        >
          <option value="repeated_indicators">Repeated indicators</option>
          <option value="extended_repeated_indicators">Extended repeated indicators</option>
          <option value="embedded_two_stage">Embedded two-stage</option>
          <option value="disjoint_two_stage">Disjoint two-stage</option>
        </select>
      </label>
      <label htmlFor="nd-hoc-measurement-type">HCM type
        <select
          id="nd-hoc-measurement-type"
          value={measurementType}
          onChange={(event) => {
            setMeasurementType(event.target.value as HigherOrderMeasurementTypeV4);
            setSubmitted(false);
          }}
        >
          <option value="reflective_reflective">Reflective–reflective (RR)</option>
          <option value="reflective_formative">Reflective–formative (RF)</option>
          <option value="formative_reflective">Formative–reflective (FR)</option>
          <option value="formative_formative">Formative–formative (FF)</option>
        </select>
      </label>
    </div>
    <dl className="nd-property-list nd-hoc-summary">
      <div><dt>Method</dt><dd>{NATIVE_HIGHER_ORDER_SCOPE_LABEL}</dd></div>
      <div><dt>LOC relation</dt><dd>{measurementType.startsWith("reflective_") ? "Mode A loadings" : "Mode B weights and VIF"}</dd></div>
      <div><dt>HOC relation</dt><dd>{measurementType.endsWith("_reflective") ? "Component loadings" : "Component weights and VIF"}</dd></div>
      <div><dt>Execution</dt><dd>{approach.includes("two_stage") ? "Repeated/lower-order stage, then generated-score HOC stage" : "One full repeated-indicator stage"}</dd></div>
    </dl>
    <fieldset className="nd-hoc-components">
      <legend>Lower-order components</legend>
      {options.map((option) => <label key={option.id} className={option.eligible ? "checkbox-row" : "checkbox-row disabled"}>
        <input
          type="checkbox"
          checked={components.includes(option.id)}
          disabled={!option.eligible || Boolean(blocker)}
          onChange={(event) => {
            setSubmitted(false);
            setComponents((current) => event.target.checked
              ? [...current, option.id]
              : current.filter((component) => component !== option.id));
          }}
        />
        <span>{option.label} <small>[{option.shortName}]</small>{option.reason ? <small>{option.reason}</small> : null}</span>
      </label>)}
    </fieldset>
    {requireInitialPath ? <fieldset className="nd-hoc-initial-path">
      <legend>Initial structural path for the saved revision</legend>
      <label htmlFor="nd-hoc-path-direction">Direction
        <select id="nd-hoc-path-direction" value={pathDirection} onChange={(event) => {
          setPathDirection(event.target.value as "hoc_to_construct" | "construct_to_hoc");
          setSubmitted(false);
        }}>
          <option value="hoc_to_construct">HOC → construct</option>
          <option value="construct_to_hoc">Construct → HOC</option>
        </select>
      </label>
      <label htmlFor="nd-hoc-path-construct">Ordinary construct
        <select id="nd-hoc-path-construct" value={pathConstructId} onChange={(event) => {
          setPathConstructId(event.target.value);
          setSubmitted(false);
        }}>
          <option value="">Choose a construct</option>
          {pathOptions.map((node) => <option key={node.id} value={node.id}>{node.data.label} [{node.data.shortName}]</option>)}
        </select>
      </label>
    </fieldset> : null}
    <p className="nd-dialog-note">After creation, connect the HOC to one or more ordinary constructs. Generated component and technical paths stay separate from authored hypotheses.</p>
    {blocker || visibleProblems.length || creationError ? <div className="nd-form-error" role="alert">{blocker ?? creationError ?? visibleProblems[0]}</div> : null}
    <footer><button type="button" onClick={close}>Cancel</button><button className="primary" type="submit" disabled={Boolean(blocker) || problems.length > 0}>Create higher-order construct</button></footer>
  </form>;
}
