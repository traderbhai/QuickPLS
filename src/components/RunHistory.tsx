import { AlertTriangle, Copy, FlaskConical, Search } from "lucide-react";
import { useWorkspace } from "../store";
import type { AnalysisRun, AssessmentResult, HtmtAssessment, PlsResult, ResultWorkspaceTab } from "../types";
import { findBcaParameter, findBootstrapParameter, findStudentizedParameter, formatParameterIdentity } from "../domain/inference";
import { analysisReadiness } from "../domain/analysisReadiness";
import { isNativeDesktop } from "../services/projectService";
import { ReadinessPanel } from "./ReadinessPanel";
import { ActionStrip, EmptyState, PageHeader, StatusBadge, TabStrip } from "./Ui";

const resultTabs: Array<{ id: ResultWorkspaceTab; label: string }> = [
  { id: "summary", label: "Summary" },
  { id: "measurement", label: "Measurement Model" },
  { id: "structural", label: "Structural Model" },
  { id: "quality", label: "Reliability and Validity" },
  { id: "inference", label: "Inference" },
  { id: "prediction", label: "Prediction" },
  { id: "groups", label: "Groups" },
  { id: "diagnostics", label: "Diagnostics" },
  { id: "comparison", label: "Comparison" },
];

export function RunHistory() {
  const runs = useWorkspace((state) => state.runs);
  const setView = useWorkspace((state) => state.setView);
  const dataset = useWorkspace((state) => state.dataset);
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const settings = useWorkspace((state) => state.analysisSettings);
  const resultState = useWorkspace((state) => state.resultWorkspaceState);
  const setResultState = useWorkspace((state) => state.setResultWorkspaceState);
  const selectedEdgeId = useWorkspace((state) => state.selectedEdgeId);
  const setSelectedEdge = useWorkspace((state) => state.setSelectedEdge);
  const setSelectedNode = useWorkspace((state) => state.setSelectedNode);
  const readiness = analysisReadiness({ dataset, nodes, edges, settings, nativeDesktop: isNativeDesktop() });
  const search = resultState.tableSearch.toLowerCase();
  const visibleRuns = runs.filter((run) => {
    const body = `${run.name} ${run.method} ${run.warnings.join(" ")} ${run.result?.paths.map((path) => `${path.source} ${path.target}`).join(" ") ?? ""}`.toLowerCase();
    return body.includes(search);
  });
  const selectedRun = visibleRuns.find((run) => run.id === resultState.selectedRunId) ?? visibleRuns[0];
  const significantWarningCount = selectedRun?.warnings.filter((warning) => !warning.toLowerCase().includes("validated")).length ?? 0;
  const bestR2 = selectedRun?.result ? Object.entries(selectedRun.result.r_squared).sort((a, b) => b[1] - a[1])[0] : null;
  const selectedEdge = edges.find((edge) => edge.id === selectedEdgeId);
  const activePath = selectedEdge ? { source: selectedEdge.source, target: selectedEdge.target } : null;
  const focusPath = (source: string, target: string) => {
    const edge = edges.find((candidate) => candidate.source === source && candidate.target === target);
    if (edge) {
      setSelectedEdge(edge.id);
      window.dispatchEvent(new CustomEvent("quickpls:focus-edge", { detail: { id: edge.id } }));
    } else {
      setSelectedNode(target);
      window.dispatchEvent(new CustomEvent("quickpls:focus-construct", { detail: { id: target } }));
    }
    setView("models");
  };
  const copyVisibleSummary = async () => {
    const text = visibleRuns.map((run) => `${run.name}\t${run.method}\t${run.status}\t${run.createdAt}`).join("\n");
    await navigator.clipboard?.writeText(text);
  };
  const exportCurrentTable = () => {
    const csv = selectedRun ? csvForCurrentResultTab(selectedRun, resultState.selectedTab) : "";
    const url = URL.createObjectURL(new Blob([csv], { type: "text/csv" }));
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `quickpls-${resultState.selectedTab}-results.csv`;
    anchor.click();
    setTimeout(() => URL.revokeObjectURL(url), 0);
  };
  const emptyPrimary = readiness.blockers[0]?.actionView ?? (readiness.canRun ? "run" : "analyses");
  const emptyPrimaryLabel = readiness.blockers[0]?.actionLabel ?? (readiness.canRun ? "Run method" : "Open setup");
  const previewTabs = ["Summary", "Measurement", "Structural", "Reliability", "Inference", "Diagnostics"];

  if (runs.length === 0) return <section className="workspace-page">
    <PageHeader title="Results" description="Completed runs, immutable recipes, estimates, and provenance records." />
    <ReadinessPanel readiness={readiness} compact onNavigate={setView} />
    <EmptyState title="No completed results" description={readiness.canRun ? "Run the selected method to create the first result." : readiness.blockers[0]?.detail ?? "Complete the analysis checklist before running."} actions={<><button className="run-button" onClick={() => setView(emptyPrimary)}>{emptyPrimaryLabel}</button><button className="secondary-button" onClick={() => setView("analyses")}>Open setup</button></>} />
    <div className="result-preview-tabs" aria-label="Result sections preview">{previewTabs.map((tab) => <span key={tab}>{tab}</span>)}</div>
  </section>;

  return <section className="workspace-page">
    <PageHeader title="Results" description="Review saved runs by measurement, structural, inference, prediction, groups, diagnostics, and comparison workflow." actions={<StatusBadge status="validated">{visibleRuns.length} visible</StatusBadge>} />
    <ActionStrip>
      <TabStrip label="Result workspace sections" tabs={resultTabs} value={resultState.selectedTab} onChange={(selectedTab) => setResultState({ selectedTab })} />
      <label className="result-search"><Search size={13} /><input aria-label="Search result tables" placeholder="Search runs, paths, warnings" value={resultState.tableSearch} onChange={(event) => setResultState({ tableSearch: event.target.value })} /></label>
      <button className="secondary-button" onClick={() => void copyVisibleSummary()}><Copy size={14} />Copy run list</button>
      <button className="secondary-button" disabled={!selectedRun?.result} onClick={exportCurrentTable}>Export current table</button>
      <button className="secondary-button" onClick={() => setResultState({ includeExperimental: !resultState.includeExperimental })}>{resultState.includeExperimental ? "Include experimental" : "Validated only"}</button>
      <button className="secondary-button" onClick={() => setResultState({ tableDensity: resultState.tableDensity === "compact" ? "comfortable" : "compact" })}>{resultState.tableDensity}</button>
    </ActionStrip>
    {selectedRun ? <div className="result-headline-grid">
      <article><span>Selected run</span><strong>{selectedRun.name}</strong><small>{selectedRun.method}</small></article>
      <article><span>Strongest R²</span><strong>{bestR2 ? bestR2[1].toFixed(4) : "N/A"}</strong><small>{bestR2?.[0] ?? "No endogenous construct"}</small></article>
      <article><span>Paths</span><strong>{selectedRun.result?.paths.length ?? 0}</strong><small>Click a row to focus the diagram</small></article>
      <article className={significantWarningCount ? "warning" : "validated"}><span>Warnings</span><strong>{significantWarningCount}</strong><small>{significantWarningCount ? "Review provenance before export" : "No extra warnings"}</small></article>
    </div> : null}
    {resultState.selectedTab === "groups" ? <section className="results-groups-bridge" aria-label="Groups result workflow">
      <div>
        <strong>Groups and segmentation results</strong>
        <p>MICOM, permutation MGA, FIMIX-PLS, PLS-POS, and IPMA outputs appear here when the selected run contains those payloads.</p>
      </div>
      <button className="secondary-button" onClick={() => setView("analyses")}>Configure group workflow in Setup</button>
    </section> : null}
    <div className={`run-list result-tab-${resultState.selectedTab} table-density-${resultState.tableDensity}`}>{visibleRuns.map((run) => <article key={run.id} className="run-row researcher-result-card">
      <div className="run-icon"><FlaskConical size={18} /></div>
      <div className="run-content"><strong>{run.name}</strong><p>{new Date(run.createdAt).toLocaleString()} | seed {run.seed} | fingerprint {run.fingerprint}</p><span><AlertTriangle size={13} />{scopeCopy(run.warnings[0])}</span>
        {run.result ? <RunResultSections run={run} tab={resultState.selectedTab} focusPath={focusPath} activePath={activePath} /> : <SectionEmpty title="No result payload" detail="This saved run does not contain a completed result payload." />}
      </div>
      <div className="run-status">Scope checked</div>
    </article>)}</div>
    {visibleRuns.length === 0 ? <EmptyState title="No matching runs" description="Clear the search field or include a broader result section." /> : null}
  </section>;
}

function RunResultSections({ run, tab, focusPath, activePath }: { run: AnalysisRun; tab: ResultWorkspaceTab; focusPath: (source: string, target: string) => void; activePath: { source: string; target: string } | null }) {
  const result = run.result;
  if (!result) return null;
  if (tab === "summary") return <SummaryResults run={run} focusPath={focusPath} activePath={activePath} />;
  if (tab === "measurement") return <MeasurementResults result={result} assessment={run.assessment} focusPath={focusPath} activePath={activePath} />;
  if (tab === "structural") return <StructuralResults run={run} focusPath={focusPath} activePath={activePath} />;
  if (tab === "quality") return <QualityResults assessment={run.assessment} />;
  if (tab === "inference") return <InferenceResults run={run} />;
  if (tab === "prediction") return <PredictionResults result={result} assessment={run.assessment} />;
  if (tab === "groups") return <GroupResults result={result} />;
  if (tab === "diagnostics") return <DiagnosticsResults run={run} />;
  return <ComparisonResults />;
}

function SummaryResults({ run, focusPath, activePath }: { run: AnalysisRun; focusPath: (source: string, target: string) => void; activePath: { source: string; target: string } | null }) {
  const result = run.result!;
  const warningCount = [...run.warnings, ...result.warnings].filter((warning) => !warning.toLowerCase().includes("validated")).length;
  return <div className="result-sections result-summary" tabIndex={0} role="region" aria-label={`${run.name} result summary`}>
    <div className="result-kpi-row">
      <MetricTile label="Iterations" value={String(result.iterations)} detail={result.converged ? "converged" : "not converged"} tone={result.converged ? "ok" : "warn"} />
      <MetricTile label="Observations" value={String(result.used_observations)} detail={result.omitted_observations ? `${result.omitted_observations} omitted` : "complete cases used"} />
      {Object.entries(result.r_squared).map(([construct, value]) => <MetricTile key={construct} label={`R² ${construct}`} value={value.toFixed(4)} detail={interpretR2(value)} tone={value >= 0.75 ? "ok" : value >= 0.25 ? "neutral" : "warn"} />)}
      <MetricTile label="Warnings" value={String(warningCount)} detail={warningCount ? "review diagnostics" : "none beyond scope status"} tone={warningCount ? "warn" : "ok"} />
    </div>
    <SectionTable title="Path coefficients" note="Click a path row to focus the related edge in the SEM diagram." columns={["Path", "Coefficient", "Direction"]} rows={result.paths.map((path) => [pathLabel(path.source, path.target), path.coefficient.toFixed(6), coefficientDirection(path.coefficient)])} activeRowIndexes={activeIndexes(result.paths, activePath)} onRowClick={(_, index) => focusPath(result.paths[index].source, result.paths[index].target)} />
    <EffectsTable result={result} activePath={activePath} />
    {result.mediation?.estimates.length ? <MediationTable run={run} /> : null}
    {result.moderation?.estimates.length ? <ModerationTable run={run} /> : null}
    <ResultGuidance title="Researcher review" items={["Open Measurement Model for indicator loadings/weights and cross-loadings.", "Open Reliability and Validity for alpha, rho_A, rho_C, AVE, Fornell-Larcker, and HTMT.", "Enable bootstrap in Setup before interpreting p values or confidence intervals."]} />
  </div>;
}

function MeasurementResults({ result, assessment, focusPath, activePath }: { result: PlsResult; assessment?: AssessmentResult; focusPath: (source: string, target: string) => void; activePath: { source: string; target: string } | null }) {
  return <div className="result-sections" tabIndex={0} role="region" aria-label="Measurement model results">
    <SectionTable title="Outer loadings and weights" note="Reflective constructs are usually interpreted through loadings; formative constructs require weights and collinearity diagnostics." columns={["Construct", "Indicator", "Loading", "Weight", "Loading status"]} rows={result.outer_estimates.map((row) => [row.construct, row.indicator, row.loading.toFixed(6), row.weight.toFixed(6), loadingStatus(row.loading)])} />
    {assessment?.formative_indicator_vif.length ? <SectionTable title="Outer VIF for formative indicators" note="Use VIF to screen formative indicator collinearity." columns={["Construct", "Indicator", "VIF", "Status"]} rows={assessment.formative_indicator_vif.map((row) => [row.construct, row.indicator, formatOptional(row.vif, 4), vifStatus(row.vif)])} /> : null}
    {assessment?.cross_loadings.length ? <SectionTable title="Cross-loadings" note="Each indicator should usually load highest on its assigned construct." columns={["Indicator", "Assigned construct", "Compared construct", "Loading"]} rows={assessment.cross_loadings.map((row) => [row.indicator, row.assigned_construct, row.construct, row.loading.toFixed(6)])} /> : null}
    <SectionTable title="Structural paths for diagram focus" note="This helper keeps measurement review linked to the model canvas." columns={["Path", "Coefficient"]} rows={result.paths.map((path) => [pathLabel(path.source, path.target), path.coefficient.toFixed(6)])} activeRowIndexes={activeIndexes(result.paths, activePath)} onRowClick={(_, index) => focusPath(result.paths[index].source, result.paths[index].target)} />
  </div>;
}

function StructuralResults({ run, focusPath, activePath }: { run: AnalysisRun; focusPath: (source: string, target: string) => void; activePath: { source: string; target: string } | null }) {
  const result = run.result!;
  const assessment = run.assessment;
  return <div className="result-sections" tabIndex={0} role="region" aria-label="Structural model results">
    <SectionTable title="Path coefficients" note="Bootstrapped t values and p values appear in Inference after bootstrap is enabled." columns={["Path", "Coefficient", "Direction"]} rows={result.paths.map((path) => [pathLabel(path.source, path.target), path.coefficient.toFixed(6), coefficientDirection(path.coefficient)])} activeRowIndexes={activeIndexes(result.paths, activePath)} onRowClick={(_, index) => focusPath(result.paths[index].source, result.paths[index].target)} />
    <EffectsTable result={result} activePath={activePath} />
    {(result.control_estimates ?? []).length ? <SectionTable title="Control paths" columns={["Control path", "Coefficient"]} rows={result.control_estimates!.map((control) => [pathLabel(control.source, control.target), control.coefficient.toFixed(6)])} /> : null}
    {assessment?.structural_quality.length ? <SectionTable title="R² and adjusted R²" note="Use R² for explained variance and adjusted R² when comparing models with different predictor counts." columns={["Construct", "Predictors", "R²", "Adjusted R²", "Interpretation"]} rows={assessment.structural_quality.map((row) => [row.construct, String(row.predictor_count), row.r_squared.toFixed(4), formatOptional(row.adjusted_r_squared, 4), interpretR2(row.r_squared)])} /> : null}
    {assessment?.structural_vif.length ? <SectionTable title="Inner VIF" note="High VIF suggests predictor collinearity in the structural model." columns={["Target", "Predictor", "VIF", "Status"]} rows={assessment.structural_vif.map((row) => [row.target_construct, row.predictor_construct, formatOptional(row.vif, 4), vifStatus(row.vif)])} /> : null}
    {assessment?.f_squared.length ? <SectionTable title="Cohen f² effect sizes" note="f² describes how much an omitted predictor changes the target construct R²." columns={["Path", "R² included", "R² excluded", "f²", "Interpretation"]} rows={assessment.f_squared.map((row) => [pathLabel(row.source_construct, row.target_construct), row.included_r_squared.toFixed(4), formatOptional(row.excluded_r_squared, 4), formatOptional(row.f_squared, 4), interpretF2(row.f_squared)])} /> : null}
    {result.mediation?.estimates.length ? <MediationTable run={run} /> : null}
    {result.moderation?.estimates.length ? <ModerationTable run={run} /> : null}
  </div>;
}

function QualityResults({ assessment }: { assessment?: AssessmentResult }) {
  if (!assessment) return <SectionEmpty title="No assessment payload" detail="Run a PLS-SEM method with assessment outputs to review reliability and validity." />;
   return <div className="result-sections quality-summary" tabIndex={0} role="region" aria-label="measurement quality tables">
    <SectionTable title="Construct reliability and convergent validity" note="Common reporting columns for reflective PLS-SEM measurement model assessment." columns={["Construct", "Cronbach alpha", "rho_A", "rho_C", "AVE", "Quick check"]} rows={assessment.construct_quality.map((quality) => [
      quality.construct,
      formatOptional(quality.cronbach_alpha, 4),
      quality.rho_a == null ? formatDiagnosticCode(quality.rho_a_reason ?? "N/A") : quality.rho_a.toFixed(4),
      formatOptional(quality.rho_c, 4),
      formatOptional(quality.ave, 4),
      reliabilityStatus(quality.cronbach_alpha, quality.rho_c, quality.ave),
    ])} />
    <MatrixTable title="Fornell-Larcker criterion" note="Diagonal values should be read against construct correlations according to the documented QuickPLS convention." constructs={assessment.fornell_larcker.constructs} values={assessment.fornell_larcker.values} />
    {assessment.htmt_plus && <HtmtTable label="HTMT+" artifact={assessment.htmt_plus} />}
    {assessment.htmt_original && <HtmtTable label="Original HTMT" artifact={assessment.htmt_original} />}
    {assessment.htmt && !assessment.htmt_plus && <MatrixTable title="HTMT+ (legacy)" constructs={assessment.htmt.constructs} values={assessment.htmt.values} />}
    {assessment.cross_loadings.length ? <SectionTable title="Cross-loadings check" note="Confirm each indicator is strongest on its assigned construct." columns={["Indicator", "Assigned construct", "Compared construct", "Loading"]} rows={assessment.cross_loadings.map((row) => [row.indicator, row.assigned_construct, row.construct, row.loading.toFixed(6)])} /> : null}
  </div>;
}

function InferenceResults({ run }: { run: AnalysisRun }) {
  if (!run.bootstrap && !run.permutation) return <SectionEmpty title="Inference not run" detail="Enable bootstrap or permutation in Setup, rerun the model, then return here for t values, p values, and confidence intervals." />;
  return <div className="result-sections" tabIndex={0} role="region" aria-label="Inference results">
    {run.bootstrap ? <BootstrapSection run={run} /> : null}
    {run.permutation ? <PermutationSection run={run} /> : null}
  </div>;
}

function PredictionResults({ result, assessment }: { result: PlsResult; assessment?: AssessmentResult }) {
  const hasPredict = Boolean(result.predict);
  const hasBlindfolding = Boolean(assessment?.blindfolding);
  if (!hasPredict && !hasBlindfolding) return <SectionEmpty title="Prediction outputs not run" detail="Enable PLSpredict or blindfolding-related prediction settings, rerun the model, then review holdout metrics and Q² here." />;
  return <div className="result-sections" tabIndex={0} role="region" aria-label="Prediction results">
    {result.predict ? <><strong className="result-section-heading">PLSpredict holdout</strong><MethodWarnings warnings={result.predict.warnings} /><PlsPredictTable targets={result.predict.targets} />{result.predict.repeated_kfold ? <><strong className="result-section-heading">Repeated k-fold prediction</strong><MethodWarnings warnings={result.predict.repeated_kfold.warnings} /><PlsPredictTable targets={result.predict.repeated_kfold.targets} />{result.predict.repeated_kfold.cvpat?.length ? <CvpatTable comparisons={result.predict.repeated_kfold.cvpat} /> : null}</> : null}</> : null}
    {assessment?.blindfolding ? <SectionTable title="Blindfolding Q²" note={`Omission distance ${assessment.blindfolding.settings.omission_distance}.`} columns={["Construct", "Q²", "PRESS", "SSO"]} rows={assessment.blindfolding.constructs.map((row) => [row.construct, formatOptional(row.q_squared, 4), formatOptional(row.prediction_error_sum_squares, 6), formatOptional(row.observation_sum_squares, 6)])} /> : null}
  </div>;
}

function GroupResults({ result }: { result: PlsResult }) {
  if (!result.mga && !result.micom && !result.mga_permutation && !result.fimix && !result.segmentation && !result.ipma) return <SectionEmpty title="No group or segmentation payloads" detail="Configure MICOM/MGA, FIMIX-PLS, PLS-POS, or IPMA in Setup and rerun the model to populate this tab." />;
  return <div className="result-sections" tabIndex={0} role="region" aria-label="Groups and segmentation results"><MethodPayloadSections result={result} /></div>;
}

function DiagnosticsResults({ run }: { run: AnalysisRun }) {
  const result = run.result!;
  const assessment = run.assessment;
  return <div className="result-sections" tabIndex={0} role="region" aria-label="Diagnostics results">
    <SectionTable title="Run provenance" columns={["Field", "Value"]} rows={[["Method", run.method], ["Created", new Date(run.createdAt).toLocaleString()], ["Seed", String(run.seed)], ["Fingerprint", run.fingerprint], ["Converged", result.converged ? "yes" : "no"], ["Iterations", String(result.iterations)], ["Used observations", String(result.used_observations)], ["Omitted observations", String(result.omitted_observations)]]} />
    <SectionTable title="Warnings and scope status" columns={["Message"]} rows={[...run.warnings, ...result.warnings, ...(assessment?.warnings ?? [])].map((warning) => [scopeCopy(warning)])} />
    {assessment?.model_fit ? <SectionTable title="Correlation-residual fit" note="PLS-SEM approximate fit diagnostics should be interpreted within the documented QuickPLS scope." columns={["Model", "SRMR", "d_ULS"]} rows={[["Saturated", assessment.model_fit.saturated.srmr.toFixed(4), assessment.model_fit.saturated.d_uls.toFixed(6)], ["Estimated", assessment.model_fit.estimated.srmr.toFixed(4), assessment.model_fit.estimated.d_uls.toFixed(6)]]} /> : null}
    {result.plsc || result.wpls || result.cca || result.cta_pls || result.endogeneity || result.nonlinear_effects || result.moderated_mediation || result.cbsem || result.gsca || result.regression || result.nca || result.pca ? <MethodPayloadSections result={result} /> : null}
  </div>;
}

function ComparisonResults() {
  return <SectionEmpty title="Comparison workflow" detail="Run at least two compatible models to compare path coefficients, R², diagnostics, and export-ready differences here." />;
}

function BootstrapSection({ run }: { run: AnalysisRun }) {
  const bootstrap = run.bootstrap!;
  return <div className="bootstrap-summary">
    <div className="bootstrap-meta"><strong>Bootstrap replicates</strong><span>{bootstrap.usable_replicates} usable</span><span>{bootstrap.failed_replicates.length} failed</span><span>{Math.round(bootstrap.percentile.confidence_level * 100)}% percentile CI</span>{bootstrap.bca && <span>{bootstrap.bca.jackknife_case_count} jackknife cases | BCa CI</span>}{bootstrap.studentized && <span>{bootstrap.studentized.inner_replicates} inner replicates | {bootstrap.studentized.failure ? "bootstrap-t failed" : "bootstrap-t CI"}</span>}</div>
    {bootstrap.studentized?.failure && <div className="inference-failure" role="alert"><strong>Bootstrap-t unavailable</strong><span>{bootstrap.studentized.failure.message}</span></div>}
    <div className="bootstrap-table-scroll result-table-scroll" tabIndex={0} role="region" aria-label={`${run.name} bootstrap parameter table`}><table><thead><tr><th>Parameter</th><th>Original</th><th>Mean</th><th>Bias</th><th>SE</th><th>t</th><th>p</th><th>Percentile lower</th><th>Percentile upper</th><th>BCa lower</th><th>BCa upper</th><th>Bootstrap-t lower</th><th>Bootstrap-t upper</th></tr></thead><tbody>
      {bootstrap.percentile.parameters.map((parameter) => { const bca = bootstrap.bca?.parameters.find((value) => value.parameter === parameter.parameter); const studentized = bootstrap.studentized?.parameters.find((value) => value.parameter === parameter.parameter); return <tr key={parameter.parameter}>
        <td>{formatParameterIdentity(parameter.parameter)}</td><td>{parameter.original.toFixed(6)}</td><td>{parameter.bootstrap_mean.toFixed(6)}</td><td>{parameter.bias.toFixed(6)}</td><td>{parameter.standard_error.toFixed(6)}</td><td>{parameter.t_statistic?.toFixed(4) ?? "N/A"}</td><td>{formatPValue(parameter.p_value_two_sided)}</td><td>{parameter.lower.toFixed(6)}</td><td>{parameter.upper.toFixed(6)}</td><td title={bca?.unavailable_reason ?? undefined}>{bca?.lower?.toFixed(6) ?? "N/A"}</td><td title={bca?.unavailable_reason ?? undefined}>{bca?.upper?.toFixed(6) ?? "N/A"}</td><td title={studentized?.unavailable_reason ?? undefined}>{studentized?.lower?.toFixed(6) ?? "N/A"}</td><td title={studentized?.unavailable_reason ?? undefined}>{studentized?.upper?.toFixed(6) ?? "N/A"}</td>
      </tr>; })}
    </tbody></table></div>
  </div>;
}

function PermutationSection({ run }: { run: AnalysisRun }) {
  const permutation = run.permutation!;
  return <div className="bootstrap-summary">
    <div className="bootstrap-meta"><strong>Freedman-Lane permutation</strong><span>{permutation.plan.permutations} samples</span><span>two-sided finite-sample corrected p-values</span></div>
    <SectionTable title="permutation parameter table" columns={["Path", "Original coefficient", "Exceedances", "p"]} rows={permutation.parameters.map((parameter) => [formatParameterIdentity(parameter.parameter), parameter.original.toFixed(6), `${parameter.exceedances} / ${parameter.permutations}`, formatPValue(parameter.p_value_two_sided)])} />
  </div>;
}

function MediationTable({ run }: { run: AnalysisRun }) {
  const estimates = run.result?.mediation?.estimates ?? [];
  return <SectionTable title="Mediation effects" note={run.bootstrap ? "Bootstrap inference is shown where the matching indirect-effect parameter exists." : "Bootstrap was not run; p values and confidence intervals are unavailable."} columns={["Effect", "Direct", "Indirect", "Total", "Indirect p", "Percentile CI", "BCa CI", "Bootstrap-t CI", "VAF", "Class"]} rows={estimates.map((effect) => {
    const parameter = findBootstrapParameter(run.bootstrap, "indirect_effect", [effect.source, effect.target]);
    const bca = parameter ? findBcaParameter(run.bootstrap, parameter.parameter) : undefined;
    const studentized = parameter ? findStudentizedParameter(run.bootstrap, parameter.parameter) : undefined;
    return [pathLabel(effect.source, effect.target), effect.direct.toFixed(6), effect.indirect.toFixed(6), effect.total.toFixed(6), formatPValue(parameter?.p_value_two_sided), formatInterval(parameter?.lower, parameter?.upper), formatInterval(bca?.lower, bca?.upper), formatInterval(studentized?.lower, studentized?.upper), effect.variance_accounted_for?.toFixed(4) ?? "N/A", formatMediationClass(effect.classification)];
  })} />;
}

function EffectsTable({ result, activePath }: { result: PlsResult; activePath: { source: string; target: string } | null }) {
  return <SectionTable
    title="Total effects"
    note="Direct, indirect, and total effects are separated so mediation and serial path interpretation stays explicit."
    columns={["Effect", "Direct", "Indirect", "Total", "Effect type"]}
    rows={result.effects.map((effect) => [
      pathLabel(effect.source, effect.target),
      effect.direct.toFixed(6),
      effect.indirect.toFixed(6),
      effect.total.toFixed(6),
      effect.indirect === 0 ? "direct only" : effect.direct === 0 ? "indirect only" : "direct and indirect",
    ])}
    activeRowIndexes={activeIndexes(result.effects, activePath)}
  />;
}

function ModerationTable({ run }: { run: AnalysisRun }) {
  const estimates = run.result?.moderation?.estimates ?? [];
  return <SectionTable title="Moderation effects" note={run.bootstrap ? "Bootstrap inference is shown where the product-path parameter exists." : "Bootstrap was not run; p values and confidence intervals are unavailable."} columns={["Interaction", "Main effect", "Interaction", "Interaction p", "Percentile CI", "BCa CI", "Bootstrap-t CI", "Simple slopes"]} rows={estimates.map((effect) => {
    const parameter = findBootstrapParameter(run.bootstrap, "path", [effect.product_construct, effect.outcome]);
    const bca = parameter ? findBcaParameter(run.bootstrap, parameter.parameter) : undefined;
    const studentized = parameter ? findStudentizedParameter(run.bootstrap, parameter.parameter) : undefined;
    return [`${effect.predictor} x ${effect.moderator} -> ${effect.outcome}`, effect.predictor_main_effect?.toFixed(6) ?? "N/A", effect.interaction_effect.toFixed(6), formatPValue(parameter?.p_value_two_sided), formatInterval(parameter?.lower, parameter?.upper), formatInterval(bca?.lower, bca?.upper), formatInterval(studentized?.lower, studentized?.upper), effect.simple_slopes.length ? effect.simple_slopes.map((slope) => `${formatModeratorLevel(slope.moderator_score)}: ${slope.effect.toFixed(6)}`).join(" | ") : "N/A"];
  })} />;
}

function SectionTable({ title, note, columns, rows, onRowClick, activeRowIndexes }: { title: string; note?: string; columns: string[]; rows: string[][]; onRowClick?: (row: string[], index: number) => void; activeRowIndexes?: number[] }) {
  if (!rows.length) return null;
  const activeRows = new Set(activeRowIndexes ?? []);
  return <section className="result-table-section">
    <div className="result-section-title"><strong>{title}</strong>{note ? <span>{note}</span> : null}</div>
    <div className="bootstrap-table-scroll result-table-scroll" tabIndex={0} role="region" aria-label={`${title} table`}><table><thead><tr>{columns.map((column) => <th key={column}>{column}</th>)}</tr></thead><tbody>
      {rows.map((row, index) => <tr key={`${title}-${index}`} className={`${onRowClick ? "result-path-row" : ""}${activeRows.has(index) ? " active-result-row" : ""}`.trim() || undefined} aria-current={activeRows.has(index) ? "true" : undefined} onClick={onRowClick ? () => onRowClick(row, index) : undefined}>{row.map((cell, cellIndex) => <td key={`${title}-${index}-${cellIndex}`}>{cell}</td>)}</tr>)}
    </tbody></table></div>
  </section>;
}

function MatrixTable({ title, note, constructs, values }: { title: string; note?: string; constructs: string[]; values: Array<Array<number | null>> }) {
  return <section className="result-table-section">
    <div className="result-section-title"><strong>{title}</strong>{note ? <span>{note}</span> : null}</div>
    <div className="bootstrap-table-scroll result-table-scroll" tabIndex={0} role="region" aria-label={`${title} matrix`}><table><thead><tr><th>Construct</th>{constructs.map((construct) => <th key={construct}>{construct}</th>)}</tr></thead><tbody>
      {values.map((row, rowIndex) => <tr key={constructs[rowIndex]}><td>{constructs[rowIndex]}</td>{row.map((value, columnIndex) => <td key={constructs[columnIndex]}>{value?.toFixed(4) ?? "N/A"}</td>)}</tr>)}
    </tbody></table></div>
  </section>;
}

function MetricTile({ label, value, detail, tone = "neutral" }: { label: string; value: string; detail: string; tone?: "ok" | "warn" | "neutral" }) {
  return <article className={`result-metric-tile ${tone}`}><span>{label}</span><strong>{value}</strong><small>{detail}</small></article>;
}

function ResultGuidance({ title, items }: { title: string; items: string[] }) {
  return <section className="result-guidance"><strong>{title}</strong><ul>{items.map((item) => <li key={item}>{item}</li>)}</ul></section>;
}

function SectionEmpty({ title, detail }: { title: string; detail: string }) {
  return <div className="result-section-empty"><strong>{title}</strong><p>{detail}</p></div>;
}

function MethodPayloadSections({ result }: { result: PlsResult }) {
  return <div className="method-results">
    {result.plsc && <><strong>PLSc correction</strong><MethodWarnings warnings={result.plsc.warnings} /><table><thead><tr><th>Construct</th><th>rho_A</th></tr></thead><tbody>
      {result.plsc.reliabilities.map((row) => <tr key={row.construct}><td>{row.construct}</td><td>{row.rho_a.toFixed(6)}</td></tr>)}
    </tbody></table>
    <table><thead><tr><th>Correlation</th><th>Original</th><th>Corrected</th></tr></thead><tbody>
      {result.plsc.construct_correlations.map((row) => <tr key={`${row.left}-${row.right}`}><td>{row.left} - {row.right}</td><td>{row.original.toFixed(6)}</td><td>{row.corrected.toFixed(6)}</td></tr>)}
    </tbody></table>
    <table><thead><tr><th>Corrected path</th><th>Coefficient</th></tr></thead><tbody>
      {result.plsc.corrected_paths.map((path) => <tr key={`${path.source}-${path.target}`}><td>{path.source} -&gt; {path.target}</td><td>{path.coefficient.toFixed(6)}</td></tr>)}
    </tbody></table></>}

    {result.wpls && <><strong>WPLS case weights</strong><MethodWarnings warnings={result.wpls.warnings} /><table><thead><tr><th>Weight column</th><th>Weight sum</th><th>Effective sample size</th><th>Covariance</th></tr></thead><tbody>
      <tr><td>{result.wpls.case_weight_column}</td><td>{result.wpls.weight_sum.toFixed(6)}</td><td>{result.wpls.effective_sample_size.toFixed(4)}</td><td>{formatDiagnosticCode(result.wpls.covariance)}</td></tr>
    </tbody></table></>}

    {result.cca && <><strong>CCA composite residuals</strong><MethodWarnings warnings={result.cca.warnings} /><div className="method-metric"><span>Max absolute residual</span><b>{result.cca.max_absolute_residual.toFixed(6)}</b></div><table><thead><tr><th>Construct pair</th><th>Observed</th><th>Reproduced</th><th>Residual</th><th>|Residual|</th></tr></thead><tbody>
      {result.cca.correlations.map((row) => <tr key={`${row.left}-${row.right}`}><td>{row.left} - {row.right}</td><td>{row.observed.toFixed(6)}</td><td>{row.reproduced.toFixed(6)}</td><td>{row.residual.toFixed(6)}</td><td>{row.absolute_residual.toFixed(6)}</td></tr>)}
    </tbody></table></>}

    {result.pca && <><strong>Standalone PCA</strong><MethodWarnings warnings={result.pca.warnings} /><div className="method-metric"><span>Retained components</span><b>{result.pca.retained_components} by {formatDiagnosticCode(result.pca.component_rule)} | {result.pca.observations} observations</b></div><table><thead><tr><th>Component</th><th>Eigenvalue</th><th>Explained variance</th><th>Cumulative</th></tr></thead><tbody>
      {result.pca.components.map((row) => <tr key={row.component}><td>{row.component}</td><td>{row.eigenvalue.toFixed(6)}</td><td>{row.explained_variance.toFixed(4)}</td><td>{row.cumulative_variance.toFixed(4)}</td></tr>)}
    </tbody></table><div className="bootstrap-table-scroll"><table><thead><tr><th>Variable</th><th>Component</th><th>Loading</th><th>Weight</th></tr></thead><tbody>
      {result.pca.loadings.slice(0, 100).map((row) => <tr key={`${row.variable}-${row.component}`}><td>{row.variable}</td><td>{row.component}</td><td>{row.loading.toFixed(6)}</td><td>{row.weight.toFixed(6)}</td></tr>)}
    </tbody></table></div></>}

    {result.regression && <><strong>{result.regression.regression_type === "process" ? "PROCESS-style workflow" : result.regression.regression_type === "logistic" ? "Logistic regression" : "OLS regression"}</strong><MethodWarnings warnings={result.regression.warnings} /><div className="method-metric"><span>Model fit</span><b>{result.regression.outcome} | n {result.regression.observations} | R2 {formatOptional(result.regression.fit.r_squared ?? result.regression.fit.pseudo_r_squared, 4)} | AIC {result.regression.fit.aic.toFixed(4)}</b></div><table><thead><tr><th>Term</th><th>Estimate</th><th>SE</th><th>t/z</th><th>p</th><th>CI</th><th>Odds ratio</th></tr></thead><tbody>
      {result.regression.coefficients.map((row) => <tr key={row.term}><td>{row.term}</td><td>{row.estimate.toFixed(6)}</td><td>{row.standard_error.toFixed(6)}</td><td>{row.statistic.toFixed(4)}</td><td>{formatPValue(row.p_value_two_sided)}</td><td>{formatInterval(row.confidence_interval_lower, row.confidence_interval_upper)}</td><td>{formatOptional(row.odds_ratio, 6)}</td></tr>)}
    </tbody></table>
    {result.regression.process && <><strong>PROCESS effects</strong><MethodWarnings warnings={result.regression.process.warnings} /><table><thead><tr><th>Effect</th><th>Estimate</th><th>Bootstrap CI</th></tr></thead><tbody>
      {result.regression.process.effects.map((row) => <tr key={row.effect}><td>{formatDiagnosticCode(row.effect)}</td><td>{row.estimate.toFixed(6)}</td><td>{formatInterval(row.lower_percentile, row.upper_percentile)}</td></tr>)}
    </tbody></table>{result.regression.process.simple_slopes.length > 0 && <table><thead><tr><th>Moderator value</th><th>Simple slope</th></tr></thead><tbody>
      {result.regression.process.simple_slopes.map((row) => <tr key={row.moderator_value}><td>{row.moderator_value.toFixed(4)}</td><td>{row.slope.toFixed(6)}</td></tr>)}
    </tbody></table>}</>}
    </>}

    {result.nca && <><strong>NCA ceilings</strong><MethodWarnings warnings={result.nca.warnings} /><div className="method-metric"><span>Variables</span><b>{result.nca.x} &gt; {result.nca.y} | {result.nca.observations} observations | {result.nca.usable_permutations}/{result.nca.permutation_samples} permutations</b></div><table><thead><tr><th>Ceiling</th><th>Effect size</th><th>Permutation p</th></tr></thead><tbody>
      {result.nca.ceilings.map((row) => <tr key={row.ceiling}><td>{formatDiagnosticCode(row.ceiling)}</td><td>{row.effect_size.toFixed(6)}</td><td>{formatPValue(row.permutation_p_value)}</td></tr>)}
    </tbody></table><table><thead><tr><th>Outcome target %</th><th>Required X %</th></tr></thead><tbody>
      {result.nca.bottlenecks.map((row) => <tr key={row.outcome_percent}><td>{row.outcome_percent.toFixed(0)}</td><td>{row.required_x_percent.toFixed(4)}</td></tr>)}
    </tbody></table></>}

    {result.gsca && <><strong>GSCA component model</strong><MethodWarnings warnings={result.gsca.warnings} /><div className="method-metric"><span>Fit</span><b>FIT {result.gsca.fit.toFixed(4)} | AFIT {result.gsca.adjusted_fit.toFixed(4)} | GFI {result.gsca.gfi.toFixed(4)}</b></div><table><thead><tr><th>Path</th><th>Coefficient</th></tr></thead><tbody>
      {result.gsca.paths.map((path) => <tr key={`${path.source}-${path.target}`}><td>{path.source} -&gt; {path.target}</td><td>{path.coefficient.toFixed(6)}</td></tr>)}
    </tbody></table><div className="bootstrap-table-scroll"><table><thead><tr><th>Construct</th><th>Indicator</th><th>Weight</th><th>Loading</th></tr></thead><tbody>
      {result.gsca.weights.slice(0, 100).map((row) => <tr key={`${row.construct}-${row.indicator}`}><td>{row.construct}</td><td>{row.indicator}</td><td>{row.weight.toFixed(6)}</td><td>{row.loading.toFixed(6)}</td></tr>)}
    </tbody></table></div></>}

    {result.predict && <><strong>PLSpredict holdout</strong><MethodWarnings warnings={result.predict.warnings} /><div className="method-metric"><span>Split</span><b>{result.predict.training_observations} train / {result.predict.test_observations} test</b></div><PlsPredictTable targets={result.predict.targets} />
    {result.predict.repeated_kfold && <><strong>Repeated k-fold prediction</strong><MethodWarnings warnings={result.predict.repeated_kfold.warnings} /><div className="method-metric"><span>Plan</span><b>{result.predict.repeated_kfold.repeats} x {result.predict.repeated_kfold.folds} folds / {result.predict.repeated_kfold.total_test_observations} tests</b></div><PlsPredictTable targets={result.predict.repeated_kfold.targets} />{result.predict.repeated_kfold.cvpat?.length ? <CvpatTable comparisons={result.predict.repeated_kfold.cvpat} /> : null}</>}</>}

    {result.mga && <><strong>Bounded two-group MGA</strong><MethodWarnings warnings={result.mga.warnings} /><div className="method-metric"><span>Group column</span><b>{result.mga.group_column}</b></div><table><thead><tr><th>Path</th><th>Group A</th><th>Coef A</th><th>Group B</th><th>Coef B</th><th>Difference</th><th>SE</th><th>t</th><th>p</th></tr></thead><tbody>
      {result.mga.comparisons.map((row) => <tr key={`${row.source}-${row.target}-${row.group_a}-${row.group_b}`} title={row.warning ?? undefined}><td>{row.source} -&gt; {row.target}</td><td>{row.group_a}</td><td>{row.coefficient_a.toFixed(6)}</td><td>{row.group_b}</td><td>{row.coefficient_b.toFixed(6)}</td><td>{row.difference.toFixed(6)}</td><td>{row.standard_error?.toFixed(6) ?? "N/A"}</td><td>{row.t_statistic?.toFixed(4) ?? "N/A"}</td><td>{formatPValue(row.p_value_two_sided)}</td></tr>)}
    </tbody></table></>}

    {result.micom && <><strong>MICOM</strong><MethodWarnings warnings={result.micom.warnings} /><table><thead><tr><th>Construct</th><th>Composition p</th><th>Mean p</th><th>Variance p</th><th>Partial</th><th>Full</th></tr></thead><tbody>
      {result.micom.constructs.map((row) => <tr key={row.construct}><td>{row.construct}</td><td>{formatPValue(row.compositional_p_value)}</td><td>{formatPValue(row.mean_p_value)}</td><td>{formatPValue(row.variance_p_value)}</td><td>{row.partial_invariance ? "yes" : "no"}</td><td>{row.full_invariance ? "yes" : "no"}</td></tr>)}
    </tbody></table></>}

    {result.mga_permutation && <><strong>Permutation MGA</strong><MethodWarnings warnings={result.mga_permutation.warnings} /><table><thead><tr><th>Path</th><th>Difference</th><th>Empirical p</th><th>Percentile</th></tr></thead><tbody>
      {result.mga_permutation.comparisons.map((row) => <tr key={`${row.source}-${row.target}`}><td>{row.source} -&gt; {row.target}</td><td>{row.original_difference.toFixed(6)}</td><td>{formatPValue(row.empirical_p_value_two_sided)}</td><td>{row.percentile_rank?.toFixed(4) ?? "N/A"}</td></tr>)}
    </tbody></table></>}

    {result.fimix && <><strong>FIMIX-PLS</strong><MethodWarnings warnings={result.fimix.warnings} /><div className="method-metric"><span>Classes</span><b>{result.fimix.classes}; BIC {result.fimix.bic.toFixed(4)}</b></div><table><thead><tr><th>Class</th><th>Observations</th><th>Share</th><th>Path</th><th>Coefficient</th></tr></thead><tbody>
      {result.fimix.classes_summary.flatMap((item) => item.paths.map((path) => <tr key={`${item.class}-${path.source}-${path.target}`}><td>{item.class}</td><td>{item.observations}</td><td>{item.share.toFixed(4)}</td><td>{path.source} -&gt; {path.target}</td><td>{path.coefficient.toFixed(6)}</td></tr>))}
    </tbody></table></>}

    {result.ipma && <><strong>IPMA / cIPMA</strong><MethodWarnings warnings={result.ipma.warnings} /><table><thead><tr><th>Target</th><th>Construct</th><th>Importance</th><th>Performance</th></tr></thead><tbody>
      {result.ipma.constructs.map((row) => <tr key={`${row.target}-${row.construct}`}><td>{row.target}</td><td>{row.construct}</td><td>{row.importance.toFixed(6)}</td><td>{row.performance.toFixed(4)}</td></tr>)}
    </tbody></table></>}

    {result.cbsem && <><strong>CB-SEM / CFA ML</strong><MethodWarnings warnings={[...result.cbsem.warnings, ...result.cbsem.diagnostics]} /><div className="method-metric"><span>Fit</span><b>chi-square {result.cbsem.fit.chi_square.toFixed(4)} | df {result.cbsem.fit.degrees_of_freedom} | CFI {formatOptional(result.cbsem.fit.cfi, 4)} | RMSEA {formatOptional(result.cbsem.fit.rmsea, 4)} | SRMR {result.cbsem.fit.srmr.toFixed(4)}</b></div>
    <table><thead><tr><th>Parameter</th><th>Kind</th><th>Estimate</th><th>SE</th><th>z</th><th>p</th><th>Fixed</th></tr></thead><tbody>
      {result.cbsem.parameters.slice(0, 80).map((row) => <tr key={row.name} title={row.warning ?? undefined}><td>{row.lhs} - {row.rhs}</td><td>{formatDiagnosticCode(row.kind)}</td><td>{row.estimate.toFixed(6)}</td><td>{formatOptional(row.standard_error, 6)}</td><td>{formatOptional(row.z_statistic, 4)}</td><td>{formatPValue(row.p_value_two_sided)}</td><td>{row.fixed ? "yes" : "no"}</td></tr>)}
    </tbody></table>
    <table><thead><tr><th>Parameter</th><th>std_lv</th><th>std_all</th></tr></thead><tbody>
      {result.cbsem.standardized.slice(0, 80).map((row) => <tr key={row.name}><td>{row.lhs} - {row.rhs}</td><td>{row.std_lv.toFixed(6)}</td><td>{row.std_all.toFixed(6)}</td></tr>)}
    </tbody></table>
    <table><thead><tr><th>Modification</th><th>MI</th><th>EPC</th></tr></thead><tbody>
      {result.cbsem.modification_indices.slice(0, 40).map((row) => <tr key={`${row.kind}-${row.lhs}-${row.rhs}`}><td>{formatDiagnosticCode(row.kind)} {row.lhs} - {row.rhs}</td><td>{row.modification_index.toFixed(4)}</td><td>{formatOptional(row.expected_parameter_change, 6)}</td></tr>)}
    </tbody></table>
    {result.cbsem.bootstrap && <><strong>CB-SEM bootstrap intervals</strong><MethodWarnings warnings={result.cbsem.bootstrap.warnings} /><table><thead><tr><th>Parameter</th><th>Original</th><th>Lower</th><th>Upper</th></tr></thead><tbody>
      {result.cbsem.bootstrap.intervals.map((row) => <tr key={row.parameter}><td>{row.parameter}</td><td>{row.original.toFixed(6)}</td><td>{row.lower_percentile.toFixed(6)}</td><td>{row.upper_percentile.toFixed(6)}</td></tr>)}
    </tbody></table></>}
    {result.cbsem.multigroup && <><strong>CB-SEM multigroup invariance</strong><MethodWarnings warnings={result.cbsem.multigroup.warnings} /><table><thead><tr><th>Group</th><th>Observations</th><th>chi-square</th><th>df</th><th>CFI</th><th>RMSEA</th></tr></thead><tbody>
      {result.cbsem.multigroup.groups.map((row) => <tr key={row.group}><td>{row.group}</td><td>{row.observations}</td><td>{row.chi_square.toFixed(4)}</td><td>{row.degrees_of_freedom}</td><td>{formatOptional(row.cfi, 4)}</td><td>{formatOptional(row.rmsea, 4)}</td></tr>)}
    </tbody></table><table><thead><tr><th>Step</th><th>chi-square</th><th>df</th><th>Delta chi-square</th><th>Delta df</th><th>Delta CFI</th><th>Delta RMSEA</th></tr></thead><tbody>
      {result.cbsem.multigroup.invariance.map((row) => <tr key={row.step} title={row.warning ?? undefined}><td>{formatDiagnosticCode(row.step)}</td><td>{row.chi_square.toFixed(4)}</td><td>{row.degrees_of_freedom}</td><td>{formatOptional(row.delta_chi_square, 4)}</td><td>{row.delta_degrees_of_freedom ?? "N/A"}</td><td>{formatOptional(row.delta_cfi, 4)}</td><td>{formatOptional(row.delta_rmsea, 4)}</td></tr>)}
    </tbody></table></>}</>}

    {result.cta_pls && <><strong>CTA-PLS tetrads</strong><MethodWarnings warnings={result.cta_pls.warnings} /><table><thead><tr><th>Construct</th><th>Max |tetrad|</th></tr></thead><tbody>
      {Object.entries(result.cta_pls.max_absolute_tetrad_by_construct).map(([construct, value]) => <tr key={construct}><td>{construct}</td><td>{value.toFixed(6)}</td></tr>)}
    </tbody></table>
    <div className="bootstrap-table-scroll"><table><thead><tr><th>Construct</th><th>Indicators</th><th>Pairing</th><th>Tetrad</th><th>|Tetrad|</th></tr></thead><tbody>
      {result.cta_pls.estimates.map((row) => <tr key={`${row.construct}-${row.indicator_a}-${row.indicator_b}-${row.indicator_c}-${row.indicator_d}-${row.pairing}`}><td>{row.construct}</td><td>{row.indicator_a}, {row.indicator_b}, {row.indicator_c}, {row.indicator_d}</td><td>{formatDiagnosticCode(row.pairing)}</td><td>{row.tetrad.toFixed(6)}</td><td>{row.absolute_tetrad.toFixed(6)}</td></tr>)}
    </tbody></table></div></>}

    {result.endogeneity && <><strong>Gaussian-copula endogeneity</strong><MethodWarnings warnings={result.endogeneity.warnings} /><table><thead><tr><th>Path</th><th>Path coefficient</th><th>Copula coefficient</th><th>t</th><th>p</th><th>Skewness</th><th>Applicability</th></tr></thead><tbody>
      {result.endogeneity.estimates.map((row) => <tr key={`${row.source}-${row.target}`} title={row.warning ?? undefined}><td>{row.source} -&gt; {row.target}</td><td>{row.path_coefficient.toFixed(6)}</td><td>{row.copula_coefficient.toFixed(6)}</td><td>{row.t_statistic.toFixed(4)}</td><td>{formatPValue(row.p_value_two_sided)}</td><td>{row.predictor_skewness.toFixed(4)}</td><td>{row.applicable ? "screenable" : "weak"}</td></tr>)}
    </tbody></table></>}

    {result.nonlinear_effects && <><strong>Nonlinear effects</strong><MethodWarnings warnings={result.nonlinear_effects.warnings} /><table><thead><tr><th>Path</th><th>Linear</th><th>Quadratic</th><th>t</th><th>p</th><th>Linear R2</th><th>Augmented R2</th><th>Delta R2</th></tr></thead><tbody>
      {result.nonlinear_effects.estimates.map((row) => <tr key={`${row.source}-${row.target}`} title={row.warning ?? undefined}><td>{row.source} -&gt; {row.target}</td><td>{row.linear_coefficient.toFixed(6)}</td><td>{row.quadratic_coefficient.toFixed(6)}</td><td>{row.t_statistic.toFixed(4)}</td><td>{formatPValue(row.p_value_two_sided)}</td><td>{row.linear_r_squared.toFixed(4)}</td><td>{row.augmented_r_squared.toFixed(4)}</td><td>{row.delta_r_squared.toFixed(4)}</td></tr>)}
    </tbody></table></>}

    {result.moderated_mediation && <><strong>Moderated mediation</strong><MethodWarnings warnings={result.moderated_mediation.warnings} /><table><thead><tr><th>Effect</th><th>Stage</th><th>Index</th><th>Conditional indirect effects</th></tr></thead><tbody>
      {result.moderated_mediation.estimates.map((row) => <tr key={`${row.interaction}-${row.predictor}-${row.target}`} title={row.warning ?? undefined}><td>{row.predictor} via {row.mediator} to {row.target}</td><td>{formatDiagnosticCode(row.moderated_stage)}</td><td>{row.index_of_moderated_mediation.toFixed(6)}</td><td>{row.conditional_indirect_effects.map((effect) => `${formatModeratorLevel(effect.moderator_score)}: ${effect.indirect_effect.toFixed(6)}`).join(" | ")}</td></tr>)}
    </tbody></table></>}
  </div>;
}

function PlsPredictTable({ targets }: { targets: NonNullable<PlsResult["predict"]>["targets"] }) {
  return <table><thead><tr><th>Construct</th><th>Predictors</th><th>RMSE PLS</th><th>MAE PLS</th><th>RMSE benchmark</th><th>MAE benchmark</th><th>Q2 predict</th><th>RMSE LM</th><th>Q2 LM</th></tr></thead><tbody>
    {targets.map((row) => <tr key={row.construct}><td>{row.construct}</td><td>{row.predictor_count}</td><td>{row.rmse_pls.toFixed(6)}</td><td>{row.mae_pls.toFixed(6)}</td><td>{row.rmse_benchmark.toFixed(6)}</td><td>{row.mae_benchmark.toFixed(6)}</td><td>{row.q_squared_predict?.toFixed(6) ?? "N/A"}</td><td>{row.rmse_lm?.toFixed(6) ?? "N/A"}</td><td>{row.q_squared_predict_lm?.toFixed(6) ?? "N/A"}</td></tr>)}
  </tbody></table>;
}

function CvpatTable({ comparisons }: { comparisons: NonNullable<NonNullable<PlsResult["predict"]>["repeated_kfold"]>["cvpat"] }) {
  return <><strong>CVPAT paired loss comparisons</strong><table><thead><tr><th>Target</th><th>Comparison</th><th>Mean loss diff</th><th>SE</th><th>t</th><th>p</th><th>Preferred</th></tr></thead><tbody>
    {(comparisons ?? []).map((row) => <tr key={`${row.target}-${row.comparison}`} title={row.warning ?? undefined}><td>{row.target}</td><td>{formatDiagnosticCode(row.comparison)}</td><td>{row.mean_loss_difference.toFixed(6)}</td><td>{row.standard_error?.toFixed(6) ?? "N/A"}</td><td>{row.t_statistic?.toFixed(4) ?? "N/A"}</td><td>{formatPValue(row.p_value_two_sided)}</td><td>{formatDiagnosticCode(row.preferred_model)}</td></tr>)}
  </tbody></table></>;
}

function MethodWarnings({ warnings }: { warnings: string[] }) {
  if (!warnings.length) return null;
  return <ul className="method-warnings">{warnings.map((warning) => <li key={warning}>{warning}</li>)}</ul>;
}

function HtmtTable({ label, artifact }: { label: string; artifact: HtmtAssessment }) {
  return <><strong>{label}</strong><table><thead><tr><th>Construct</th>{artifact.constructs.map((construct) => <th key={construct}>{construct}</th>)}</tr></thead><tbody>
    {artifact.cells.map((row, rowIndex) => <tr key={artifact.constructs[rowIndex]}><th>{artifact.constructs[rowIndex]}</th>{row.map((cell, columnIndex) => <td key={artifact.constructs[columnIndex]} title={cell.reason ? formatDiagnosticCode(cell.reason) : undefined}>{cell.value?.toFixed(4) ?? formatDiagnosticCode(cell.reason ?? cell.status)}</td>)}</tr>)}
  </tbody></table></>;
}

function formatDiagnosticCode(code: string) {
  return code.replace(/^(rho_a|htmt)\./, "").replaceAll("_", " ");
}

function formatMediationClass(code: string) {
  return code.replaceAll("_", " ");
}

function formatPValue(value: number | null | undefined) {
  if (value == null) return "N/A";
  return value < 0.0001 ? "<0.0001" : value.toFixed(4);
}

function formatInterval(lower: number | null | undefined, upper: number | null | undefined) {
  if (lower == null || upper == null) return "N/A";
  return `${lower.toFixed(6)} to ${upper.toFixed(6)}`;
}

function formatOptional(value: number | null | undefined, digits: number) {
  return value == null || !Number.isFinite(value) ? "N/A" : value.toFixed(digits);
}

function formatModeratorLevel(value: number) {
  if (value === -1) return "-1 SD";
  if (value === 0) return "Mean";
  if (value === 1) return "+1 SD";
  return value.toFixed(2);
}

function pathLabel(source: string, target: string) {
  return `${source} -> ${target}`;
}

function activeIndexes<T extends { source: string; target: string }>(rows: T[], activePath: { source: string; target: string } | null) {
  if (!activePath) return [];
  return rows.map((row, index) => row.source === activePath.source && row.target === activePath.target ? index : -1).filter((index) => index >= 0);
}

function coefficientDirection(value: number) {
  if (Math.abs(value) < 0.000001) return "near zero";
  return value > 0 ? "positive" : "negative";
}

function loadingStatus(value: number) {
  const absolute = Math.abs(value);
  if (absolute >= 0.708) return "strong";
  if (absolute >= 0.4) return "review";
  return "weak";
}

function vifStatus(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value)) return "N/A";
  if (value >= 5) return "high";
  if (value >= 3.3) return "review";
  return "acceptable";
}

function interpretR2(value: number) {
  if (value >= 0.75) return "substantial";
  if (value >= 0.5) return "moderate";
  if (value >= 0.25) return "weak to moderate";
  return "weak";
}

function interpretF2(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value)) return "N/A";
  if (value >= 0.35) return "large";
  if (value >= 0.15) return "medium";
  if (value >= 0.02) return "small";
  return "very small";
}

function reliabilityStatus(alpha: number | null, rhoC: number | null, ave: number | null) {
  const issues: string[] = [];
  if (alpha != null && alpha < 0.7) issues.push("alpha");
  if (rhoC != null && rhoC < 0.7) issues.push("rho_C");
  if (ave != null && ave < 0.5) issues.push("AVE");
  return issues.length ? `review ${issues.join(", ")}` : "passes common cutoffs";
}

function scopeCopy(warning: string | undefined) {
  if (!warning) return "Validated for documented QuickPLS scope.";
  return warning.replace(/QuickPLS v\d+\.\d+\.\d+ supported scope/g, "documented QuickPLS supported scope");
}

function csvForCurrentResultTab(run: AnalysisRun, tab: ResultWorkspaceTab) {
  const result = run.result;
  const assessment = run.assessment;
  const rows: string[][] = [];
  if (!result) return "message\nNo result payload";
  if (tab === "measurement") {
    rows.push(["construct", "indicator", "loading", "weight"], ...result.outer_estimates.map((row) => [row.construct, row.indicator, row.loading.toString(), row.weight.toString()]));
  } else if (tab === "quality") {
    rows.push(["construct", "cronbach_alpha", "rho_a", "rho_c", "ave"], ...(assessment?.construct_quality ?? []).map((row) => [row.construct, String(row.cronbach_alpha ?? ""), String(row.rho_a ?? ""), String(row.rho_c ?? ""), String(row.ave ?? "")]));
  } else if (tab === "structural" || tab === "summary") {
    rows.push(["path", "coefficient"], ...result.paths.map((path) => [pathLabel(path.source, path.target), path.coefficient.toString()]));
  } else if (tab === "inference" && run.bootstrap) {
    rows.push(["parameter", "original", "mean", "bias", "se", "p"], ...run.bootstrap.percentile.parameters.map((parameter) => [formatParameterIdentity(parameter.parameter), String(parameter.original), String(parameter.bootstrap_mean), String(parameter.bias), String(parameter.standard_error), String(parameter.p_value_two_sided ?? "")]));
  } else if (tab === "prediction" && assessment?.blindfolding) {
    rows.push(["construct", "q2", "press", "sso"], ...assessment.blindfolding.constructs.map((row) => [row.construct, String(row.q_squared ?? ""), String(row.prediction_error_sum_squares ?? ""), String(row.observation_sum_squares ?? "")]));
  } else if (tab === "diagnostics") {
    rows.push(["field", "value"], ["method", run.method], ["seed", String(run.seed)], ["fingerprint", run.fingerprint], ["iterations", String(result.iterations)], ["observations", String(result.used_observations)]);
  } else {
    rows.push(["message"], [`No exportable ${tab} table is available for this run.`]);
  }
  return rows.map((row) => row.map(csvCell).join(",")).join("\n");
}

function csvCell(value: string) {
  return /[",\n]/.test(value) ? `"${value.replaceAll('"', '""')}"` : value;
}
