import { useId } from "react";
import type {
  PlsSavedRunBicRowV1,
  PlsSavedRunComparisonDocumentV1,
  PlsSavedRunComparisonIssueV1,
  PlsSavedRunCvpatSnapshotV1,
  PlsSavedRunMetricValueV1,
} from "../domain/plsSavedRunComparisonV1";
import type { NativePlsSavedRunComparisonV1 } from "../native/nativePlsSavedRunComparisonV1";

export type PlsSavedRunComparisonPanelStateV1 =
  | NativePlsSavedRunComparisonV1
  | { status: "missing" }
  | { status: "loading" };

export interface PlsSavedRunComparisonPanelV1Props {
  state: PlsSavedRunComparisonPanelStateV1;
  firstName: string;
  secondName: string;
}

function displayNumber(value: number): string {
  if (Object.is(value, -0) || value === 0) return "0";
  return Number.isInteger(value) ? String(value) : String(Number(value.toPrecision(8)));
}

function displayMetric(metric: PlsSavedRunMetricValueV1): string {
  if (metric.value !== null) return displayNumber(metric.value);
  return `Not available (${(metric.missing_reason ?? "not_estimated").replaceAll("_", " ")})`;
}

function issueList(issues: readonly PlsSavedRunComparisonIssueV1[], label: string) {
  if (issues.length === 0) return null;
  return (
    <div className="pls-saved-comparison__issues" aria-label={label}>
      <ul>
        {issues.map((item) => (
          <li key={item.id}>
            <strong>{item.title}.</strong> {item.message}
          </li>
        ))}
      </ul>
    </div>
  );
}

function PredictionTable({ comparison, firstName, secondName }: {
  comparison: PlsSavedRunComparisonDocumentV1;
  firstName: string;
  secondName: string;
}) {
  if (comparison.prediction_rows.length === 0) return null;
  return (
    <div className="pls-saved-comparison__table-wrap">
      <table>
        <caption>Indicator-level PLSpredict metrics</caption>
        <thead>
          <tr>
            <th scope="col">Outcome and metric</th>
            <th scope="col">{firstName}</th>
            <th scope="col">{secondName}</th>
            <th scope="col">Change (second − first)</th>
          </tr>
        </thead>
        <tbody>
          {comparison.prediction_rows.flatMap((row) => row.metrics.map((metric) => (
            <tr key={`${row.id}:${metric.id}`}>
              <th scope="row">{row.construct} / {row.indicator} — {metric.label}</th>
              <td>{displayMetric(metric.first)}</td>
              <td>{displayMetric(metric.second)}</td>
              <td>{metric.change === null ? "Not available" : displayNumber(metric.change)}</td>
            </tr>
          )))}
        </tbody>
      </table>
    </div>
  );
}

function cvpatCells(snapshot: PlsSavedRunCvpatSnapshotV1) {
  return (
    <>
      <td>{displayMetric(snapshot.pls_mean_loss)}</td>
      <td>{displayMetric(snapshot.mean_loss_difference)}</td>
      <td>{displayMetric(snapshot.p_value_one_sided)}</td>
      <td>{snapshot.status}</td>
    </>
  );
}

function CvpatTable({ comparison, firstName, secondName }: {
  comparison: PlsSavedRunComparisonDocumentV1;
  firstName: string;
  secondName: string;
}) {
  if (comparison.cvpat_rows.length === 0) return null;
  return (
    <div className="pls-saved-comparison__table-wrap">
      <table>
        <caption>Stored CVPAT benchmark assessments for each model</caption>
        <thead>
          <tr>
            <th rowSpan={2} scope="col">Benchmark</th>
            <th colSpan={4} scope="colgroup">{firstName}</th>
            <th colSpan={4} scope="colgroup">{secondName}</th>
          </tr>
          <tr>
            <th scope="col">PLS mean loss</th>
            <th scope="col">Difference from benchmark</th>
            <th scope="col">One-sided p</th>
            <th scope="col">Status</th>
            <th scope="col">PLS mean loss</th>
            <th scope="col">Difference from benchmark</th>
            <th scope="col">One-sided p</th>
            <th scope="col">Status</th>
          </tr>
        </thead>
        <tbody>
          {comparison.cvpat_rows.map((row) => (
            <tr key={row.id}>
              <th scope="row">{row.benchmark}</th>
              {cvpatCells(row.first)}
              {cvpatCells(row.second)}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function preference(row: PlsSavedRunBicRowV1, firstName: string, secondName: string): string {
  if (row.preferred === "tie") return "Exact tie";
  return row.preferred === "first" ? `${firstName} has lower BIC` : `${secondName} has lower BIC`;
}

function displayWeight(value: number | null): string {
  return value === null ? "Not available" : `${(value * 100).toFixed(2)}%`;
}

function BicTable({ comparison, firstName, secondName }: {
  comparison: PlsSavedRunComparisonDocumentV1;
  firstName: string;
  secondName: string;
}) {
  if (comparison.bic_rows.length === 0) return null;
  return (
    <div className="pls-saved-comparison__table-wrap">
      <table>
        <caption>Prediction-oriented BIC and exact stored Akaike weights</caption>
        <thead>
          <tr>
            <th scope="col">Outcome</th>
            <th scope="col">{firstName} BIC</th>
            <th scope="col">{secondName} BIC</th>
            <th scope="col">Change (second − first)</th>
            <th scope="col">{firstName} weight</th>
            <th scope="col">{secondName} weight</th>
            <th scope="col">Lower-BIC result</th>
          </tr>
        </thead>
        <tbody>
          {comparison.bic_rows.map((row) => (
            <tr key={row.id}>
              <th scope="row">{row.outcome}</th>
              <td>{displayNumber(row.first_bic)}</td>
              <td>{displayNumber(row.second_bic)}</td>
              <td>{displayNumber(row.bic_change)}</td>
              <td>{displayWeight(row.first_akaike_weight)}</td>
              <td>{displayWeight(row.second_akaike_weight)}</td>
              <td>{preference(row, firstName, secondName)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** Accessible Internal/Labs view. Standard workflows receive `hidden` and render nothing. */
export function PlsSavedRunComparisonPanelV1({
  state,
  firstName,
  secondName,
}: PlsSavedRunComparisonPanelV1Props) {
  const headingId = useId();
  if (state.status === "hidden") return null;
  if (state.status === "missing") {
    return (
      <section className="pls-saved-comparison" aria-labelledby={headingId} data-surface="labs">
        <h2 id={headingId}>Compare saved PLS runs</h2>
        <p role="status">Select two completed runs from distinct model specifications.</p>
      </section>
    );
  }
  if (state.status === "loading") {
    return (
      <section className="pls-saved-comparison" aria-labelledby={headingId} aria-busy="true" data-surface="labs">
        <h2 id={headingId}>Compare saved PLS runs</h2>
        <p role="status">Checking data, outcomes, method, settings, and cross-validation design.</p>
      </section>
    );
  }
  if (state.status === "unavailable") {
    return (
      <section className="pls-saved-comparison" aria-labelledby={headingId} data-surface="labs">
        <h2 id={headingId}>Compare saved PLS runs</h2>
        <div role="alert">
          <strong>Comparison could not be prepared.</strong>
          <ul>{state.messages.map((message) => <li key={message}>{message}</li>)}</ul>
        </div>
      </section>
    );
  }
  if (state.status === "blocked") {
    return (
      <section className="pls-saved-comparison" aria-labelledby={headingId} data-surface="labs">
        <h2 id={headingId}>Compare saved PLS runs</h2>
        <div role="alert">
          <strong>These runs cannot be compared.</strong>
          {issueList(state.issues, "Required corrections")}
        </div>
      </section>
    );
  }
  const { comparison } = state;
  return (
    <section className="pls-saved-comparison" aria-labelledby={headingId} data-surface="labs">
      <header>
        <span className="method-status-chip" aria-label="Experimental feature">Experimental</span>
        <h2 id={headingId}>Saved PLS run comparison</h2>
        <p role="status">Compatible stored results are shown for {firstName} and {secondName}.</p>
      </header>
      <p>
        This descriptive view does not refit either model or run a paired CVPAT test between them.
        Missing criteria remain unavailable.
      </p>
      <PredictionTable comparison={comparison} firstName={firstName} secondName={secondName} />
      <CvpatTable comparison={comparison} firstName={firstName} secondName={secondName} />
      <BicTable comparison={comparison} firstName={firstName} secondName={secondName} />
      {issueList(comparison.issues, "Comparison notes")}
      <details>
        <summary>Method details</summary>
        <p>
          Compatibility requires the same immutable data, analysis method and version, analytical settings,
          prediction outcomes, and cross-validation assignment. The two scientific model digests must differ.
          Akaike weights are shown only when the canonical results store exact weights for the same two-model candidate set.
        </p>
      </details>
    </section>
  );
}
