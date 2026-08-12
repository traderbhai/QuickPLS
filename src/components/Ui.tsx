import type { ButtonHTMLAttributes, HTMLAttributes, ReactNode } from "react";
import type { AnalysisRun, MethodDefinition } from "../types";
import { methodStatusDescription } from "../domain/methodStatus";

export function WorkspacePage({ children, className = "", ...props }: HTMLAttributes<HTMLElement> & { children: ReactNode }) {
  return <section {...props} className={`workspace-page qpls2-workspace qpls2-page-shell ${className}`.trim()}>{children}</section>;
}

export function PageHeader({ title, description, actions, kicker }: { title: string; description: string; actions?: ReactNode; kicker?: string }) {
  return <div className="page-heading page-heading-pro qpls2-workspace-hero">
    <div>
      {kicker ? <span className="qpls2-page-kicker">{kicker}</span> : null}
      <h1 className="qpls2-page-title">{title}</h1>
      <p className="qpls2-page-subtitle">{description}</p>
    </div>
    {actions ? <div className="page-actions">{actions}</div> : null}
  </div>;
}

export function StatusBadge({ status, children }: { status: "validated" | "experimental" | "unsupported" | "warning" | "info"; children: ReactNode }) {
  return <span className={`status-text ${status === "warning" || status === "info" ? "experimental" : status} ui-status-badge`}>{children}</span>;
}

export function ActionStrip({ children }: { children: ReactNode }) {
  return <div className="ui-action-strip">{children}</div>;
}

export function Panel({ title, description, actions, children, tone = "plain", className = "" }: { title: string; description?: string; actions?: ReactNode; children?: ReactNode; tone?: "plain" | "warning" | "validated" | "danger"; className?: string }) {
  return <section className={`qpls2-panel qpls2-design-panel ${tone} ${className}`.trim()}>
    <header>
      <div>
        <strong className="qpls2-panel-title">{title}</strong>
        {description ? <span>{description}</span> : null}
      </div>
      {actions ? <div className="qpls2-panel-actions">{actions}</div> : null}
    </header>
    {children ? <div className="qpls2-panel-body">{children}</div> : null}
  </section>;
}

export function Card({ title, description, children, tone = "plain" }: { title: string; description?: string; children?: ReactNode; tone?: "plain" | "warning" | "validated" }) {
  return <article className={`ui-card qpls2-design-card ${tone}`}>
    <header className="ui-card-heading"><strong className="qpls2-card-title">{title}</strong>{description ? <span className="qpls2-card-body">{description}</span> : null}</header>
    {children ? <div className="ui-card-actions">{children}</div> : null}
  </article>;
}

export function MetricCard({ label, value, detail, tone = "plain" }: { label: string; value: ReactNode; detail?: ReactNode; tone?: "plain" | "success" | "warning" | "danger" | "info" }) {
  return <article className={`qpls2-metric-card ${tone}`}>
    <span>{label}</span>
    <strong>{value}</strong>
    {detail ? <small>{detail}</small> : null}
  </article>;
}

export function CommandGroup({ label, children }: { label: string; children: ReactNode }) {
  return <div className="qpls2-command-group" aria-label={label}>
    <span>{label}</span>
    <div>{children}</div>
  </div>;
}

export function ToolbarButton({ active = false, reason, children, className = "", ...props }: ButtonHTMLAttributes<HTMLButtonElement> & { active?: boolean; reason?: string }) {
  return <button
    {...props}
    className={`qpls2-toolbar-button ${active ? "active" : ""} ${className}`.trim()}
    aria-disabled={props.disabled}
    title={props.disabled && reason ? reason : props.title}
  >
    {children}
    {props.disabled && reason ? <span className="qpls2-disabled-reason">{reason}</span> : null}
  </button>;
}

export function InlineNotice({ tone = "info", title, children, action }: { tone?: "info" | "success" | "warning" | "danger"; title: string; children?: ReactNode; action?: ReactNode }) {
  return <div className={`qpls2-inline-notice ${tone}`}>
    <div><strong>{title}</strong>{children ? <span>{children}</span> : null}</div>
    {action ? <div>{action}</div> : null}
  </div>;
}

export function TabStrip<T extends string>({ tabs, value, onChange, label }: { tabs: Array<{ id: T; label: string; count?: number }>; value: T; onChange: (value: T) => void; label: string }) {
  return <div className="ui-tab-strip" role="tablist" aria-label={label}>
    {tabs.map((tab) => <button key={tab.id} role="tab" aria-selected={value === tab.id} className={value === tab.id ? "active" : ""} onClick={() => onChange(tab.id)}>{tab.label}{tab.count !== undefined ? <span>{tab.count}</span> : null}</button>)}
  </div>;
}

export function EmptyState({ title, description, actions }: { title: string; description: string; actions?: ReactNode }) {
  return <div className="empty-state ui-empty-state"><h2>{title}</h2><p>{description}</p>{actions ? <div className="empty-actions">{actions}</div> : null}</div>;
}

export function MethodScopeDrawer({ method, open, onToggle }: { method?: MethodDefinition; open: boolean; onToggle: () => void }) {
  const status = method?.status ?? "validated";
  return <section className="method-scope-drawer" aria-label="Method scope transparency">
    <button type="button" className="scope-drawer-trigger" onClick={onToggle}>Why trust this result?</button>
    {open ? <div className="scope-drawer-panel">
      <div><strong>{method?.name ?? "Selected method"} scope</strong><StatusBadge status={status === "validated" ? "validated" : status === "experimental" ? "experimental" : "unsupported"}>{status === "validated" ? "Validated scope" : status}</StatusBadge></div>
      <p>{method ? methodStatusDescription(method) : "This result is interpreted only inside the documented QuickPLS supported scope."}</p>
      <dl>
        <div><dt>Validation basis</dt><dd>Published equations, independent references, deterministic fixtures, and QuickPLS audit artifacts.</dd></div>
        <div><dt>Tolerance policy</dt><dd>Deterministic values require documented agreement or known-difference notes before scoped promotion.</dd></div>
        <div><dt>Runtime dependency</dt><dd>QuickPLS runs offline. R/Rscript and external engines are validation-only, never runtime requirements.</dd></div>
        <div><dt>Known limits</dt><dd>Unsupported variants remain blocked or watermarked; QuickPLS does not claim SmartPLS project import or equivalence.</dd></div>
      </dl>
      <a href="docs/VALIDATION_ARTIFACT_INDEX_V1_0.md">Open validation artifact index</a>
    </div> : null}
  </section>;
}

export function MethodConfidencePanel({ run }: { run: AnalysisRun }) {
  return <section className="method-confidence-panel" aria-label="Method confidence">
    <header><strong>Method Confidence</strong><StatusBadge status="validated">Validated scope</StatusBadge></header>
    <dl>
      <div><dt>Method</dt><dd>{run.method}</dd></div>
      <div><dt>Seed</dt><dd>{run.seed}</dd></div>
      <div><dt>Data fingerprint</dt><dd>{run.fingerprint}</dd></div>
      <div><dt>Status</dt><dd>{run.status}</dd></div>
      <div><dt>Warnings</dt><dd>{run.warnings.filter((warning) => !warning.toLowerCase().includes("validated")).length || "none beyond scope status"}</dd></div>
    </dl>
  </section>;
}

export type ReportabilityStatus = "ready" | "review" | "issue" | "unavailable" | "not applicable";

export interface ReportabilityItem {
  id: string;
  label: string;
  status: ReportabilityStatus;
  evidence: string;
  action?: string;
}

export function ReportabilityChecklist({ items, onSelect }: { items: ReportabilityItem[]; onSelect?: (item: ReportabilityItem) => void }) {
  return <section className="reportability-checklist" data-v230-reportability-checklist="true" aria-label="PLS-SEM reportability checklist">
    <header><strong>Reportability checklist</strong><span>Threshold colors are methodological guidance, not universal pass/fail rules.</span></header>
    <div className="reportability-grid">
      {items.map((item) => <button key={item.id} type="button" className={`reportability-item ${item.status}`} onClick={() => onSelect?.(item)}>
        <span>{item.status}</span>
        <strong>{item.label}</strong>
        <small>{item.evidence}</small>
        {item.action ? <em>{item.action}</em> : null}
      </button>)}
    </div>
  </section>;
}

export function ResearchTable({ title, columns, rows, note, actions }: { title: string; columns: string[]; rows: string[][]; note?: string; actions?: ReactNode }) {
  return <section className="research-table-shell">
    <header>
      <div><strong>{title}</strong>{note ? <span>{note}</span> : null}</div>
      <div className="research-table-tools"><span>{rows.length} rows</span><span>{columns.length} columns</span>{actions}</div>
    </header>
    <div className="bootstrap-table-scroll research-table-scroll" tabIndex={0} role="region" aria-label={`${title} research table`}>
      <table><thead><tr>{columns.map((column, index) => <th key={column} className={index === 0 ? "sticky-col" : undefined}>{column}</th>)}</tr></thead><tbody>
        {rows.map((row, rowIndex) => <tr key={rowIndex}>{row.map((cell, columnIndex) => <td key={`${rowIndex}-${columnIndex}`} className={columnIndex === 0 ? "sticky-col" : undefined}>{cell}</td>)}</tr>)}
      </tbody></table>
    </div>
  </section>;
}
