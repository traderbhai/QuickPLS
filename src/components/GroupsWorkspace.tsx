import { Users } from "lucide-react";
import { useMemo, useState } from "react";
import { useWorkspace } from "../store";
import type { PlsResult } from "../types";

type GroupTab = "MGA" | "MICOM" | "FIMIX" | "PLS-POS" | "IPMA";

export function GroupsWorkspace() {
  const runs = useWorkspace((state) => state.runs);
  const setView = useWorkspace((state) => state.setView);
  const groupRuns = useMemo(() => runs.filter((run) => hasGroupPayload(run.result)), [runs]);
  const [selectedRunId, setSelectedRunId] = useState(groupRuns.at(0)?.id ?? "");
  const selectedRun = groupRuns.find((run) => run.id === selectedRunId) ?? groupRuns.at(0);
  const result = selectedRun?.result;
  const availableTabs = groupTabs(result);
  const [activeTab, setActiveTab] = useState<GroupTab>("MGA");
  const tab = availableTabs.includes(activeTab) ? activeTab : availableTabs[0];

  if (!groupRuns.length || !result) {
    return <section className="workspace-page">
      <div className="page-heading"><div><h1>Groups</h1><p>Heterogeneity and segment diagnostics</p></div></div>
      <div className="empty-state"><Users size={30} /><h2>No group results</h2><p>Configure a group or segmentation method, then run it to create MICOM, MGA, FIMIX, PLS-POS, or IPMA output.</p><div className="empty-actions"><button className="secondary-button" onClick={() => setView("analyses")}>Configure method</button><button className="secondary-button" onClick={() => setView("run")}>Open run step</button></div></div>
    </section>;
  }

  return <section className="workspace-page">
    <div className="page-heading"><div><h1>Groups</h1><p>Experimental group and segmentation payloads remain watermarked until method validation gates pass.</p></div></div>
    <div className="analysis-settings">
      <div><strong>Group result</strong><span className="status-text experimental">experimental</span></div>
      <label>Saved run<select value={selectedRun?.id ?? ""} onChange={(event) => setSelectedRunId(event.target.value)}>
        {groupRuns.map((run) => <option key={run.id} value={run.id}>{run.name}</option>)}
      </select></label>
    </div>
    <div className="method-table">
      {availableTabs.map((item) => <button key={item} type="button" className={`method-row ${tab === item ? "selected" : ""}`} onClick={() => setActiveTab(item)}><strong>{item}</strong><span>Groups</span><span className="status-text experimental">experimental</span></button>)}
    </div>
    <div className="report-preview">
      {tab === "MGA" && result.mga && <MgaPanel result={result} />}
      {tab === "MICOM" && result.micom && <MicomPanel result={result} />}
      {tab === "FIMIX" && result.fimix && <FimixPanel result={result} />}
      {tab === "PLS-POS" && result.segmentation && <PosPanel result={result} />}
      {tab === "IPMA" && result.ipma && <IpmaPanel result={result} />}
    </div>
  </section>;
}

function hasGroupPayload(result: PlsResult | undefined) {
  return Boolean(result?.segmentation || result?.mga || result?.micom || result?.mga_permutation || result?.fimix || result?.ipma);
}

function groupTabs(result: PlsResult | undefined): GroupTab[] {
  return [
    result?.mga ? "MGA" : null,
    result?.micom ? "MICOM" : null,
    result?.fimix ? "FIMIX" : null,
    result?.segmentation ? "PLS-POS" : null,
    result?.ipma ? "IPMA" : null,
  ].filter(Boolean) as GroupTab[];
}

function MgaPanel({ result }: { result: PlsResult }) {
  const mga = result.mga!;
  return <>
    <article>
      <div><strong>Two-group MGA</strong><span className="status-text experimental">{mga.group_column}</span></div>
      <div className="group-metrics">
        <Metric label="Groups" value={mga.groups.map((group) => group.group).join(" / ")} />
        <Metric label="Observations" value={String(mga.groups.reduce((sum, group) => sum + group.observations, 0))} />
        <Metric label="Comparisons" value={String(mga.comparisons.length)} />
        <Metric label="Method" value={mga.method_version} />
      </div>
      <div className="bootstrap-table-scroll" tabIndex={0} role="region" aria-label="Two-group MGA comparisons table"><table><thead><tr><th>Path</th><th>Group A</th><th>Coef A</th><th>Group B</th><th>Coef B</th><th>Difference</th><th>SE</th><th>t</th><th>p</th></tr></thead><tbody>
        {mga.comparisons.map((comparison) => <tr key={`${comparison.source}-${comparison.target}-${comparison.group_a}-${comparison.group_b}`}><td>{comparison.source} -&gt; {comparison.target}</td><td>{comparison.group_a}</td><td>{comparison.coefficient_a.toFixed(6)}</td><td>{comparison.group_b}</td><td>{comparison.coefficient_b.toFixed(6)}</td><td>{comparison.difference.toFixed(6)}</td><td>{comparison.standard_error == null ? "N/A" : comparison.standard_error.toFixed(6)}</td><td>{comparison.t_statistic == null ? "N/A" : comparison.t_statistic.toFixed(4)}</td><td>{comparison.p_value_two_sided == null ? "N/A" : comparison.p_value_two_sided.toFixed(4)}</td></tr>)}
      </tbody></table></div>
    </article>
    {result.mga_permutation && <article>
      <div><strong>Permutation MGA</strong><span className="status-text experimental">{result.mga_permutation.usable_permutations} usable permutations</span></div>
      <div className="bootstrap-table-scroll" tabIndex={0} role="region" aria-label="Permutation MGA comparisons table"><table><thead><tr><th>Path</th><th>Original difference</th><th>Empirical p</th><th>Percentile</th></tr></thead><tbody>
        {result.mga_permutation.comparisons.map((row) => <tr key={`${row.source}-${row.target}`}><td>{row.source} -&gt; {row.target}</td><td>{row.original_difference.toFixed(6)}</td><td>{row.empirical_p_value_two_sided?.toFixed(4) ?? "N/A"}</td><td>{row.percentile_rank?.toFixed(4) ?? "N/A"}</td></tr>)}
      </tbody></table></div>
    </article>}
  </>;
}

function MicomPanel({ result }: { result: PlsResult }) {
  const micom = result.micom!;
  return <article>
    <div><strong>MICOM</strong><span className="status-text experimental">{micom.usable_permutations} usable permutations</span></div>
    <div className="bootstrap-table-scroll" tabIndex={0} role="region" aria-label="MICOM invariance table"><table><thead><tr><th>Construct</th><th>Configural</th><th>Composition r</th><th>Composition p</th><th>Mean p</th><th>Variance p</th><th>Partial</th><th>Full</th></tr></thead><tbody>
      {micom.constructs.map((row) => <tr key={row.construct}><td>{row.construct}</td><td>{row.configural_invariance ? "yes" : "no"}</td><td>{row.compositional_correlation.toFixed(6)}</td><td>{row.compositional_p_value?.toFixed(4) ?? "N/A"}</td><td>{row.mean_p_value?.toFixed(4) ?? "N/A"}</td><td>{row.variance_p_value?.toFixed(4) ?? "N/A"}</td><td>{row.partial_invariance ? "yes" : "no"}</td><td>{row.full_invariance ? "yes" : "no"}</td></tr>)}
    </tbody></table></div>
  </article>;
}

function FimixPanel({ result }: { result: PlsResult }) {
  const fimix = result.fimix!;
  return <>
    <article>
      <div><strong>FIMIX-PLS</strong><span className="status-text experimental">{fimix.classes} classes</span></div>
      <div className="group-metrics">
        <Metric label="Starts" value={String(fimix.starts)} />
        <Metric label="Log likelihood" value={fimix.log_likelihood.toFixed(4)} />
        <Metric label="BIC" value={fimix.bic.toFixed(4)} />
        <Metric label="Entropy" value={fimix.entropy.toFixed(4)} />
      </div>
    </article>
    <article>
      <div><strong>Class paths</strong><span className="status-text experimental">screening</span></div>
      <div className="bootstrap-table-scroll" tabIndex={0} role="region" aria-label="FIMIX class paths table"><table><thead><tr><th>Class</th><th>Observations</th><th>Share</th><th>Path</th><th>Coefficient</th><th>R2</th></tr></thead><tbody>
        {fimix.classes_summary.flatMap((item) => item.paths.map((path) => <tr key={`${item.class}-${path.source}-${path.target}`}><td>{item.class}</td><td>{item.observations}</td><td>{item.share.toFixed(4)}</td><td>{path.source} -&gt; {path.target}</td><td>{path.coefficient.toFixed(6)}</td><td>{item.r_squared[path.target]?.toFixed(4) ?? "N/A"}</td></tr>))}
      </tbody></table></div>
    </article>
  </>;
}

function PosPanel({ result }: { result: PlsResult }) {
  const segmentation = result.segmentation!;
  return <>
    <article>
      <div><strong>PLS-POS</strong><span className="status-text experimental">{segmentation.method_version}</span></div>
      <div className="group-metrics">
        <Metric label="Segments" value={`${segmentation.selected_segments} / ${segmentation.requested_segments}`} />
        <Metric label="Observations" value={String(segmentation.observations)} />
        <Metric label="Objective gain" value={segmentation.objective_improvement.toFixed(4)} />
        <Metric label="Max path separation" value={segmentation.max_path_separation.toFixed(4)} />
      </div>
      <p>{segmentation.algorithm.replaceAll("_", " ")}. {segmentation.assignment}</p>
    </article>
    <article>
      <div><strong>Segment paths</strong><span className="status-text experimental">screening</span></div>
      <div className="bootstrap-table-scroll" tabIndex={0} role="region" aria-label="PLS-POS segment paths table"><table><thead><tr><th>Segment</th><th>Observations</th><th>Share</th><th>Path</th><th>Coefficient</th><th>R2</th></tr></thead><tbody>
        {segmentation.segments.flatMap((segment) => segment.paths.map((path) => <tr key={`${segment.segment}-${path.source}-${path.target}`}><td>{segment.segment.replaceAll("_", " ")}</td><td>{segment.observations}</td><td>{segment.share.toFixed(4)}</td><td>{path.source} -&gt; {path.target}</td><td>{path.coefficient.toFixed(6)}</td><td>{segment.r_squared[path.target]?.toFixed(4) ?? "N/A"}</td></tr>))}
      </tbody></table></div>
    </article>
  </>;
}

function IpmaPanel({ result }: { result: PlsResult }) {
  const ipma = result.ipma!;
  return <article>
    <div><strong>IPMA / cIPMA</strong><span className="status-text experimental">{ipma.performance_scale}</span></div>
    <div className="bootstrap-table-scroll" tabIndex={0} role="region" aria-label="IPMA importance performance table"><table><thead><tr><th>Target</th><th>Construct</th><th>Importance</th><th>Performance</th><th>Score mean</th></tr></thead><tbody>
      {ipma.constructs.map((row) => <tr key={`${row.target}-${row.construct}`}><td>{row.target}</td><td>{row.construct}</td><td>{row.importance.toFixed(6)}</td><td>{row.performance.toFixed(4)}</td><td>{row.score_mean.toFixed(6)}</td></tr>)}
    </tbody></table></div>
  </article>;
}

function Metric({ label, value }: { label: string; value: string }) {
  return <div className="method-metric"><span>{label}</span><b>{value}</b></div>;
}
