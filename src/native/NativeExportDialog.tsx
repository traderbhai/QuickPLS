import { FileSpreadsheet, FileText, GitBranch, ShieldCheck } from "lucide-react";
import { useMemo, useRef, useState } from "react";
import { publicationDiagramSvg } from "../domain/publicationDiagram";
import { tablesToCsv, tablesToHtml, type ResultTable } from "../domain/resultTables";
import {
  exportNativeTextFile,
  exportNativeXlsxTables,
  isNativeDesktop,
} from "../services/projectService";
import { useWorkspace } from "../store";
import type { AnalysisRun } from "../types";
import { nativeCbsemDiagramRun } from "./nativeResults";
import { resolveAnalysisModel } from "./nativeRunModelSnapshot";
import { nativeOlsPredictionExportTable, nativePcaScoreExportTable, nativeRunProvenanceTable } from "./nativeExportTables";
import { isStandaloneNativeAnalysis } from "./nativeStandaloneAnalysis";

interface NativeExportDialogProps {
  run: AnalysisRun;
  tables: ResultTable[];
  close: () => void;
}

type ExportAction = "csv" | "html" | "reviewer" | "xlsx" | "svg" | "print";

interface ExportFeedback {
  tone: "neutral" | "success" | "error";
  message: string;
}

export interface NativeExportScope {
  includeModelDiagram: boolean;
  reviewerPackDetail: string;
  printDetail: string;
}

export function nativeExportScope(run: Pick<AnalysisRun, "result" | "provenance">): NativeExportScope {
  const isGroupComparison = Boolean(run.result?.mga);
  const isStandalone = isStandaloneNativeAnalysis(run.provenance?.method);
  const tablesOnly = isGroupComparison || isStandalone;
  return {
    includeModelDiagram: !tablesOnly,
    reviewerPackDetail: tablesOnly
      ? "Results tables and run provenance"
      : "Diagram, results, and run provenance",
    printDetail: isGroupComparison
      ? "Print the selected MGA results table"
      : isStandalone
        ? "Print the selected standalone-analysis results table"
      : "Use the Windows print dialog",
  };
}

export function nativeReviewerPackHtml(tablesHtml: string, modelSvg: string | null): string {
  if (!modelSvg) return tablesHtml;
  return tablesHtml.replace("</body>", `<section><h2>Model estimates</h2>${modelSvg}</section></body>`);
}

function readableError(error: unknown) {
  if (error instanceof Error && error.message) return error.message;
  if (typeof error === "string" && error.trim()) return error;
  return "The desktop export service did not provide an error message.";
}

function fileNameFromPath(path: string, fallback: string) {
  return path.split(/[\\/]/).filter(Boolean).at(-1) ?? fallback;
}

function download(name: string, contents: string, type: string) {
  const url = URL.createObjectURL(new Blob([contents], { type }));
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = name;
  anchor.click();
  window.setTimeout(() => URL.revokeObjectURL(url), 0);
}

export default function NativeExportDialog({ run, tables, close }: NativeExportDialogProps) {
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const settings = useWorkspace((state) => state.publicationDiagramSettings);
  const layout = useWorkspace((state) => state.diagramLayout);
  const exportScope = nativeExportScope(run);
  const svg = useMemo(() => {
    if (!exportScope.includeModelDiagram) return null;
    const model = resolveAnalysisModel(run, nodes, edges, layout);
    return publicationDiagramSvg(model.nodes, model.edges, nativeCbsemDiagramRun(run), settings, model.diagramLayout);
  }, [edges, exportScope.includeModelDiagram, layout, nodes, run, settings]);
  const nativeDesktop = isNativeDesktop();
  const busyRef = useRef(false);
  const [busy, setBusy] = useState<ExportAction | null>(null);
  const [feedback, setFeedback] = useState<ExportFeedback | null>(null);
  const provenanceStatus = tables.some((table) => table.status === "experimental") ? "experimental" : "validated";
  const tablesWithProvenance = useMemo(
    () => {
      const pcaScores = nativePcaScoreExportTable(run);
      const olsPredictions = nativeOlsPredictionExportTable(run);
      return [
        ...tables.filter((table) => table.id !== "run_provenance" && table.id !== "pca_scores" && table.id !== "ols_fitted_residuals"),
        ...(pcaScores ? [pcaScores] : []),
        ...(olsPredictions ? [olsPredictions] : []),
        nativeRunProvenanceTable(run, provenanceStatus),
      ];
    },
    [provenanceStatus, run, tables],
  );
  const csv = useMemo(() => tablesToCsv(tablesWithProvenance), [tablesWithProvenance]);
  const html = useMemo(() => tablesToHtml(tablesWithProvenance), [tablesWithProvenance]);
  const reviewerHtml = useMemo(
    () => nativeReviewerPackHtml(html, svg),
    [html, svg],
  );

  const runExport = async (
    action: ExportAction,
    fallbackFileName: string,
    operation: () => Promise<string | null>,
  ) => {
    if (busyRef.current) return;
    busyRef.current = true;
    setBusy(action);
    setFeedback({
      tone: "neutral",
      message: nativeDesktop ? "Choose a destination in the Save dialog." : "Preparing the download.",
    });
    try {
      const path = await operation();
      if (!path) {
        setFeedback({ tone: "neutral", message: "Export cancelled. No file was created." });
        return;
      }
      const name = fileNameFromPath(path, fallbackFileName);
      setFeedback({
        tone: "success",
        message: nativeDesktop ? `Saved ${name}.` : `Downloaded ${name}.`,
      });
    } catch (error) {
      setFeedback({ tone: "error", message: `Export failed: ${readableError(error)}` });
    } finally {
      busyRef.current = false;
      setBusy(null);
    }
  };

  const exportText = (
    action: Exclude<ExportAction, "xlsx" | "print">,
    fileName: string,
    filterName: string,
    extension: "csv" | "html" | "svg",
    contents: string,
    mediaType: string,
  ) => {
    void runExport(action, fileName, async () => {
      if (nativeDesktop) {
        return exportNativeTextFile({ defaultPath: fileName, filterName, extension, contents });
      }
      download(fileName, contents, mediaType);
      return fileName;
    });
  };

  const printResults = () => {
    if (busyRef.current) return;
    busyRef.current = true;
    setBusy("print");
    setFeedback({ tone: "neutral", message: "Opening the Windows print dialog." });
    window.setTimeout(() => {
      try {
        window.print();
        setFeedback({ tone: "success", message: "Print dialog closed." });
      } catch (error) {
        setFeedback({ tone: "error", message: `Print failed: ${readableError(error)}` });
      } finally {
        busyRef.current = false;
        setBusy(null);
      }
    }, 0);
  };

  const isBusy = busy !== null;
  const detail = (action: ExportAction, idle: string) => busy === action ? "Working..." : idle;

  return <div className="nd-export-dialog" aria-busy={isBusy}>
    <p>Export the selected completed run. Files are created locally.</p>
    <div className="nd-export-list">
      <button disabled={isBusy} onClick={() => exportText("csv", "quickpls-results.csv", "CSV tables", "csv", csv, "text/csv;charset=utf-8")}><FileSpreadsheet size={20} /><span><strong>CSV tables</strong><small>{detail("csv", "Current calculation tables")}</small></span></button>
      <button disabled={isBusy} onClick={() => exportText("html", "quickpls-report.html", "HTML report", "html", html, "text/html;charset=utf-8")}><FileText size={20} /><span><strong>HTML report</strong><small>{detail("html", "Tables and run provenance")}</small></span></button>
      <button disabled={isBusy} onClick={() => exportText("reviewer", "quickpls-reviewer-pack.html", "Reviewer pack", "html", reviewerHtml, "text/html;charset=utf-8")}><ShieldCheck size={20} /><span><strong>Reviewer pack</strong><small>{detail("reviewer", exportScope.reviewerPackDetail)}</small></span></button>
      <button disabled={!nativeDesktop || isBusy} title={nativeDesktop ? "Export an XLSX workbook" : "Available in the desktop runtime"} onClick={() => { void runExport("xlsx", "quickpls-result-tables.xlsx", () => exportNativeXlsxTables(tablesWithProvenance)); }}><FileSpreadsheet size={20} /><span><strong>XLSX workbook</strong><small>{detail("xlsx", "Native spreadsheet export")}</small></span></button>
      {svg ? <button disabled={isBusy} onClick={() => exportText("svg", "quickpls-model.svg", "SVG model diagram", "svg", svg, "image/svg+xml")}><GitBranch size={20} /><span><strong>Model diagram</strong><small>{detail("svg", "SVG with selected run estimates")}</small></span></button> : null}
      <button disabled={isBusy} onClick={printResults}><FileText size={20} /><span><strong>Print / PDF</strong><small>{detail("print", exportScope.printDetail)}</small></span></button>
    </div>
    {feedback ? <p id="nd-export-feedback" className={`nd-export-feedback ${feedback.tone}`} role={feedback.tone === "error" ? "alert" : "status"} aria-live="polite">{feedback.message}</p> : null}
    <footer><button disabled={isBusy} onClick={close}>Close</button></footer>
  </div>;
}
