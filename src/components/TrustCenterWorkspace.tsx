import { useEffect } from "react";
import { BookOpenCheck, ClipboardCheck, Database, FileCheck2, FlaskConical, LockKeyhole, Microscope, ScrollText, ShieldCheck } from "lucide-react";
import { methods } from "../data/sample";
import { analysisCatalogCapabilityEntriesV2 } from "../domain/analysisCatalogCapabilityV2";
import { methodApplicabilityFor } from "../domain/methodApplicability";
import { methodStatusDescription } from "../domain/methodStatus";
import { isNativeDesktop } from "../services/projectService";
import { useWorkspace } from "../store";
import type { AnalysisMethodId } from "../types";
import { Card, MetricCard, PageHeader, Panel, ResearchTable, StatusBadge, WorkspacePage } from "./Ui";

const methodDocs: Record<string, string> = {
  pls_pm: "docs/methods/PLS_PM_V1.md",
  bootstrap: "docs/methods/RESAMPLING_ENGINE_V4.md",
  permutation: "docs/methods/PERMUTATION_ENGINE_V1.md",
  plsc: "docs/methods/PLSC_V2.md",
  wpls: "docs/methods/PLS_WPLS_V1.md",
  cca: "docs/methods/PLS_CCA_V1.md",
  cta_pls: "docs/methods/PLS_CTA_PLS_V1.md",
  endogeneity: "docs/methods/PLS_GAUSSIAN_COPULA_ENDOGENEITY_V1.md",
  nonlinear_effects: "docs/methods/PLS_NONLINEAR_EFFECTS_V1.md",
  moderated_mediation: "docs/methods/PLS_MODERATED_MEDIATION_V1.md",
  predict: "docs/methods/PLSPREDICT_HOLDOUT_V1.md",
  mga: "docs/methods/MICOM_V1.md; docs/methods/PLS_MGA_PERMUTATION_V1.md",
  ipma: "docs/methods/IPMA_V1.md",
  cbsem: "docs/methods/CBSEM_ML_V1.md; docs/methods/CFA_ML_V1.md",
  pca: "docs/methods/PCA_V1.md",
  gsca: "docs/methods/GSCA_ALS_V2.md",
  regression: "docs/methods/REGRESSION_OLS_V1.md; docs/methods/REGRESSION_LOGISTIC_V2.md; docs/methods/REGRESSION_LOGISTIC_V1.md (legacy compatibility); docs/methods/PROCESS_V1.md",
  nca: "docs/methods/NCA_V2.md; docs/methods/NCA_V1.md (legacy compatibility)",
};

const validationIndexRows = [
  ["Supported setups", "docs/V1_SUPPORTED_SCOPE.md", "Lists the model, data, and settings supported by each method."],
  ["Compatibility matrix", "docs/METHOD_COMPATIBILITY.md", "Maps methods to Supported, Experimental, or Not available."],
  ["Known differences", "docs/V1_KNOWN_DIFFERENCES.md", "Documents convention differences and exclusions."],
  ["Method references", "docs/VALIDATION_ARTIFACT_INDEX_V1_0.md", "Lists equations, published references, and reproducible comparison material."],
  ["Interpretation guidance", "docs/METHOD_PROMOTION_CRITERIA.md", "Explains the scientific checks used before a method is shown as Supported."],
  ["Release notes", "docs/RELEASE_NOTES_V1_0.md", "Describes version changes and known limitations."],
];

const boundaryRows = [
  ["Runtime", "Offline desktop", "No account, activation server, telemetry, cloud sync, or remote computation is required."],
  ["External engines", "Not required", "R/Rscript, lavaan, cSEM, SEMinR, and Python references are not bundled or required at runtime."],
  ["SmartPLS", "No equivalence claim", "QuickPLS implements documented methods independently and does not import SmartPLS projects."],
  ["Exports", "Method-aware", "Exports identify Experimental results and omit options that were not run."],
  ["Scientific checks", "Reproducible comparisons", "Supported methods are checked with formulas, fixtures, independent references, simulations, and documented differences."],
];

const realDatasetProtocolRows = [
  ["Protocol", "docs/V2_12_0_REAL_DATASET_REVIEW_PROTOCOL.md", "Manual checklist for reviewing private datasets without committing private data."],
  ["Template", "validation/templates/real_dataset_issue_register_template.json", "Anonymized issue register for UI, workflow, method guidance, export, and unresolved-method notes."],
  ["Repository rule", "No private data", "Do not commit raw datasets, private .qpls files, value-revealing screenshots, or private exports."],
  ["Automation boundary", "Fixtures only", "Automated gates use bundled or generated fixtures; private datasets remain manual review inputs."],
];

const confidenceChecks = [
  { label: "Formula specification", detail: "Method equations, assumptions, and output definitions are documented and versioned.", icon: BookOpenCheck },
  { label: "Independent references", detail: "Published examples, hand fixtures, Python/R references, or external engines are used where applicable.", icon: FlaskConical },
  { label: "Deterministic tolerance", detail: "Reported deterministic coefficients target agreement within 1e-6 when conventions match.", icon: Microscope },
  { label: "Offline provenance", detail: "Runs preserve seed, worker count, data fingerprint, recipe, warnings, and engine version.", icon: Database },
];

function downloadTrustFile(filename: string, text: string, type = "text/plain") {
  const blob = new Blob([text], { type });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
}

function trustCsvCell(value: unknown) {
  const text = String(value ?? "");
  return `"${text.replace(/"/g, '""')}"`;
}

export function TrustCenterWorkspace() {
  const dataset = useWorkspace((state) => state.dataset);
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const settings = useWorkspace((state) => state.analysisSettings);
  const uiPreferences = useWorkspace((state) => state.uiPreferences);
  const runs = useWorkspace((state) => state.runs);
  const setView = useWorkspace((state) => state.setView);
  const currentMethod = methods.find((method) => method.id === settings.method);
  const latestRun = runs[0];
  const nativeDesktop = isNativeDesktop();
  const methodEntries = analysisCatalogCapabilityEntriesV2(methods, settings, {
    experimentalLabsEnabled: uiPreferences.experimentalLabsEnabled,
  });
  const currentMethodEntry = methodEntries.find((entry) => entry.method.id === settings.method);
  const currentAvailability = currentMethodEntry?.availability;

  const methodRows = methodEntries.map(({ method, settings: methodSettings, availability }) => {
    const applicability = methodApplicabilityFor(method.id as AnalysisMethodId, { dataset, nodes, edges, settings: methodSettings, nativeDesktop });
    return [
      method.name,
      method.family,
      availability.label ?? "Not available",
      applicability.status.replace("_", " "),
      applicability.reason,
      methodDocs[method.id] ?? "docs/METHOD_COMPATIBILITY.md",
    ];
  });

  const currentMethodRows = [
    ["Selected method", currentMethod?.name ?? settings.method],
    ["Availability", currentAvailability?.tier === "standard" ? "Supported for the listed requirements" : currentAvailability?.label ?? "Not available"],
    ["Applicability now", currentMethod ? methodApplicabilityFor(currentMethod.id as AnalysisMethodId, { dataset, nodes, edges, settings, nativeDesktop }).status.replace("_", " ") : "Unknown"],
    ["Dataset fingerprint", dataset.fingerprint ?? "Not imported into desktop project yet"],
    ["Saved runs", runs.length ? `${runs.length} completed/recorded run${runs.length === 1 ? "" : "s"}` : "No completed run yet"],
    ["Latest run", latestRun ? `${latestRun.name} | seed ${latestRun.seed} | ${latestRun.fingerprint}` : "No run provenance available yet"],
  ];

  useEffect(() => {
    const sendStatus = (message: string) => {
      window.dispatchEvent(new CustomEvent("quickpls:status-message", { detail: { message, tone: "success" } }));
    };

    const refreshEvidence = () => {
      sendStatus("Method references refreshed from the bundled QuickPLS documentation.");
    };
    const openMethodDoc = async () => {
      const docList = currentMethod ? methodDocs[currentMethod.id] ?? "docs/METHOD_COMPATIBILITY.md" : "docs/METHOD_COMPATIBILITY.md";
      await navigator.clipboard?.writeText(docList);
      sendStatus(`Copied method documentation path: ${docList}`);
    };
    const exportEvidenceIndex = () => {
      const sections = [
        ["Method information", ["Area", "Document", "Purpose"], validationIndexRows],
        ["Method compatibility", ["Method", "Family", "Availability", "Applicability now", "Reason", "Method specification"], methodRows],
        ["Product boundaries", ["Area", "Rule", "Meaning"], boundaryRows],
      ];
      const csv = sections.flatMap(([title, columns, rows]) => [
        [title],
        columns as string[],
        ...(rows as string[][]),
        [""],
      ]).map((row) => row.map(trustCsvCell).join(",")).join("\n");
      downloadTrustFile("quickpls-method-reference-index.csv", csv, "text/csv");
      sendStatus("Method reference index exported as CSV.");
    };

    window.addEventListener("quickpls:trust-refresh-evidence", refreshEvidence);
    window.addEventListener("quickpls:trust-open-method-doc", openMethodDoc);
    window.addEventListener("quickpls:trust-export-evidence-index", exportEvidenceIndex);
    return () => {
      window.removeEventListener("quickpls:trust-refresh-evidence", refreshEvidence);
      window.removeEventListener("quickpls:trust-open-method-doc", openMethodDoc);
      window.removeEventListener("quickpls:trust-export-evidence-index", exportEvidenceIndex);
    };
  }, [currentMethod, methodRows]);

  return <WorkspacePage className="trust-workspace trust-v2-workspace trust-v214-workspace trust-v219-workspace" data-v219-mockup-screen="trust">
    <PageHeader
      title="Methods & References"
      description="Review method requirements, applicability, assumptions, references, and known limitations before reporting results."
      actions={<StatusBadge status="experimental">Method guidance</StatusBadge>}
    />

    <section className="trust-v2-hero" aria-label="Methods and references overview">
      <div className="trust-v2-hero-copy">
        <span className="qpls2-page-kicker">Research confidence</span>
        <h2>Is this method appropriate for my analysis?</h2>
        <p>Review the question the method answers, its model and data requirements, main settings, assumptions, references, and known limitations. Run Details preserves the exact data fingerprint, recipe, seed, workers, method version, and warnings for each result.</p>
        <div className="trust-v2-hero-actions">
          <button className="qpls2-primary-action" onClick={() => setView("analyses")}><ShieldCheck size={16} />Review current method</button>
          <button className="qpls2-secondary-action" onClick={() => setView("reports")}><ScrollText size={16} />Review report details</button>
        </div>
      </div>
      <div className="trust-v2-current-method" aria-label="Current method confidence">
        <header><strong>{currentMethod?.name ?? "Selected method"}</strong><StatusBadge status={currentAvailability?.tier === "standard" ? "validated" : currentAvailability?.tier === "experimental" ? "experimental" : "unsupported"}>{currentAvailability?.tier === "standard" ? "Supported setup" : currentAvailability?.label ?? "Not available"}</StatusBadge></header>
        <p>{currentMethod
          ? currentAvailability?.selectable
            ? methodStatusDescription(currentMethod, settings)
            : "This method is not available for a new calculation with the current product requirements."
          : "Select a method in Setup to review its requirements."}</p>
        <dl>
          {currentMethodRows.map(([label, value]) => <div key={label}><dt>{label}</dt><dd>{value}</dd></div>)}
        </dl>
      </div>
    </section>

    <div className="trust-v2-confidence-grid trust-v214-confidence-grid">
      {confidenceChecks.map((check) => {
        const Icon = check.icon;
        return <MetricCard
          key={check.label}
          label={check.label}
          value={<Icon size={19} aria-hidden="true" />}
          detail={check.detail}
          tone="info"
        />;
      })}
    </div>

    <Panel
      title="Method information"
      description="Requirements, references, interpretation guidance, and known differences"
      actions={<FileCheck2 size={18} aria-hidden="true" />}
      className="trust-v2-panel"
    >
      <ResearchTable title="Method documents" columns={["Area", "Document", "Purpose"]} rows={validationIndexRows} />
    </Panel>

    <Panel
      title="Real dataset review protocol"
      description="Use this when reviewing private SEM datasets without storing private values in the repository."
      actions={<ClipboardCheck size={18} aria-hidden="true" />}
      className="trust-v2-panel real-dataset-protocol-panel"
      data-real-dataset-protocol-entrypoint="trust-center"
    >
      <ResearchTable title="Private dataset review safeguards" columns={["Area", "Artifact", "Purpose"]} rows={realDatasetProtocolRows} />
    </Panel>

    <Panel
      title="Method compatibility and applicability"
      description={`${methodRows.length} method entries evaluated against the current project`}
      actions={<ShieldCheck size={18} aria-hidden="true" />}
      className="trust-v2-panel"
    >
      <ResearchTable title="Method compatibility" columns={["Method", "Family", "Availability", "Applicability now", "Reason", "Method specification"]} rows={methodRows} />
    </Panel>

    <div className="qpls2-hero-grid trust-v2-policy-grid">
      <Card title="Requirements are specific" description="Supported means the current model, data, and settings meet the requirements listed in Method Details." tone="validated" />
      <Card title="Incompatible setups explain why" description="QuickPLS blocks incompatible choices and identifies the model, data, or setting that must change." />
      <Card title="Independent implementation" description="QuickPLS follows published methods and independent references; it does not import proprietary SmartPLS project files." />
    </div>

    <Panel
      title="Offline operation and interoperability"
      description="Runtime, external-reference, export, and file-compatibility rules"
      actions={<LockKeyhole size={18} aria-hidden="true" />}
      className="trust-v2-panel"
    >
      <ResearchTable title="Product boundaries" columns={["Area", "Rule", "Meaning"]} rows={boundaryRows} />
    </Panel>
  </WorkspacePage>;
}
