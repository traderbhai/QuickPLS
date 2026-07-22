import { ArrowLeftRight, Network, Plus, Trash2 } from "lucide-react";
import { useState } from "react";
import { useWorkspace } from "../store";
import type { MeasurementMode } from "../types";

export function Inspector() {
  const selectedNodeId = useWorkspace((state) => state.selectedNodeId);
  const selectedEdgeId = useWorkspace((state) => state.selectedEdgeId);
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const dataset = useWorkspace((state) => state.dataset);
  const node = nodes.find((item) => item.id === selectedNodeId);
  const edge = edges.find((item) => item.id === selectedEdgeId);
  const selectedRun = useWorkspace((state) => state.runs.find((run) => run.id === state.selectedResultRunId) ?? state.runs[0]);
  const updateConstruct = useWorkspace((state) => state.updateConstruct);
  const updateEdge = useWorkspace((state) => state.updateEdge);
  const assignIndicator = useWorkspace((state) => state.assignIndicator);
  const unassignIndicator = useWorkspace((state) => state.unassignIndicator);
  const reverseSelectedPath = useWorkspace((state) => state.reverseSelectedPath);
  const removeSelection = useWorkspace((state) => state.removeSelection);
  const autoLayout = useWorkspace((state) => state.autoLayout);
  const resetIndicatorLayout = useWorkspace((state) => state.resetIndicatorLayout);
  const setConstructIndicatorSide = useWorkspace((state) => state.setConstructIndicatorSide);
  const addTwoStageInteraction = useWorkspace((state) => state.addTwoStageInteraction);
  const setSelectedNode = useWorkspace((state) => state.setSelectedNode);
  const setSelectedEdge = useWorkspace((state) => state.setSelectedEdge);
  const [interactionDraft, setInteractionDraft] = useState({ predictor: "", moderator: "", outcome: "" });

  if (edge) {
    const source = nodes.find((item) => item.id === edge.source);
    const target = nodes.find((item) => item.id === edge.target);
    const isControl = edge.data?.role === "control";
    const isCovariance = edge.data?.role === "covariance";
    const controlLabel = typeof edge.data?.controlLabel === "string" ? edge.data.controlLabel : "";
    if (isCovariance) return <aside className="inspector">
      <div className="inspector-tabs"><button onClick={() => setSelectedNode(source?.id ?? null)}>Construct</button><button className="active">Covariance</button><button onClick={() => setSelectedNode(null)}>Model</button></div>
      <div className="path-heading"><Network size={16} /><div><strong>Covariance display</strong><span>{source?.data.shortName} &lt;-&gt; {target?.data.shortName}</span></div></div>
      <label>Left construct<input value={source?.data.label ?? edge.source} readOnly /></label>
      <label>Right construct<input value={target?.data.label ?? edge.target} readOnly /></label>
      <label>Display label<input value={String(edge.label ?? "")} onChange={(event) => updateEdge(edge.id, { label: event.target.value })} /></label>
      <div className="inspector-actions"><button className="secondary-button danger" onClick={removeSelection}><Trash2 size={14} />Delete</button></div>
      <div className="method-note"><strong>Visual covariance</strong><p>This arc is excluded from PLS recipe paths. CB-SEM covariance estimation remains controlled by the supported method settings and engine schema.</p></div>
    </aside>;
    return <aside className="inspector">
      <div className="inspector-tabs"><button onClick={() => setSelectedNode(source?.id ?? null)}>Construct</button><button className="active">Path</button><button onClick={() => setSelectedNode(null)}>Model</button></div>
      <div className="path-heading"><Network size={16} /><div><strong>Structural path</strong><span>{source?.data.shortName} -&gt; {target?.data.shortName}</span></div></div>
      <label>From<input value={source?.data.label ?? edge.source} readOnly /></label>
      <label>To<input value={target?.data.label ?? edge.target} readOnly /></label>
      <label>Display label<input value={String(edge.label ?? "")} onChange={(event) => updateEdge(edge.id, { label: event.target.value })} /></label>
      <fieldset><legend>Path role</legend><label className="checkbox-row"><input type="checkbox" checked={isControl} onChange={(event) => updateEdge(edge.id, { label: event.target.checked ? "Control" : "Path", data: { ...edge.data, role: event.target.checked ? "control" : undefined, controlLabel: event.target.checked ? controlLabel : undefined } })} />Control variable</label>
        {isControl && <label>Control label<input value={controlLabel} onChange={(event) => updateEdge(edge.id, { data: { ...edge.data, role: "control", controlLabel: event.target.value } })} /></label>}
      </fieldset>
      <label>Line routing<select value={edge.type ?? "smoothstep"} onChange={(event) => updateEdge(edge.id, { type: event.target.value })}>
        <option value="smoothstep">Orthogonal</option><option value="default">Curved</option><option value="straight">Straight</option>
      </select></label>
      <div className="inspector-actions">
        <button className="secondary-button" onClick={reverseSelectedPath}><ArrowLeftRight size={14} />Reverse</button>
        <button className="secondary-button danger" onClick={removeSelection}><Trash2 size={14} />Delete</button>
      </div>
      <div className="method-note"><strong>Path direction matters</strong><p>The arrow points from predictor to outcome. Reverse only when the theoretical relationship is specified in the opposite direction.</p></div>
    </aside>;
  }

  if (!node) return <aside className="inspector model-inspector">
    <div className="inspector-tabs"><button onClick={() => setSelectedNode(nodes[0]?.id ?? null)}>Construct</button><button onClick={() => setSelectedEdge(edges[0]?.id ?? null)}>Path</button><button className="active">Model</button></div>
    <div className="path-heading"><Network size={16} /><div><strong>Structural model</strong><span>{nodes.length} constructs | {edges.length} paths</span></div></div>
    <details className="inspector-section" open><summary>Essentials</summary>
      <label>Weighting scheme<select defaultValue="path"><option value="path">Path weighting</option><option value="factor">Factor weighting</option><option value="pca">PCA weighting</option></select></label>
      <label>Preprocessing<select defaultValue="standardized"><option value="standardized">Standardized</option><option value="centered">Mean centered</option><option value="unstandardized">Unstandardized</option></select></label>
    </details>
    <details className="inspector-section"><summary>Advanced interactions</summary>
    <fieldset><legend>Two-stage interaction</legend>
      <label>Predictor<select value={interactionDraft.predictor} onChange={(event) => setInteractionDraft((value) => ({ ...value, predictor: event.target.value }))}><option value="">Choose...</option>{nodes.map((item) => <option key={item.id} value={item.id}>{item.data.shortName}</option>)}</select></label>
      <label>Moderator<select value={interactionDraft.moderator} onChange={(event) => setInteractionDraft((value) => ({ ...value, moderator: event.target.value }))}><option value="">Choose...</option>{nodes.map((item) => <option key={item.id} value={item.id}>{item.data.shortName}</option>)}</select></label>
      <label>Outcome<select value={interactionDraft.outcome} onChange={(event) => setInteractionDraft((value) => ({ ...value, outcome: event.target.value }))}><option value="">Choose...</option>{nodes.map((item) => <option key={item.id} value={item.id}>{item.data.shortName}</option>)}</select></label>
      <div className="inspector-actions"><button className="secondary-button" disabled={new Set([interactionDraft.predictor, interactionDraft.moderator, interactionDraft.outcome]).size !== 3 || !interactionDraft.predictor || !interactionDraft.moderator || !interactionDraft.outcome} onClick={() => addTwoStageInteraction(interactionDraft.predictor, interactionDraft.moderator, interactionDraft.outcome)}>Create interaction</button></div>
    </fieldset>
    </details>
    <div className="inspector-actions"><button className="secondary-button" onClick={() => autoLayout("horizontal")}>Arrange model</button></div>
    <div className="method-note"><strong>QuickPLS v1.0 stable scope</strong><p>Supported PLS-SEM estimates and assessment outputs are validated for the documented v1.0 scope. Unsupported model shapes remain blocked or explicitly marked.</p></div>
  </aside>;

  const update = (patch: Parameters<typeof updateConstruct>[1]) => updateConstruct(node.id, patch);
  const availableIndicators = dataset.columns.filter((column) => !node.data.indicators.includes(column));
  const componentCandidates = nodes.filter((item) => item.id !== node.id && item.data.semantic !== "interaction");
  const higherOrder = node.data.higherOrder;
  const higherOrderComponents = higherOrder?.components ?? [];
  const setHigherOrderEnabled = (enabled: boolean) => {
    if (!enabled) {
      update({ semantic: undefined, higherOrder: undefined });
      return;
    }
    update({
      semantic: "higher_order",
      higherOrder: {
        id: node.id,
        components: componentCandidates.slice(0, 2).map((item) => item.id),
        method: "repeated_indicators",
        stage_one_recipe: null,
      },
    });
  };
  const updateHigherOrderComponent = (componentId: string, checked: boolean) => {
    const components = checked
      ? [...new Set([...higherOrderComponents, componentId])]
      : higherOrderComponents.filter((item) => item !== componentId);
    update({
      semantic: "higher_order",
      higherOrder: {
        id: node.id,
        method: higherOrder?.method ?? "repeated_indicators",
        stage_one_recipe: higherOrder?.stage_one_recipe ?? null,
        components,
      },
    });
  };
  return <aside className="inspector">
    <div className="inspector-tabs"><button className="active">Construct</button><button onClick={() => setSelectedEdge(edges.find((item) => item.source === node.id || item.target === node.id)?.id ?? edges[0]?.id ?? null)}>Path</button><button onClick={() => setSelectedNode(null)}>Model</button></div>
    <details className="inspector-section" open><summary>Essentials</summary>
    <label>Name<input value={node.data.label} onChange={(event) => update({ label: event.target.value })} /></label>
    <label>Short name<input value={node.data.shortName} onChange={(event) => update({ shortName: event.target.value.toUpperCase().slice(0, 8) })} /></label>
    <fieldset><legend>Measurement mode</legend><div className="segmented">
      {(["reflective", "formative"] as MeasurementMode[]).map((mode) => <button key={mode} className={node.data.mode === mode ? "active" : ""} onClick={() => update({ mode })}>{mode}</button>)}
    </div></fieldset>
    <div className="inspector-actions"><button className="secondary-button danger" onClick={removeSelection}><Trash2 size={14} />Delete construct</button></div>
    </details>
    {node.data.semantic === "interaction" && node.data.interaction ? <div className="method-note"><strong>Two-stage interaction</strong><p>{nodes.find((item) => item.id === node.data.interaction?.predictor)?.data.shortName} x {nodes.find((item) => item.id === node.data.interaction?.moderator)?.data.shortName} moderates {nodes.find((item) => item.id === node.data.interaction?.outcome)?.data.shortName}. Estimation is blocked until the v0.5 two-stage engine gate is complete.</p></div> : null}
    <details className="inspector-section" open><summary>Indicators</summary>
    <div className="inspector-section-title"><strong>Indicators ({node.data.indicators.length})</strong></div>
    <label className="indicator-picker"><span><Plus size={13} />Assign dataset variable</span><select value="" onChange={(event) => { if (event.target.value) assignIndicator(node.id, event.target.value); }}>
      <option value="">Choose variable...</option>{availableIndicators.map((indicator) => <option key={indicator}>{indicator}</option>)}
    </select></label>
    <div className="indicator-table">
      <div className="indicator-table-head"><span>Indicator</span><span>Role</span><span /></div>
      {node.data.indicators.map((indicator) => <div className="indicator-table-row" key={indicator}>
        <span className="indicator-name">{indicator}</span>
        <span>{node.data.mode === "reflective" ? "Loading" : "Weight"}</span>
        <button title={`Remove ${indicator}`} onClick={() => unassignIndicator(node.id, indicator)}><Trash2 size={14} /></button>
      </div>)}
      {node.data.indicators.length === 0 ? <div className="indicator-empty">No indicators assigned.</div> : null}
    </div>
    </details>
    <details className="inspector-section"><summary>Layout</summary>
      <div className="inspector-actions wrap">
        <button className="secondary-button" onClick={() => setConstructIndicatorSide(node.id, "left")}>Indicators left</button>
        <button className="secondary-button" onClick={() => setConstructIndicatorSide(node.id, "right")}>Indicators right</button>
        <button className="secondary-button" onClick={() => setConstructIndicatorSide(node.id, "top")}>Indicators top</button>
        <button className="secondary-button" onClick={() => setConstructIndicatorSide(node.id, "bottom")}>Indicators bottom</button>
        <button className="secondary-button" onClick={() => resetIndicatorLayout(node.id)}>Reset layout</button>
      </div>
    </details>
    {node.data.semantic !== "interaction" ? <details className="inspector-section"><summary>Advanced semantics</summary><fieldset><legend>Higher-order construct</legend>
      <label className="checkbox-row"><input type="checkbox" checked={node.data.semantic === "higher_order"} onChange={(event) => setHigherOrderEnabled(event.target.checked)} />Use as higher-order construct</label>
      {node.data.semantic === "higher_order" && <div className="hoc-editor">
        <label>Method<select value={higherOrder?.method ?? "repeated_indicators"} onChange={(event) => update({ semantic: "higher_order", higherOrder: { id: node.id, components: higherOrderComponents, method: event.target.value as "repeated_indicators" | "two_stage" | "hybrid", stage_one_recipe: higherOrder?.stage_one_recipe ?? null } })}>
          <option value="repeated_indicators">Repeated indicators</option>
          <option value="two_stage">Two-stage</option>
          <option value="hybrid">Hybrid</option>
        </select></label>
        {higherOrder?.method === "hybrid" ? <div className="method-note"><strong>Hybrid experimental</strong><p>Hybrid splits each component's indicators between lower-order and higher-order blocks. Components need at least two indicators.</p></div> : null}
        <div className="component-list">
          {componentCandidates.map((candidate) => <label className="checkbox-row" key={candidate.id}><input type="checkbox" checked={higherOrderComponents.includes(candidate.id)} onChange={(event) => updateHigherOrderComponent(candidate.id, event.target.checked)} />{candidate.data.shortName}</label>)}
          {componentCandidates.length === 0 ? <div className="indicator-empty">No lower-order components available.</div> : null}
        </div>
      </div>}
    </fieldset></details> : null}
    <details className="inspector-section"><summary>Diagnostics</summary>
      <label>Missing values<select><option>Use dataset setting</option><option>Casewise deletion</option><option>Mean replacement</option></select></label>
    </details>
    <details className="inspector-section"><summary>Results</summary>
      {selectedRun?.result ? <div className="method-note"><strong>Selected run values</strong><p>R² {node.id}: {selectedRun.result.r_squared[node.id]?.toFixed(4) ?? "N/A"}. Indicator values and path coefficients remain synchronized with the selected result overlay.</p></div> : <div className="indicator-empty">Run or select a compatible result to inspect construct-level values here.</div>}
    </details>
    <div className="method-note"><strong>QuickPLS v1.0 stable scope</strong><p>Mode A and Mode B measurement, supported assessment, bootstrap, and permutation workflows are available within the documented v1.0 scope.</p></div>
  </aside>;
}
