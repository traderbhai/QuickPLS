import type { Edge, Node } from "@xyflow/react";
import { useMemo, useState } from "react";
import type { AddHigherOrderConstructResult } from "../store";
import type { ConstructData } from "../types";
import {
  NATIVE_HIGHER_ORDER_SCOPE_LABEL,
  nativeHigherOrderComponentOptions,
  nativeHigherOrderCreationBlocker,
  nativeHigherOrderDraftProblems,
} from "./nativeHigherOrder";

export interface NativeHigherOrderDialogProps {
  nodes: readonly Node<ConstructData>[];
  edges: readonly Edge[];
  selectedComponentIds?: readonly string[];
  create: (draft: { name: string; shortName: string; components: string[] }) => AddHigherOrderConstructResult;
  close: () => void;
}

export default function NativeHigherOrderDialog({
  nodes,
  edges,
  selectedComponentIds = [],
  create,
  close,
}: NativeHigherOrderDialogProps) {
  const options = useMemo(() => nativeHigherOrderComponentOptions(nodes, edges), [edges, nodes]);
  const eligibleIds = useMemo(() => new Set(options.filter((option) => option.eligible).map((option) => option.id)), [options]);
  const initialComponents = [...new Set(selectedComponentIds)].filter((id) => eligibleIds.has(id));
  const [name, setName] = useState("Higher-order construct");
  const [shortName, setShortName] = useState("HOC");
  const [components, setComponents] = useState(initialComponents);
  const [submitted, setSubmitted] = useState(false);
  const [creationError, setCreationError] = useState<string | null>(null);
  const blocker = nativeHigherOrderCreationBlocker(nodes, edges);
  const problems = nativeHigherOrderDraftProblems({ name, shortName, components }, nodes, edges);
  const visibleProblems = submitted ? problems : [];

  return <form
    className="nd-dialog-form nd-higher-order-dialog"
    onSubmit={(event) => {
      event.preventDefault();
      setSubmitted(true);
      setCreationError(null);
      if (blocker || problems.length) return;
      const result = create({ name, shortName, components });
      if (result.status === "created") close();
      else setCreationError(result.detail);
    }}
  >
    <p className="nd-dialog-intro">Create one reflective–reflective higher-order construct from lower-order component scores.</p>
    <div className="nd-form-grid">
      <label htmlFor="nd-hoc-name">Name
        <input id="nd-hoc-name" autoFocus value={name} onChange={(event) => { setName(event.target.value); setSubmitted(false); }} />
      </label>
      <label htmlFor="nd-hoc-short-name">Short name
        <input id="nd-hoc-short-name" maxLength={12} value={shortName} onChange={(event) => { setShortName(event.target.value); setSubmitted(false); }} />
      </label>
    </div>
    <dl className="nd-property-list nd-hoc-summary">
      <div><dt>Method</dt><dd>{NATIVE_HIGHER_ORDER_SCOPE_LABEL}</dd></div>
      <div><dt>Stage 1</dt><dd>Estimate lower-order component scores</dd></div>
      <div><dt>Stage 2</dt><dd>Use component scores as generated HOC indicators</dd></div>
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
    <p className="nd-dialog-note">After creation, use the Path tool to add one HOC-to-outcome relationship. The bounded native workflow permits no other structural path and runs with PLS-SEM Algorithm, path weighting, standardized data, and listwise deletion. HOC bootstrapping and permutation inference remain unavailable.</p>
    {blocker || visibleProblems.length || creationError ? <div className="nd-form-error" role="alert">{blocker ?? creationError ?? visibleProblems[0]}</div> : null}
    <footer><button type="button" onClick={close}>Cancel</button><button className="primary" type="submit" disabled={Boolean(blocker) || problems.length > 0}>Create higher-order construct</button></footer>
  </form>;
}
