import { AlertTriangle, FlaskConical } from "lucide-react";
import { useWorkspace } from "../store";
import type { HtmtAssessment, PlsResult } from "../types";
import { findBcaParameter, findBootstrapParameter, findStudentizedParameter, formatParameterIdentity } from "../domain/inference";
import { analysisReadiness } from "../domain/analysisReadiness";
import { isNativeDesktop } from "../services/projectService";
import { ReadinessPanel } from "./ReadinessPanel";

export function RunHistory() {
  const runs = useWorkspace((state) => state.runs);
  const setView = useWorkspace((state) => state.setView);
  const dataset = useWorkspace((state) => state.dataset);
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const settings = useWorkspace((state) => state.analysisSettings);
  const readiness = analysisReadiness({ dataset, nodes, edges, settings, nativeDesktop: isNativeDesktop() });

  if (runs.length === 0) return <section className="workspace-page">
    <div className="page-heading"><div><h1>Results</h1><p>Completed runs, immutable recipes, estimates, and provenance records.</p></div></div>
    <ReadinessPanel readiness={readiness} compact onNavigate={setView} />
    <div className="empty-state"><FlaskConical size={30} /><h2>No completed results</h2><p>{readiness.canRun ? "Run the selected method to create the first result." : readiness.blockers[0]?.detail ?? "Complete the analysis checklist before running."}</p><div className="empty-actions"><button className="secondary-button" onClick={() => setView("models")}>Open model</button><button className="secondary-button" onClick={() => setView("analyses")}>Check readiness</button></div></div>
  </section>;

  return <section className="workspace-page">
    <div className="page-heading"><div><h1>Results</h1><p>Completed runs, immutable analysis recipes, estimates, and provenance records.</p></div></div>
    <div className="run-list">{runs.map((run) => <article key={run.id} className="run-row">
      <div className="run-icon"><FlaskConical size={18} /></div>
      <div className="run-content"><strong>{run.name}</strong><p>{new Date(run.createdAt).toLocaleString()} | seed {run.seed} | fingerprint {run.fingerprint}</p><span><AlertTriangle size={13} />{run.warnings[0]}</span>
        {run.result && <div className="result-summary" tabIndex={0} role="region" aria-label={`${run.name} result summary`}>
          <div><b>{run.result.iterations}</b><small>Iterations</small></div>
          <div><b>{run.result.used_observations}</b><small>Observations</small></div>
          {Object.entries(run.result.r_squared).map(([construct, value]) => <div key={construct}><b>{value.toFixed(4)}</b><small>R2 {construct}</small></div>)}
          <table><thead><tr><th>Path</th><th>Coefficient</th></tr></thead><tbody>{run.result.paths.map((path) => <tr key={`${path.source}-${path.target}`}><td>{path.source} -&gt; {path.target}</td><td>{path.coefficient.toFixed(6)}</td></tr>)}</tbody></table>
          {(run.result.control_estimates ?? []).length > 0 && <><strong>Control paths</strong><table><thead><tr><th>Control</th><th>Coefficient</th></tr></thead><tbody>{run.result.control_estimates!.map((control) => <tr key={`${control.source}-${control.target}`}><td>{control.label ? `${control.label} ` : ""}{control.source} -&gt; {control.target}</td><td>{control.coefficient.toFixed(6)}</td></tr>)}</tbody></table></>}
          {(run.result.mediation?.estimates ?? []).length > 0 && <><strong>Mediation effects</strong><table><thead><tr><th>Effect</th><th>Direct</th><th>Indirect</th><th>Total</th><th>Indirect p</th><th>Percentile CI</th><th>BCa CI</th><th>Bootstrap-t CI</th><th>VAF</th><th>Class</th></tr></thead><tbody>
            {run.result.mediation!.estimates.map((effect) => {
              const parameter = findBootstrapParameter(run.bootstrap, "indirect_effect", [effect.source, effect.target]);
              const bca = parameter ? findBcaParameter(run.bootstrap, parameter.parameter) : undefined;
              const studentized = parameter ? findStudentizedParameter(run.bootstrap, parameter.parameter) : undefined;
              return <tr key={`${effect.source}-${effect.target}`}><td>{effect.source} -&gt; {effect.target}</td><td>{effect.direct.toFixed(6)}</td><td>{effect.indirect.toFixed(6)}</td><td>{effect.total.toFixed(6)}</td><td>{formatPValue(parameter?.p_value_two_sided)}</td><td>{formatInterval(parameter?.lower, parameter?.upper)}</td><td title={bca?.unavailable_reason ?? undefined}>{formatInterval(bca?.lower, bca?.upper)}</td><td title={studentized?.unavailable_reason ?? undefined}>{formatInterval(studentized?.lower, studentized?.upper)}</td><td>{effect.variance_accounted_for?.toFixed(4) ?? "N/A"}</td><td>{formatMediationClass(effect.classification)}</td></tr>;
            })}
          </tbody></table></>}
          {(run.result.moderation?.estimates ?? []).length > 0 && <><strong>Moderation effects</strong><table><thead><tr><th>Interaction</th><th>Main effect</th><th>Interaction</th><th>Interaction p</th><th>Percentile CI</th><th>BCa CI</th><th>Bootstrap-t CI</th><th>Simple slopes</th></tr></thead><tbody>
            {run.result.moderation!.estimates.map((effect) => {
              const parameter = findBootstrapParameter(run.bootstrap, "path", [effect.product_construct, effect.outcome]);
              const bca = parameter ? findBcaParameter(run.bootstrap, parameter.parameter) : undefined;
              const studentized = parameter ? findStudentizedParameter(run.bootstrap, parameter.parameter) : undefined;
              return <tr key={effect.interaction}>
                <td>{effect.predictor} x {effect.moderator} -&gt; {effect.outcome}</td>
                <td>{effect.predictor_main_effect?.toFixed(6) ?? "N/A"}</td>
                <td>{effect.interaction_effect.toFixed(6)}</td>
                <td>{formatPValue(parameter?.p_value_two_sided)}</td>
                <td>{formatInterval(parameter?.lower, parameter?.upper)}</td>
                <td title={bca?.unavailable_reason ?? undefined}>{formatInterval(bca?.lower, bca?.upper)}</td>
                <td title={studentized?.unavailable_reason ?? undefined}>{formatInterval(studentized?.lower, studentized?.upper)}</td>
                <td title={effect.warning ?? undefined}>{effect.simple_slopes.length ? effect.simple_slopes.map((slope) => `${formatModeratorLevel(slope.moderator_score)}: ${slope.effect.toFixed(6)}`).join(" | ") : "N/A"}</td>
              </tr>;
            })}
          </tbody></table></>}
          <MethodPayloadSections result={run.result} />
        </div>}
        {run.assessment && <div className="quality-summary" tabIndex={0} role="region" aria-label={`${run.name} measurement quality tables`}>
          <strong>Measurement quality</strong>
          <table><thead><tr><th>Construct</th><th>Alpha</th><th>rho_A</th><th>rho_C</th><th>AVE</th></tr></thead><tbody>
            {run.assessment.construct_quality.map((quality) => <tr key={quality.construct}>
              <td>{quality.construct}</td>
              <td>{quality.cronbach_alpha?.toFixed(4) ?? "N/A"}</td>
              <td>{quality.rho_a?.toFixed(4) ?? "N/A"}{[quality.rho_a_reason, ...(quality.rho_a_warning_codes ?? [])].filter((value): value is string => Boolean(value)).map((code) => <small className="rho-a-note" key={code}>{formatDiagnosticCode(code)}</small>)}</td>
              <td>{quality.rho_c?.toFixed(4) ?? "N/A"}</td>
              <td>{quality.ave?.toFixed(4) ?? "N/A"}</td>
            </tr>)}
          </tbody></table>
          {run.assessment.htmt_plus && <HtmtTable label="HTMT+" artifact={run.assessment.htmt_plus} />}
          {run.assessment.htmt_original && <HtmtTable label="Original HTMT" artifact={run.assessment.htmt_original} />}
          {run.assessment.htmt && !run.assessment.htmt_plus && <><strong>HTMT+ (legacy)</strong><table><thead><tr><th>Construct</th>{run.assessment.htmt.constructs.map((construct) => <th key={construct}>{construct}</th>)}</tr></thead><tbody>
            {run.assessment.htmt.values.map((row, rowIndex) => <tr key={run.assessment!.htmt!.constructs[rowIndex]}><th>{run.assessment!.htmt!.constructs[rowIndex]}</th>{row.map((value, columnIndex) => <td key={run.assessment!.htmt!.constructs[columnIndex]}>{value?.toFixed(4) ?? "N/A"}</td>)}</tr>)}
          </tbody></table></>}
          {(run.assessment.structural_quality ?? []).length > 0 && <><strong>Structural quality</strong><table><thead><tr><th>Construct</th><th>Predictors</th><th>R2</th><th>Adjusted R2</th></tr></thead><tbody>
            {(run.assessment.structural_quality ?? []).map((quality) => <tr key={quality.construct}><td>{quality.construct}</td><td>{quality.predictor_count}</td><td>{quality.r_squared.toFixed(4)}</td><td>{quality.adjusted_r_squared?.toFixed(4) ?? "N/A"}</td></tr>)}
          </tbody></table></>}
          {(run.assessment.structural_vif ?? []).length > 0 && <><strong>Inner VIF</strong><table><thead><tr><th>Target</th><th>Predictor</th><th>VIF</th></tr></thead><tbody>
            {(run.assessment.structural_vif ?? []).map((value) => <tr key={`${value.target_construct}-${value.predictor_construct}`}><td>{value.target_construct}</td><td>{value.predictor_construct}</td><td>{value.vif?.toFixed(4) ?? "N/A"}</td></tr>)}
          </tbody></table></>}
          {(run.assessment.formative_indicator_vif ?? []).length > 0 && <><strong>Formative indicator VIF</strong><table><thead><tr><th>Construct</th><th>Indicator</th><th>VIF</th></tr></thead><tbody>
            {(run.assessment.formative_indicator_vif ?? []).map((value) => <tr key={`${value.construct}-${value.indicator}`}><td>{value.construct}</td><td>{value.indicator}</td><td>{value.vif?.toFixed(4) ?? "N/A"}</td></tr>)}
          </tbody></table></>}
          {(run.assessment.f_squared ?? []).length > 0 && <><strong>Cohen f2 effect sizes</strong><table><thead><tr><th>Path</th><th>R2 included</th><th>R2 excluded</th><th>f2</th></tr></thead><tbody>
            {(run.assessment.f_squared ?? []).map((value) => <tr key={`${value.source_construct}-${value.target_construct}`}><td>{value.source_construct} -&gt; {value.target_construct}</td><td>{value.included_r_squared.toFixed(4)}</td><td>{value.excluded_r_squared?.toFixed(4) ?? "N/A"}</td><td>{value.f_squared?.toFixed(4) ?? "N/A"}</td></tr>)}
          </tbody></table></>}
          {run.assessment.model_fit && <><strong>Correlation-residual fit</strong><table><thead><tr><th>Model</th><th>SRMR</th><th>d_ULS</th></tr></thead><tbody>
            <tr><td>Saturated</td><td>{run.assessment.model_fit.saturated.srmr.toFixed(4)}</td><td>{run.assessment.model_fit.saturated.d_uls.toFixed(6)}</td></tr>
            <tr><td>Estimated</td><td>{run.assessment.model_fit.estimated.srmr.toFixed(4)}</td><td>{run.assessment.model_fit.estimated.d_uls.toFixed(6)}</td></tr>
          </tbody></table></>}
          {run.assessment.blindfolding && <><strong>Blindfolding Q2</strong><span>Omission distance {run.assessment.blindfolding.settings.omission_distance}</span><table><thead><tr><th>Construct</th><th>Q2</th><th>PRESS</th><th>SSO</th></tr></thead><tbody>
            {run.assessment.blindfolding.constructs.map((value) => <tr key={value.construct}><td>{value.construct}</td><td>{value.q_squared?.toFixed(4) ?? "N/A"}</td><td>{value.prediction_error_sum_squares?.toFixed(6) ?? "N/A"}</td><td>{value.observation_sum_squares?.toFixed(6) ?? "N/A"}</td></tr>)}
          </tbody></table></>}
        </div>}
        {run.bootstrap && <div className="bootstrap-summary">
          <div className="bootstrap-meta"><strong>Bootstrap replicates</strong><span>{run.bootstrap.usable_replicates} usable</span><span>{run.bootstrap.failed_replicates.length} failed</span><span>{Math.round(run.bootstrap.percentile.confidence_level * 100)}% percentile CI</span>{run.bootstrap.bca && <span>{run.bootstrap.bca.jackknife_case_count} jackknife cases | BCa CI</span>}{run.bootstrap.studentized && <span>{run.bootstrap.studentized.inner_replicates} inner replicates | {run.bootstrap.studentized.failure ? "bootstrap-t failed" : "bootstrap-t CI"}</span>}</div>
          {run.bootstrap.studentized?.failure && <div className="inference-failure" role="alert"><strong>Bootstrap-t unavailable</strong><span>{run.bootstrap.studentized.failure.message}</span></div>}
          <div className="bootstrap-table-scroll" tabIndex={0} role="region" aria-label={`${run.name} bootstrap parameter table`}><table><thead><tr><th>Parameter</th><th>Original</th><th>Mean</th><th>Bias</th><th>SE</th><th>t</th><th>p (two-sided)</th><th>Percentile lower</th><th>Percentile upper</th><th>BCa lower</th><th>BCa upper</th><th>Bootstrap-t lower</th><th>Bootstrap-t upper</th></tr></thead><tbody>
              {run.bootstrap.percentile.parameters.map((parameter) => { const bca = run.bootstrap!.bca?.parameters.find((value) => value.parameter === parameter.parameter); const studentized = run.bootstrap!.studentized?.parameters.find((value) => value.parameter === parameter.parameter); return <tr key={parameter.parameter}>
                <td>{formatParameterIdentity(parameter.parameter)}</td><td>{parameter.original.toFixed(6)}</td><td>{parameter.bootstrap_mean.toFixed(6)}</td><td>{parameter.bias.toFixed(6)}</td><td>{parameter.standard_error.toFixed(6)}</td><td>{parameter.t_statistic?.toFixed(4) ?? "N/A"}</td><td>{formatPValue(parameter.p_value_two_sided)}</td><td>{parameter.lower.toFixed(6)}</td><td>{parameter.upper.toFixed(6)}</td><td title={bca?.unavailable_reason ?? undefined}>{bca?.lower?.toFixed(6) ?? "N/A"}</td><td title={bca?.unavailable_reason ?? undefined}>{bca?.upper?.toFixed(6) ?? "N/A"}</td><td title={studentized?.unavailable_reason ?? undefined}>{studentized?.lower?.toFixed(6) ?? "N/A"}</td><td title={studentized?.unavailable_reason ?? undefined}>{studentized?.upper?.toFixed(6) ?? "N/A"}</td>
              </tr>; })}
            </tbody></table></div>
        </div>}
        {run.permutation && <div className="bootstrap-summary">
          <div className="bootstrap-meta"><strong>Freedman-Lane permutation</strong><span>{run.permutation.plan.permutations} samples</span><span>two-sided finite-sample corrected p-values</span></div>
          <div className="bootstrap-table-scroll" tabIndex={0} role="region" aria-label={`${run.name} permutation parameter table`}><table><thead><tr><th>Path</th><th>Original coefficient</th><th>Exceedances</th><th>p (two-sided)</th></tr></thead><tbody>
            {run.permutation.parameters.map((parameter) => <tr key={parameter.parameter}><td>{formatParameterIdentity(parameter.parameter)}</td><td>{parameter.original.toFixed(6)}</td><td>{parameter.exceedances} / {parameter.permutations}</td><td>{formatPValue(parameter.p_value_two_sided)}</td></tr>)}
          </tbody></table></div>
        </div>}
      </div>
      <div className="run-status">Scope checked</div>
    </article>)}</div>
  </section>;
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

    {result.cbsem && <><strong>CB-SEM / CFA ML beta</strong><MethodWarnings warnings={[...result.cbsem.warnings, ...result.cbsem.diagnostics]} /><div className="method-metric"><span>Fit</span><b>chi-square {result.cbsem.fit.chi_square.toFixed(4)} | df {result.cbsem.fit.degrees_of_freedom} | CFI {formatOptional(result.cbsem.fit.cfi, 4)} | RMSEA {formatOptional(result.cbsem.fit.rmsea, 4)} | SRMR {result.cbsem.fit.srmr.toFixed(4)}</b></div>
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
