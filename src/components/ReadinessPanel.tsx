import { AlertTriangle, CheckCircle2, CircleAlert } from "lucide-react";
import type { AnalysisReadiness } from "../domain/analysisReadiness";
import type { WorkspaceView } from "../types";

export function ReadinessPanel({ readiness, compact = false, onNavigate }: { readiness: AnalysisReadiness; compact?: boolean; onNavigate?: (view: WorkspaceView) => void }) {
  return <section className={compact ? "readiness-panel compact" : "readiness-panel"} aria-label="Analysis readiness">
    <div className="readiness-heading">
      <strong>Analysis readiness</strong>
      <span className={readiness.canRun ? "status-text validated" : "status-text experimental"}>
        {readiness.canRun ? <CheckCircle2 size={15} /> : <CircleAlert size={15} />}
        {readiness.summary}
      </span>
    </div>
    <div className="readiness-grid">
      {readiness.items.map((item) => <div key={item.id} className={`readiness-item ${item.status}`}>
        {item.status === "ready" ? <CheckCircle2 size={16} /> : <AlertTriangle size={16} />}
        <div><strong>{item.label}</strong><span>{item.detail}</span>{item.actionLabel && item.actionView && onNavigate ? <button type="button" className="readiness-action" onClick={() => onNavigate(item.actionView!)}>{item.actionLabel}</button> : null}</div>
      </div>)}
    </div>
  </section>;
}
