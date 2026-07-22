import type { ReactNode } from "react";

export function PageHeader({ title, description, actions }: { title: string; description: string; actions?: ReactNode }) {
  return <div className="page-heading page-heading-pro">
    <div><h1>{title}</h1><p>{description}</p></div>
    {actions ? <div className="page-actions">{actions}</div> : null}
  </div>;
}

export function StatusBadge({ status, children }: { status: "validated" | "experimental" | "unsupported" | "warning" | "info"; children: ReactNode }) {
  return <span className={`status-text ${status === "warning" || status === "info" ? "experimental" : status} ui-status-badge`}>{children}</span>;
}

export function ActionStrip({ children }: { children: ReactNode }) {
  return <div className="ui-action-strip">{children}</div>;
}

export function Card({ title, description, children, tone = "plain" }: { title: string; description?: string; children?: ReactNode; tone?: "plain" | "warning" | "validated" }) {
  return <article className={`ui-card ${tone}`}>
    <div className="ui-card-heading"><strong>{title}</strong>{description ? <span>{description}</span> : null}</div>
    {children}
  </article>;
}

export function TabStrip<T extends string>({ tabs, value, onChange, label }: { tabs: Array<{ id: T; label: string; count?: number }>; value: T; onChange: (value: T) => void; label: string }) {
  return <div className="ui-tab-strip" role="tablist" aria-label={label}>
    {tabs.map((tab) => <button key={tab.id} role="tab" aria-selected={value === tab.id} className={value === tab.id ? "active" : ""} onClick={() => onChange(tab.id)}>{tab.label}{tab.count !== undefined ? <span>{tab.count}</span> : null}</button>)}
  </div>;
}

export function EmptyState({ title, description, actions }: { title: string; description: string; actions?: ReactNode }) {
  return <div className="empty-state ui-empty-state"><h2>{title}</h2><p>{description}</p>{actions ? <div className="empty-actions">{actions}</div> : null}</div>;
}
