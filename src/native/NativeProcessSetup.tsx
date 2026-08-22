import { Plus, Trash2 } from "lucide-react";
import { useEffect, useMemo, useRef } from "react";
import { ReadOnlyDiagramViewport } from "../components/ReadOnlyDiagramViewport";
import type {
  AnalysisUiSettings,
  NativeProcessGraphRelationshipConfig,
  NativeProcessModerationConfig,
  NativeProcessModeratorConfig,
  NativeProcessPathConfig,
} from "../types";
import {
  NATIVE_PROCESS_CENTERING_POLICY,
  NATIVE_PROCESS_MAX_CONTROLS,
  NATIVE_PROCESS_MAX_MEDIATORS,
  NATIVE_PROCESS_MAX_MODERATIONS,
  NATIVE_PROCESS_MAX_MODERATORS,
  NATIVE_PROCESS_MAX_PATHS,
  NATIVE_PROCESS_MAX_PREDICTORS,
  nativeProcessGraphAssessment,
  parseNativeProcessGraph,
} from "./nativeProcess";
import { nativeOlsCsvValues } from "./nativeOls";
import { nativeProcessPreviewDiagramV1 } from "./nativeProcessPreviewDiagramV1";

interface NativeProcessSetupProps {
  settings: AnalysisUiSettings;
  setSettings: (patch: Partial<AnalysisUiSettings>) => void;
  numericColumns: readonly string[];
  disabled?: boolean;
}

function initialGraph(focal: string, outcome: string): NativeProcessGraphRelationshipConfig {
  return {
    model: "graph",
    focal_predictor: focal,
    paths: focal && outcome && focal !== outcome ? [{ from: focal, to: outcome }] : [],
    moderators: [],
    moderations: [],
    continuous_product_centering: NATIVE_PROCESS_CENTERING_POLICY,
  };
}

function pathToken(path: Pick<NativeProcessPathConfig, "from" | "to">): string {
  return `${path.from}\u0000${path.to}`;
}

function pathLabel(path: Pick<NativeProcessPathConfig, "from" | "to">): string {
  return `${path.from} -> ${path.to}`;
}

function roleVariables(graph: NativeProcessGraphRelationshipConfig, outcome: string): Set<string> {
  return new Set([
    graph.focal_predictor,
    outcome,
    ...graph.paths.flatMap((path) => [path.from, path.to]),
    ...graph.moderators.map((moderator) => moderator.variable),
  ].filter(Boolean));
}

function ProcessGraphPreview({
  graph,
  outcome,
}: {
  graph: NativeProcessGraphRelationshipConfig;
  outcome: string;
}) {
  const diagram = useMemo(() => nativeProcessPreviewDiagramV1(graph, outcome), [graph, outcome]);
  const description = graph.paths.length
    ? graph.paths.map((path) => {
      const moderation = graph.moderations.find((row) => row.from === path.from && row.to === path.to);
      return `${pathLabel(path)}${moderation ? ` moderated by ${moderation.moderator}${moderation.conditioning_moderator ? ` and ${moderation.conditioning_moderator}` : ""}` : ""}`;
    }).join("; ")
    : "No directed paths have been defined.";
  return (
    <figure id="nd-process-graph-preview" className="nd-process-graph-preview">
      <figcaption>Relationship preview</figcaption>
      <span id="nd-process-preview-title" className="sr-only">Graph-defined path analysis preview</span>
      <span id="nd-process-preview-description" className="sr-only">{description}</span>
      <ReadOnlyDiagramViewport graph={diagram} ariaLabel={`Graph-defined path analysis preview. ${description}`} />
    </figure>
  );
}

export default function NativeProcessSetup({
  settings,
  setSettings,
  numericColumns,
  disabled = false,
}: NativeProcessSetupProps) {
  const initialized = useRef(false);
  const outcome = settings.regressionOutcome?.trim() ?? "";
  const controls = nativeOlsCsvValues(settings.regressionControls);
  const parsed = parseNativeProcessGraph(settings.processGraph);
  const fallbackFocal = numericColumns.find((variable) => variable !== outcome) ?? "";
  const graph = parsed ?? initialGraph(fallbackFocal, outcome);
  const assessment = useMemo(() => nativeProcessGraphAssessment(settings), [settings]);
  const graphRoleSet = roleVariables(graph, outcome);
  const pathNodes = [...new Set([graph.focal_predictor, ...graph.paths.flatMap((path) => [path.from, path.to]), outcome])]
    .filter(Boolean);
  const mediatorCount = assessment.mediators.length;
  const predictorCount = assessment.predictors.length;

  const commit = (
    nextGraph: NativeProcessGraphRelationshipConfig,
    nextOutcome = outcome,
    nextControls = controls,
  ) => {
    const draft = {
      ...settings,
      regressionType: "process" as const,
      regressionOutcome: nextOutcome || null,
      regressionControls: nextControls.join(",") || null,
      regressionPredictors: null,
      processGraph: nextGraph,
    };
    const nextAssessment = nativeProcessGraphAssessment(draft);
    setSettings({
      regressionType: "process",
      regressionOutcome: nextOutcome || null,
      regressionControls: nextControls.join(",") || null,
      regressionPredictors: nextAssessment.predictors.join(",") || null,
      processGraph: nextGraph,
      robustSe: "hc3",
      preprocessing: "unstandardized",
      confidenceLevel: 0.95,
      studentizedInnerSamples: 0,
      permutationSamples: 0,
    });
  };

  useEffect(() => {
    if (initialized.current || parsed || numericColumns.length < 2) return;
    initialized.current = true;
    const nextOutcome = outcome || numericColumns[0];
    const focal = numericColumns.find((variable) => variable !== nextOutcome) ?? "";
    commit(initialGraph(focal, nextOutcome), nextOutcome, controls.filter((variable) => variable !== focal && variable !== nextOutcome));
  }, [controls, numericColumns, outcome, parsed]);

  const changeOutcome = (nextOutcome: string) => {
    const focal = graph.focal_predictor === nextOutcome
      ? numericColumns.find((variable) => variable !== nextOutcome && !graph.moderators.some((row) => row.variable === variable)) ?? ""
      : graph.focal_predictor;
    const paths = graph.paths
      .filter((path) => path.from !== nextOutcome)
      .map((path) => ({ ...path, from: path.from === graph.focal_predictor ? focal : path.from }))
      .filter((path) => path.from && path.to && path.from !== path.to);
    const normalizedPaths = paths.length ? paths : focal && nextOutcome ? [{ from: focal, to: nextOutcome }] : [];
    const keys = new Set(normalizedPaths.map(pathToken));
    commit({
      ...graph,
      focal_predictor: focal,
      paths: normalizedPaths,
      moderations: graph.moderations.filter((row) => keys.has(pathToken(row))),
    }, nextOutcome, controls.filter((variable) => variable !== nextOutcome && variable !== focal));
  };

  const changeFocal = (focal: string) => {
    const paths = graph.paths
      .filter((path) => path.to !== focal)
      .map((path) => ({ ...path, from: path.from === graph.focal_predictor ? focal : path.from }))
      .filter((path) => path.from && path.to && path.from !== path.to);
    const normalizedPaths = paths.length ? paths : focal && outcome ? [{ from: focal, to: outcome }] : [];
    const keys = new Set(normalizedPaths.map(pathToken));
    commit({
      ...graph,
      focal_predictor: focal,
      paths: normalizedPaths,
      moderations: graph.moderations.filter((row) => keys.has(pathToken(row))),
    }, outcome, controls.filter((variable) => variable !== focal));
  };

  const updatePath = (index: number, patch: Partial<NativeProcessPathConfig>) => {
    const paths = graph.paths.map((path, rowIndex) => rowIndex === index ? { ...path, ...patch } : path);
    const keys = new Set(paths.map(pathToken));
    commit({ ...graph, paths, moderations: graph.moderations.filter((row) => keys.has(pathToken(row))) });
  };

  const addPath = () => {
    const candidates = [graph.focal_predictor, ...pathNodes.filter((node) => node !== outcome)];
    const targets = [...pathNodes.filter((node) => node !== graph.focal_predictor), outcome];
    const existing = new Set(graph.paths.map(pathToken));
    const newMediator = mediatorCount < NATIVE_PROCESS_MAX_MEDIATORS && predictorCount < NATIVE_PROCESS_MAX_PREDICTORS
      ? numericColumns.find((candidate) => !roleVariables(graph, outcome).has(candidate) && !controls.includes(candidate))
      : undefined;
    const pairs = [
      ...candidates.flatMap((from) => targets.map((to) => ({ from, to }))),
      ...(newMediator ? [{ from: graph.focal_predictor, to: newMediator }] : []),
    ];
    const next = pairs
      .find((path) => path.from && path.to && path.from !== path.to && !existing.has(pathToken(path)));
    if (next) commit({ ...graph, paths: [...graph.paths, next] });
  };

  const removePath = (index: number) => {
    const removed = graph.paths[index];
    commit({
      ...graph,
      paths: graph.paths.filter((_, rowIndex) => rowIndex !== index),
      moderations: graph.moderations.filter((row) => pathToken(row) !== pathToken(removed)),
    });
  };

  const updateModerator = (index: number, patch: Partial<NativeProcessModeratorConfig>) => {
    const previous = graph.moderators[index];
    const moderators = graph.moderators.map((row, rowIndex) => rowIndex === index ? { ...row, ...patch } : row);
    const moderations = graph.moderations.map((row) => ({
      ...row,
      ...(row.moderator === previous.variable ? { moderator: patch.variable ?? row.moderator } : {}),
      ...(row.conditioning_moderator === previous.variable
        ? { conditioning_moderator: patch.variable ?? row.conditioning_moderator }
        : {}),
    }));
    commit({ ...graph, moderators, moderations });
  };

  const addModerator = () => {
    const used = roleVariables(graph, outcome);
    const variable = predictorCount < NATIVE_PROCESS_MAX_PREDICTORS
      ? numericColumns.find((candidate) => !used.has(candidate) && !controls.includes(candidate))
      : undefined;
    if (variable) commit({ ...graph, moderators: [...graph.moderators, { variable, scale: "continuous" }] });
  };

  const removeModerator = (index: number) => {
    const removed = graph.moderators[index];
    commit({
      ...graph,
      moderators: graph.moderators.filter((_, rowIndex) => rowIndex !== index),
      moderations: graph.moderations.filter((row) => row.moderator !== removed.variable && row.conditioning_moderator !== removed.variable),
    });
  };

  const updateModeration = (index: number, patch: Partial<NativeProcessModerationConfig>) => {
    commit({
      ...graph,
      moderations: graph.moderations.map((row, rowIndex) => rowIndex === index ? { ...row, ...patch } : row),
    });
  };

  const setModerationEdge = (index: number, token: string) => {
    const path = graph.paths.find((candidate) => pathToken(candidate) === token);
    if (path) updateModeration(index, {
      from: path.from,
      to: path.to,
      ...(path.from !== graph.focal_predictor || path.to !== outcome ? { conditioning_moderator: undefined } : {}),
    });
  };

  const addModeration = () => {
    const used = new Set(graph.moderations.map(pathToken));
    const path = graph.paths.find((candidate) => !used.has(pathToken(candidate)));
    const moderator = graph.moderators[0];
    if (path && moderator) commit({
      ...graph,
      moderations: [...graph.moderations, { from: path.from, to: path.to, moderator: moderator.variable }],
    });
  };

  const setControl = (variable: string, selected: boolean) => {
    const next = selected ? [...controls, variable] : controls.filter((candidate) => candidate !== variable);
    commit(graph, outcome, next);
  };

  return (
    <section id="nd-calculation-process-graph" className="nd-process-setup" aria-labelledby="nd-process-setup-title">
      <header>
        <div>
          <h3 id="nd-process-setup-title">Graph-defined relationships</h3>
        </div>
        <span>{graph.paths.length}/{NATIVE_PROCESS_MAX_PATHS} paths</span>
      </header>

      <div className="nd-process-role-grid">
        <label htmlFor="nd-process-outcome">Outcome
          <select id="nd-process-outcome" required disabled={disabled} value={outcome} onChange={(event) => changeOutcome(event.target.value)}>
            <option value="">Select terminal outcome</option>
            {numericColumns.map((variable) => <option key={variable} value={variable} disabled={graph.moderators.some((row) => row.variable === variable)}>{variable}</option>)}
          </select>
        </label>
        <label htmlFor="nd-process-focal">Focal predictor
          <select id="nd-process-focal" required disabled={disabled} value={graph.focal_predictor} onChange={(event) => changeFocal(event.target.value)}>
            <option value="">Select focal predictor</option>
            {numericColumns.map((variable) => <option key={variable} value={variable} disabled={variable === outcome || controls.includes(variable) || graph.moderators.some((row) => row.variable === variable)}>{variable}</option>)}
          </select>
        </label>
      </div>

      <fieldset className="nd-process-rows">
        <legend>Directed paths ({graph.paths.length}; {predictorCount}/{NATIVE_PROCESS_MAX_PREDICTORS} graph predictors; up to {NATIVE_PROCESS_MAX_MEDIATORS} mediators)</legend>
        {graph.paths.map((path, index) => (
          <div key={`process-path-${index}`} data-process-path-row className="nd-process-row">
            <label htmlFor={`nd-process-path-from-${index}`}>From
              <select id={`nd-process-path-from-${index}`} disabled={disabled} value={path.from} onChange={(event) => updatePath(index, { from: event.target.value })}>
                {[graph.focal_predictor, ...pathNodes.filter((node) => node !== graph.focal_predictor && node !== outcome)].filter(Boolean).map((node) => <option key={node} value={node}>{node}</option>)}
                {numericColumns.filter((node) => !graphRoleSet.has(node) && !controls.includes(node)).map((node) => <option key={node} value={node} disabled={mediatorCount >= NATIVE_PROCESS_MAX_MEDIATORS || predictorCount >= NATIVE_PROCESS_MAX_PREDICTORS}>{node} (new mediator)</option>)}
              </select>
            </label>
            <span aria-hidden="true">-&gt;</span>
            <label htmlFor={`nd-process-path-to-${index}`}>To
              <select id={`nd-process-path-to-${index}`} disabled={disabled} value={path.to} onChange={(event) => updatePath(index, { to: event.target.value })}>
                {[...pathNodes.filter((node) => node !== graph.focal_predictor), outcome].filter((node, rowIndex, values) => node && values.indexOf(node) === rowIndex).map((node) => <option key={node} value={node} disabled={node === path.from}>{node}</option>)}
                {numericColumns.filter((node) => !graphRoleSet.has(node) && !controls.includes(node)).map((node) => <option key={node} value={node} disabled={mediatorCount >= NATIVE_PROCESS_MAX_MEDIATORS || predictorCount >= NATIVE_PROCESS_MAX_PREDICTORS}>{node} (new mediator)</option>)}
              </select>
            </label>
            <button type="button" data-process-remove-path aria-label={`Remove path ${pathLabel(path)}`} disabled={disabled || graph.paths.length <= 1} onClick={() => removePath(index)}><Trash2 size={14} aria-hidden="true" /></button>
          </div>
        ))}
        <button id="nd-process-add-path" type="button" disabled={disabled || graph.paths.length >= NATIVE_PROCESS_MAX_PATHS} onClick={addPath}><Plus size={14} aria-hidden="true" /> Add path</button>
      </fieldset>

      <fieldset className="nd-process-rows">
        <legend>Moderator declarations ({graph.moderators.length}/{NATIVE_PROCESS_MAX_MODERATORS})</legend>
        {graph.moderators.map((moderator, index) => (
          <div key={`process-moderator-${index}`} data-process-moderator-row className="nd-process-row">
            <label htmlFor={`nd-process-moderator-variable-${index}`}>Variable
              <select id={`nd-process-moderator-variable-${index}`} disabled={disabled} value={moderator.variable} onChange={(event) => updateModerator(index, { variable: event.target.value })}>
                {numericColumns.map((variable) => <option key={variable} value={variable} disabled={variable !== moderator.variable && (roleVariables({ ...graph, moderators: graph.moderators.filter((_, rowIndex) => rowIndex !== index) }, outcome).has(variable) || controls.includes(variable))}>{variable}</option>)}
              </select>
            </label>
            <label htmlFor={`nd-process-moderator-scale-${index}`}>Scale / coding
              <select id={`nd-process-moderator-scale-${index}`} disabled={disabled} value={moderator.scale} onChange={(event) => updateModerator(index, { scale: event.target.value as NativeProcessModeratorConfig["scale"] })}>
                <option value="continuous">Continuous (sample-mean centered in products)</option>
                <option value="binary_0_1">Binary (must be exact 0/1; uncentered)</option>
              </select>
            </label>
            <button type="button" data-process-remove-moderator aria-label={`Remove moderator ${moderator.variable}`} disabled={disabled} onClick={() => removeModerator(index)}><Trash2 size={14} aria-hidden="true" /></button>
          </div>
        ))}
        <button id="nd-process-add-moderator" type="button" disabled={disabled || graph.moderators.length >= NATIVE_PROCESS_MAX_MODERATORS || predictorCount >= NATIVE_PROCESS_MAX_PREDICTORS} onClick={addModerator}><Plus size={14} aria-hidden="true" /> Add moderator</button>
      </fieldset>

      <fieldset className="nd-process-rows">
        <legend>Moderated paths ({graph.moderations.length}/{NATIVE_PROCESS_MAX_MODERATIONS})</legend>
        {graph.moderations.map((moderation, index) => (
          <div key={`process-moderation-${index}`} data-process-moderation-row className="nd-process-row nd-process-moderation-row">
            <label htmlFor={`nd-process-moderation-edge-${index}`}>Path
              <select id={`nd-process-moderation-edge-${index}`} disabled={disabled} value={pathToken(moderation)} onChange={(event) => setModerationEdge(index, event.target.value)}>
                {graph.paths.map((path) => <option key={pathToken(path)} value={pathToken(path)} disabled={pathToken(path) !== pathToken(moderation) && graph.moderations.some((row, rowIndex) => rowIndex !== index && pathToken(row) === pathToken(path))}>{pathLabel(path)}</option>)}
              </select>
            </label>
            <label htmlFor={`nd-process-moderation-primary-${index}`}>Primary / solved moderator
              <select id={`nd-process-moderation-primary-${index}`} disabled={disabled} value={moderation.moderator} onChange={(event) => updateModeration(index, { moderator: event.target.value, ...(event.target.value === moderation.conditioning_moderator ? { conditioning_moderator: undefined } : {}) })}>
                {graph.moderators.map((row) => <option key={row.variable} value={row.variable}>{row.variable}</option>)}
              </select>
            </label>
            <label htmlFor={`nd-process-moderation-conditioning-${index}`}>Conditioning moderator
              <select id={`nd-process-moderation-conditioning-${index}`} disabled={disabled || moderation.from !== graph.focal_predictor || moderation.to !== outcome} value={moderation.conditioning_moderator ?? ""} onChange={(event) => updateModeration(index, { conditioning_moderator: event.target.value || undefined })}>
                <option value="">None (two-way)</option>
                {graph.moderators.map((row) => <option key={row.variable} value={row.variable} disabled={row.variable === moderation.moderator}>{row.variable} (three-way)</option>)}
              </select>
            </label>
            <button type="button" data-process-remove-moderation aria-label={`Remove moderation from ${pathLabel(moderation)}`} disabled={disabled} onClick={() => commit({ ...graph, moderations: graph.moderations.filter((_, rowIndex) => rowIndex !== index) })}><Trash2 size={14} aria-hidden="true" /></button>
          </div>
        ))}
        <button id="nd-process-add-moderation" type="button" disabled={disabled || !graph.moderators.length || graph.moderations.length >= Math.min(NATIVE_PROCESS_MAX_MODERATIONS, graph.paths.length)} onClick={addModeration}><Plus size={14} aria-hidden="true" /> Moderate a path</button>
      </fieldset>

      <fieldset className="nd-pca-variables nd-process-controls">
        <legend>Controls ({controls.length}/{NATIVE_PROCESS_MAX_CONTROLS}; entered in every equation)</legend>
        <div className="nd-pca-variable-list">
          {numericColumns.map((variable) => (
            <label key={variable}>
              <input type="checkbox" data-process-control disabled={disabled || (graphRoleSet.has(variable) && !controls.includes(variable)) || (!controls.includes(variable) && controls.length >= NATIVE_PROCESS_MAX_CONTROLS)} checked={controls.includes(variable)} onChange={(event) => setControl(variable, event.target.checked)} />
              <span>{variable}</span>
            </label>
          ))}
        </div>
      </fieldset>

      <ProcessGraphPreview graph={graph} outcome={outcome} />
      <div className={`nd-process-assessment ${assessment.canRun ? "ready" : "blocked"}`} role="status" aria-live="polite">
        <strong>{assessment.canRun ? "Graph structure ready" : "Graph needs attention"}</strong>
        <span>{assessment.detail}</span>
      </div>
    </section>
  );
}
