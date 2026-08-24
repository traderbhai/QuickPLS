import {
  AlertTriangle,
  CheckCircle2,
  Database,
  Download,
  ShieldAlert,
} from "lucide-react";
import { useEffect, useId, useMemo, useState, type KeyboardEvent } from "react";
import {
  MultiModContractErrorV1,
  parseMultiModResultAttachmentV1,
  type MultiModAnalysisResultV1,
  type MultiModResultAttachmentV1,
  type MultimodIntervalV1,
  type MultimodProvenanceV1,
  type MultimodReplicateLedgerSummaryV1,
  type MultimodResultSidecarDescriptorV1,
} from "../domain/multimodContractsV1";
import type { ResultTable } from "../domain/resultTables";
import type { CanonicalResultDocumentV2 } from "../domain/canonicalResultDocumentV2";
import { CanonicalResultExportPanelV2 } from "./CanonicalResultExportPanelV2";
import { NativeResultTable } from "./NativeResultTable";
import {
  isNativeMultiModRawSidecarExportableV1,
  publishNativeMultiModRawSidecarV1,
  type NativeMultiModRawSidecarExportAuthorityV1,
} from "./nativeMultiModRawSidecarExportV1";

export type MultiModResultsTabV1 =
  | "eligibility"
  | "diagnostics"
  | "estimates"
  | "inference"
  | "exclusions"
  | "failures"
  | "provenance"
  | "sidecars";

type MultiModResultTableCategoryV1 = MultiModResultsTabV1;

interface MultiModResultTableV1 {
  readonly id: string;
  readonly category: MultiModResultTableCategoryV1;
  readonly title: string;
  readonly description: string;
  readonly columns: readonly string[];
  readonly rows: readonly (readonly string[])[];
  readonly rowKeys?: readonly string[];
}

interface MultiModResultsViewV1 {
  readonly familyLabel: string;
  readonly profileLabel: string;
  readonly provenance: MultimodProvenanceV1;
  readonly tables: readonly MultiModResultTableV1[];
}

export interface NativeMultiModResultReadinessV1 {
  readonly completeScientificResult: boolean;
  readonly reasons: readonly string[];
}

export interface NativeMultiModResultsV1Props {
  /** Must originate from parseMultiModResultAttachmentV1 or the strict V6 archive reader. */
  readonly validatedResult: MultiModResultAttachmentV1;
  /** Strict post-reopen canonical projection for semantic exports. */
  readonly canonicalDocument?: CanonicalResultDocumentV2;
  readonly rawSidecarExportAuthority?: NativeMultiModRawSidecarExportAuthorityV1;
  readonly initialTab?: MultiModResultsTabV1;
  readonly initialPageSize?: 25 | 50 | 100;
}

const RESULT_TABS: readonly {
  id: MultiModResultsTabV1;
  label: string;
  description: string;
  scientific: boolean;
}[] = [
  {
    id: "eligibility",
    label: "Eligibility",
    description: "Admission and interpretation gates",
    scientific: false,
  },
  {
    id: "diagnostics",
    label: "Diagnostics",
    description: "Method and comparability evidence",
    scientific: false,
  },
  {
    id: "estimates",
    label: "Estimates",
    description: "Completed point estimates",
    scientific: true,
  },
  {
    id: "inference",
    label: "Inference",
    description: "Intervals, probabilities, and contrasts",
    scientific: true,
  },
  {
    id: "exclusions",
    label: "Exclusions",
    description: "Rows, blockers, and scope boundaries",
    scientific: false,
  },
  {
    id: "failures",
    label: "Failures",
    description: "Complete resampling ledger evidence",
    scientific: false,
  },
  {
    id: "provenance",
    label: "Provenance",
    description: "Immutable method and digest identities",
    scientific: false,
  },
  {
    id: "sidecars",
    label: "Sidecars",
    description: "Required Arrow evidence inventory",
    scientific: false,
  },
];

const NUMBER_FORMAT = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 6,
  minimumFractionDigits: 0,
  useGrouping: true,
});

/** Frozen DOM-safety contract: no scientific result grid renders over 100 body rows. */
export const MULTIMOD_RESULT_DOM_ROW_CAP_V1 = 100 as const;
/** Accessible virtual row window retained in the DOM within a selected page. */
export const MULTIMOD_RESULT_VIRTUAL_WINDOW_ROWS_V1 = 25 as const;

function formatNumber(value: number | undefined): string {
  return value === undefined ? "Not estimated" : NUMBER_FORMAT.format(value);
}

function formatProbability(value: number | undefined): string {
  return value === undefined
    ? "Not estimated"
    : value < 0.000001
      ? "< 0.000001"
      : NUMBER_FORMAT.format(value);
}

function formatPercent(value: number): string {
  return `${NUMBER_FORMAT.format(value * 100)}%`;
}

function formatBytes(value: number): string {
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${NUMBER_FORMAT.format(value / 1024)} KiB`;
  return `${NUMBER_FORMAT.format(value / (1024 * 1024))} MiB`;
}

function humanize(value: string): string {
  return value
    .replaceAll("_", " ")
    .replace(/\b\w/gu, (character) => character.toUpperCase());
}

function yesNo(value: boolean): string {
  return value ? "Yes" : "No";
}

function intervalCells(interval: MultimodIntervalV1 | undefined): string[] {
  if (!interval)
    return [
      "Not estimated",
      "Not estimated",
      "Not estimated",
      "Not estimated",
      "Not estimated",
    ];
  return [
    formatPercent(interval.confidence_level),
    humanize(interval.family),
    humanize(interval.alternative),
    interval.lower === undefined
      ? "Not applicable"
      : formatNumber(interval.lower),
    interval.upper === undefined
      ? "Not applicable"
      : formatNumber(interval.upper),
  ];
}

const INTERVAL_COLUMNS = [
  "Confidence",
  "Interval family",
  "Alternative",
  "Lower",
  "Upper",
] as const;

function resultProvenance(
  result: MultiModAnalysisResultV1,
): MultimodProvenanceV1 {
  return result.analysis.provenance;
}

function resultLedgers(
  result: MultiModAnalysisResultV1,
): MultimodReplicateLedgerSummaryV1[] {
  if (result.kind === "pls_multigroup_analysis_v1")
    return result.analysis.replicate_ledgers;
  if (result.kind === "pls_heterogeneity_analysis_v2") {
    return result.analysis.bootstrap_ledger
      ? [result.analysis.bootstrap_ledger]
      : [];
  }
  return [result.analysis.replicate_ledger];
}

export function nativeMultiModResultReadinessV1(
  result: MultiModAnalysisResultV1,
): NativeMultiModResultReadinessV1 {
  const reasons: string[] = [];
  const provenance = resultProvenance(result);
  if (provenance.qualification === "failed_closed") {
    reasons.push("The result qualification state is failed closed.");
  }
  resultLedgers(result).forEach((ledger, index) => {
    if (!ledger.complete) {
      reasons.push(
        `Resampling ledger ${index + 1} has ${ledger.usable} usable draws; ${ledger.minimum_required} are required.`,
      );
    }
  });
  if (result.kind === "pls_multigroup_analysis_v1") {
    result.analysis.group_eligibility
      .filter((group) => !group.eligible)
      .forEach((group) => {
        reasons.push(
          `Group ${group.label} is ineligible for multigroup interpretation.`,
        );
      });
  }
  if (
    result.kind === "pls_heterogeneity_analysis_v2" &&
    result.analysis.locked_algorithm &&
    result.analysis.locked_k
  ) {
    const lockedCandidate = result.analysis.candidates.find(
      (candidate) =>
        candidate.method.kind === "segmentation" &&
        candidate.method.algorithm === result.analysis.locked_algorithm &&
        candidate.k === result.analysis.locked_k,
    );
    if (!lockedCandidate || lockedCandidate.state !== "converged_stable") {
      reasons.push(
        "The locked segmentation is missing or did not reproduce a converged stable solution.",
      );
    }
    if (!result.analysis.bootstrap_ledger) {
      reasons.push(
        "The locked segmentation has no completed fixed-algorithm, fixed-K bootstrap ledger.",
      );
    }
  }
  if (result.kind === "interventional_mediation_result_v1") {
    result.analysis.positivity
      .filter((diagnostic) => !diagnostic.supported)
      .forEach((diagnostic) => {
        reasons.push(
          `Requested ${diagnostic.variable_id} value ${formatNumber(diagnostic.requested_value)} fails ${diagnostic.support_rule} (${diagnostic.support_count} observed; ${diagnostic.minimum_required_count} required).`,
        );
      });
  }
  if (result.kind === "general_sem_conditional_process_result_v2") {
    const minimumRequired = result.analysis.replicate_ledger.minimum_required;
    result.analysis.targets
      .filter((target) => target.usable_replicates < minimumRequired)
      .forEach((target) => {
        reasons.push(
          `Target ${target.target_id} has only ${target.usable_replicates} usable draws; ${minimumRequired} are required.`,
        );
      });
  }
  return { completeScientificResult: reasons.length === 0, reasons };
}

export function nextMultiModResultsTabV1(
  current: MultiModResultsTabV1,
  key: string,
  disabled: ReadonlySet<MultiModResultsTabV1> = new Set(),
): MultiModResultsTabV1 | null {
  if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(key)) return null;
  const enabled = RESULT_TABS.map((tab) => tab.id).filter(
    (id) => !disabled.has(id),
  );
  if (!enabled.length) return null;
  if (key === "Home") return enabled[0];
  if (key === "End") return enabled.at(-1) ?? null;
  const currentIndex = Math.max(0, enabled.indexOf(current));
  const delta = key === "ArrowRight" ? 1 : -1;
  return enabled[(currentIndex + delta + enabled.length) % enabled.length];
}

export function paginateMultiModRowsV1<T>(
  rows: readonly T[],
  page: number,
  pageSize: number,
): {
  rows: readonly T[];
  page: number;
  pageCount: number;
  start: number;
  end: number;
} {
  const safePageSize = Math.min(
    MULTIMOD_RESULT_DOM_ROW_CAP_V1,
    Math.max(1, Math.trunc(pageSize)),
  );
  const pageCount = Math.max(1, Math.ceil(rows.length / safePageSize));
  const safePage = Math.min(pageCount - 1, Math.max(0, Math.trunc(page)));
  const start = safePage * safePageSize;
  const end = Math.min(rows.length, start + safePageSize);
  return {
    rows: rows.slice(start, end),
    page: safePage,
    pageCount,
    start,
    end,
  };
}

export function windowMultiModRowsV1<T>(
  rows: readonly T[],
  requestedStart: number,
  requestedSize = MULTIMOD_RESULT_VIRTUAL_WINDOW_ROWS_V1,
): {
  rows: readonly T[];
  start: number;
  end: number;
  size: number;
} {
  const size = Math.min(
    MULTIMOD_RESULT_VIRTUAL_WINDOW_ROWS_V1,
    Math.max(1, Math.trunc(requestedSize)),
  );
  const maximumStart = Math.max(0, rows.length - size);
  const start = Math.min(
    maximumStart,
    Math.max(0, Math.trunc(requestedStart)),
  );
  const end = Math.min(rows.length, start + size);
  return { rows: rows.slice(start, end), start, end, size };
}

function table(
  id: string,
  category: MultiModResultTableCategoryV1,
  title: string,
  description: string,
  columns: readonly string[],
  rows: readonly (readonly string[])[],
  rowKeys?: readonly string[],
): MultiModResultTableV1 {
  return {
    id,
    category,
    title,
    description,
    columns,
    rows,
    ...(rowKeys ? { rowKeys } : {}),
  };
}

function qualificationRows(
  attachment: MultiModResultAttachmentV1,
  readiness: NativeMultiModResultReadinessV1,
): string[][] {
  const provenance = resultProvenance(attachment.result);
  return [
    [
      "Strict result parser",
      "Passed",
      "The attachment and result family passed the exact frontend archive contract.",
    ],
    [
      "Qualification",
      humanize(provenance.qualification),
      provenance.qualification === "unqualified_labs"
        ? "Labs output only; schema validity is not release qualification."
        : provenance.qualification === "release_qualified_candidate"
          ? "Candidate state only; live exact-commit manifests remain authoritative."
          : "Scientific interpretation is blocked.",
    ],
    [
      "Scientific display",
      readiness.completeScientificResult ? "Complete" : "Withheld",
      readiness.completeScientificResult
        ? "All result-level display gates passed."
        : readiness.reasons.join(" "),
    ],
    [
      "Sidecar descriptors",
      "Passed",
      `${attachment.sidecars.length} required descriptor${attachment.sidecars.length === 1 ? "" : "s"} agree with the scientific result inventory; archive byte verification remains authoritative.`,
    ],
  ];
}

function commonTables(
  attachment: MultiModResultAttachmentV1,
  readiness: NativeMultiModResultReadinessV1,
): MultiModResultTableV1[] {
  const result = attachment.result;
  const provenance = resultProvenance(result);
  const ledgers = resultLedgers(result);
  const tables: MultiModResultTableV1[] = [
    table(
      "multimod-result-eligibility",
      "eligibility",
      "Result eligibility",
      "Strict parsing, qualification, scientific-display, and evidence-inventory decisions.",
      ["Gate", "Status", "Detail"],
      qualificationRows(attachment, readiness),
      ["parser", "qualification", "scientific-display", "sidecars"],
    ),
    table(
      "multimod-provenance",
      "provenance",
      "Immutable provenance",
      "Exact attachment, recipe, model, dataset, engine, seed, capability, and digest identities.",
      ["Field", "Value"],
      [
        ["Result ID", attachment.result_id],
        ["Attachment recipe ID", attachment.recipe_id],
        ["Result SHA-256", attachment.result_sha256],
        ["Attachment identity SHA-256", attachment.identity_sha256],
        ["Method version", provenance.method_version],
        ["Result recipe ID", provenance.recipe_id],
        ["Recipe analytical SHA-256", provenance.recipe_analytical_sha256],
        ["Configuration SHA-256", provenance.config_sha256],
        ["Model scientific SHA-256", provenance.model_scientific_sha256],
        ["Dataset fingerprint", provenance.dataset_fingerprint],
        ["Engine version", provenance.engine_version],
        ["Seed", String(provenance.seed)],
        ["Capability ID", provenance.capability_cell.capability_id],
        ["Capability cell", provenance.capability_cell.cell_id],
        ["Capability version", provenance.capability_cell.capability_version],
        ["Qualification", humanize(provenance.qualification)],
      ],
    ),
    table(
      "multimod-sidecars",
      "sidecars",
      "Required Arrow sidecars",
      "Complete descriptor inventory. Missing, altered, duplicate, schema-mismatched, or identity-mismatched entries are rejected before this view.",
      [
        "Archive entry",
        "Evidence role",
        "Schema contract",
        "Contract version",
        "Rows",
        "Columns",
        "Size",
        "Compression",
        "Arrow schema SHA-256",
        "Content SHA-256",
        "Identity SHA-256",
        "Required",
      ],
      attachment.sidecars.map((sidecar) => sidecarRow(sidecar)),
      attachment.sidecars.map((sidecar) => sidecar.entry_name),
    ),
    table(
      "multimod-scope-boundaries",
      "exclusions",
      "Scientific scope boundaries",
      "These boundaries remain visible in every exported and on-screen result.",
      ["Boundary", "Reason"],
      [
        [
          "Release qualification is not implied",
          "A validated Labs/candidate result still requires exact-commit live manifests and every dependent qualification gate.",
        ],
        [
          "Continuous moderation V1 is separate",
          "This additive result does not reinterpret or replace the protected continuous-moderation workflow.",
        ],
        [
          "Partial execution is not a result",
          "Cancellation caches and incomplete ledgers are never presented as completed scientific estimates.",
        ],
      ],
      ["qualification", "continuous-v1", "partial-execution"],
    ),
  ];
  if (ledgers.length) {
    tables.push(...ledgerTables(ledgers));
  }
  return tables;
}

function sidecarRow(sidecar: MultimodResultSidecarDescriptorV1): string[] {
  return [
    sidecar.entry_name,
    sidecar.evidence_role,
    sidecar.arrow_schema_contract_id,
    String(sidecar.arrow_schema_contract_version),
    NUMBER_FORMAT.format(sidecar.row_count),
    NUMBER_FORMAT.format(sidecar.column_count),
    formatBytes(sidecar.uncompressed_bytes),
    sidecar.compression,
    sidecar.arrow_schema_sha256,
    sidecar.sha256,
    sidecar.identity_sha256,
    yesNo(sidecar.required_for_scientific_reopen),
  ];
}

function ledgerTables(
  ledgers: readonly MultimodReplicateLedgerSummaryV1[],
): MultiModResultTableV1[] {
  const ledgerId = (ledger: MultimodReplicateLedgerSummaryV1, index: number) =>
    `Ledger ${index + 1} · ${ledger.ledger_sha256.slice(0, 12)}`;
  const summaries = ledgers.map((ledger, index) => [
    ledgerId(ledger, index),
    NUMBER_FORMAT.format(ledger.requested),
    NUMBER_FORMAT.format(ledger.usable),
    NUMBER_FORMAT.format(ledger.minimum_required),
    formatPercent(ledger.usable_fraction),
    ledger.complete ? "Complete" : "Incomplete",
    ledger.ledger_sha256,
  ]);
  const counts = ledgers.flatMap((ledger, index) =>
    Object.entries(ledger.failure_counts).map(([code, count]) => [
      ledgerId(ledger, index),
      code,
      NUMBER_FORMAT.format(count),
    ]),
  );
  const failures = ledgers.flatMap((ledger, index) =>
    ledger.failures.map((failure) => [
      ledgerId(ledger, index),
      NUMBER_FORMAT.format(failure.replicate_index),
      humanize(failure.kind),
      failure.stable_code,
      failure.detail || "No additional detail retained",
    ]),
  );
  return [
    table(
      "multimod-ledgers",
      "failures",
      "Resampling ledger summary",
      "Requested, usable, minimum, completion, and immutable shared-ledger identities. Failed draws are not replaced.",
      [
        "Ledger",
        "Requested",
        "Usable",
        "Minimum",
        "Usable fraction",
        "State",
        "Ledger SHA-256",
      ],
      summaries,
      ledgers.map((ledger, index) => `${index}:${ledger.ledger_sha256}`),
    ),
    table(
      "multimod-failure-counts",
      "failures",
      "Failure counts",
      "Complete aggregate failure inventory by stable reason code.",
      ["Ledger", "Stable code", "Count"],
      counts,
    ),
    table(
      "multimod-failure-details",
      "failures",
      "Retained replicate failures",
      "Typed per-replicate failure details retained by the result contract.",
      ["Ledger", "Replicate", "Kind", "Stable code", "Detail"],
      failures,
    ),
  ];
}

function mgaTables(
  result: Extract<
    MultiModAnalysisResultV1,
    { kind: "pls_multigroup_analysis_v1" }
  >,
  scientificReady: boolean,
): MultiModResultTableV1[] {
  const analysis = result.analysis;
  const tables: MultiModResultTableV1[] = [
    table(
      "mga-group-eligibility",
      "eligibility",
      "Group eligibility",
      "Selected rows, complete cases, warning counts, blocker counts, and exact admission state for every selected group.",
      [
        "Group",
        "Group ID",
        "Selected rows",
        "Complete cases",
        "Eligible",
        "Warnings",
        "Blockers",
      ],
      analysis.group_eligibility.map((group) => [
        group.label,
        group.group_id,
        NUMBER_FORMAT.format(group.selected_rows),
        NUMBER_FORMAT.format(group.complete_cases),
        yesNo(group.eligible),
        NUMBER_FORMAT.format(group.warnings.length),
        NUMBER_FORMAT.format(group.blockers.length),
      ]),
      analysis.group_eligibility.map((group) => group.group_id),
    ),
    table(
      "mga-micom",
      "diagnostics",
      "Pairwise MICOM",
      "Configural, compositional, mean, and variance invariance evidence. No omnibus MICOM claim is made.",
      [
        "Left group",
        "Right group",
        "Construct",
        "Interpretation",
        "Configural reviewed",
        "Observed correlation",
        "Lower quantile",
        "Compositional probability",
        "Compositional invariance",
        "Partial invariance",
        "Equal-mean probability",
        "Equal-variance probability",
      ],
      analysis.micom_pairs.map((pair) => [
        pair.left_group_id,
        pair.right_group_id,
        pair.construct_id,
        humanize(pair.interpretation),
        yesNo(pair.configural_invariance_confirmed),
        formatNumber(pair.compositional_correlation),
        formatNumber(pair.compositional_lower_quantile),
        formatProbability(pair.compositional_p_value),
        yesNo(pair.compositional_invariance),
        yesNo(pair.partial_invariance),
        formatProbability(pair.equal_mean_p_value),
        formatProbability(pair.equal_variance_p_value),
      ]),
    ),
  ];

  const messages = analysis.group_eligibility.flatMap((group) => [
    ...group.warnings.map((message) => [group.label, "Warning", message]),
    ...group.blockers.map((message) => [group.label, "Blocker", message]),
  ]);
  const blockedComparisons = analysis.pairwise.filter(
    (comparison) => comparison.interpretation_blocked,
  );
  tables.push(
    table(
      "mga-exclusions",
      "exclusions",
      "MGA exclusions and blockers",
      "Group-level warnings/blockers and pairwise targets suppressed by the partial-invariance gate.",
      ["Scope", "Kind", "Reason"],
      [
        ...messages,
        ...blockedComparisons.map((comparison) => [
          `${comparison.left_group_id} − ${comparison.right_group_id}: ${comparison.target_id}`,
          "Interpretation blocked",
          "Required partial measurement invariance was not established.",
        ]),
      ],
    ),
  );
  tables.push(
    table(
      "mga-excluded-rows",
      "exclusions",
      "Excluded source rows",
      "Stable row tokens, typed group values, and explicit exclusion reasons.",
      ["Stable row token", "Typed group value", "Reason"],
      analysis.excluded_rows.map((row) => [
        row.stable_row_token,
        row.typed_group_value,
        humanize(row.reason),
      ]),
      analysis.excluded_rows.map((row) => row.stable_row_token),
    ),
  );

  if (!scientificReady) return tables;
  tables.push(
    table(
      "mga-group-parameters",
      "estimates",
      "Group-specific parameters",
      "Completed group-specific path, loading, weight, interaction, or other selected parameter estimates.",
      [
        "Group",
        "Target",
        "Kind",
        "Estimate",
        "Standard error",
        "p value",
        ...INTERVAL_COLUMNS,
      ],
      analysis.group_parameters.map((row) => [
        row.group_id,
        row.parameter.target_id,
        humanize(row.parameter.target_kind),
        formatNumber(row.parameter.estimate),
        formatNumber(row.parameter.standard_error),
        formatProbability(row.parameter.p_value),
        ...intervalCells(row.parameter.interval),
      ]),
      analysis.group_parameters.map(
        (row) => `${row.group_id}:${row.parameter.target_id}`,
      ),
    ),
  );
  tables.push(
    table(
      "mga-omnibus",
      "inference",
      "K-group omnibus comparisons",
      "Explicit max-spread permutation or inverse-variance Wald omnibus tests before pairwise follow-up.",
      ["Procedure", "Target", "Statistic", "Degrees of freedom", "p value"],
      analysis.omnibus.map((comparison) => [
        humanize(comparison.procedure),
        comparison.target_id,
        formatNumber(comparison.statistic),
        NUMBER_FORMAT.format(comparison.degrees_of_freedom),
        formatProbability(comparison.p_value),
      ]),
    ),
  );
  tables.push(
    table(
      "mga-pairwise",
      "inference",
      "Pairwise group comparisons",
      `Signed left-minus-right comparisons with ${humanize(analysis.multiplicity)} multiplicity handling. Directional PLS-MGA probability is not relabelled as an ordinary two-sided p value.`,
      [
        "Procedure",
        "Left",
        "Right",
        "Target",
        "Left minus right",
        "Raw p value",
        "Adjusted p value",
        "Directional probability",
        ...INTERVAL_COLUMNS,
        "Measurement comparable",
        "Interpretation blocked",
      ],
      analysis.pairwise.map((comparison) => [
        humanize(comparison.procedure),
        comparison.left_group_id,
        comparison.right_group_id,
        comparison.target_id,
        formatNumber(comparison.difference_left_minus_right),
        formatProbability(comparison.raw_p_value),
        formatProbability(comparison.adjusted_p_value),
        formatProbability(comparison.directional_probability),
        ...intervalCells(comparison.interval),
        yesNo(comparison.measurement_comparability_satisfied),
        yesNo(comparison.interpretation_blocked),
      ]),
    ),
  );
  return tables;
}

function heterogeneityTables(
  result: Extract<
    MultiModAnalysisResultV1,
    { kind: "pls_heterogeneity_analysis_v2" }
  >,
  scientificReady: boolean,
): MultiModResultTableV1[] {
  const analysis = result.analysis;
  const methodLabel = (
    candidate: (typeof analysis.candidates)[number],
  ): string =>
    candidate.method.kind === "pooled_baseline_v1"
      ? "Pooled baseline"
      : humanize(candidate.method.algorithm);
  const candidateRows = analysis.candidates.map((candidate) => [
    methodLabel(candidate),
    `K = ${candidate.k}`,
    humanize(candidate.state),
    NUMBER_FORMAT.format(candidate.converged_starts),
    NUMBER_FORMAT.format(candidate.stable_starts),
    formatNumber(candidate.log_likelihood),
    formatNumber(candidate.objective),
    NUMBER_FORMAT.format(candidate.blockers.length),
  ]);
  const criterionRows = analysis.candidates.flatMap((candidate) =>
    Object.entries(candidate.criteria).map(([criterion, value]) => [
      methodLabel(candidate),
      `K = ${candidate.k}`,
      humanize(criterion),
      formatNumber(value),
    ]),
  );
  const shareRows = analysis.candidates.flatMap((candidate) =>
    candidate.class_or_segment_shares.map((share, index) => [
      methodLabel(candidate),
      `K = ${candidate.k}`,
      String(index + 1),
      formatPercent(share),
    ]),
  );
  const blockerRows = analysis.candidates.flatMap((candidate) =>
    candidate.blockers.map((blocker) => [
      `${methodLabel(candidate)}, K = ${candidate.k}`,
      humanize(candidate.state),
      blocker,
    ]),
  );
  const blockedContrasts = analysis.contrasts.filter(
    (contrast) => contrast.inferential_interpretation_blocked,
  );
  const tables: MultiModResultTableV1[] = [
    table(
      "heterogeneity-lock",
      "eligibility",
      "Segmentation lock and comparability state",
      "Candidate inspection never silently selects K. Inference requires an explicit algorithm/K lock.",
      ["Field", "Value", "Interpretation"],
      [
        [
          "Profile",
          humanize(analysis.profile),
          "Exact admitted interaction profile",
        ],
        [
          "Locked algorithm",
          analysis.locked_algorithm
            ? humanize(analysis.locked_algorithm)
            : "Not locked",
          analysis.locked_algorithm
            ? "Analyst-selected for fixed-K inference"
            : "Discovery diagnostics only",
        ],
        [
          "Locked K",
          analysis.locked_k === undefined
            ? "Not locked"
            : String(analysis.locked_k),
          analysis.locked_k === undefined
            ? "No inferential segmentation selected"
            : "Fixed during bootstrap",
        ],
        [
          "Discovery identity",
          analysis.discovery_result_identity_sha256,
          "Binds the pooled baseline and complete candidate inventory",
        ],
        [
          "Inference lock receipt",
          analysis.inference_lock ? "Validated and persisted" : "Not present",
          analysis.inference_lock
            ? "Selected method/K and analyst confirmation are immutable"
            : "Discovery diagnostics only",
        ],
        [
          "Contrast mode",
          analysis.descriptive_only
            ? "Descriptive only"
            : "Common-metric gate retained",
          analysis.descriptive_only
            ? "Segment-to-segment inference is suppressed"
            : "Each contrast still carries its own gate",
        ],
      ],
    ),
    table(
      "heterogeneity-candidates",
      "diagnostics",
      "Candidate segmentation diagnostics",
      "Algorithm/K convergence and multi-start stability. Criteria are evidence for inspection, never automatic selection.",
      [
        "Algorithm",
        "K",
        "State",
        "Converged starts",
        "Stable starts",
        "Log likelihood",
        "Objective",
        "Blockers",
      ],
      candidateRows,
    ),
    table(
      "heterogeneity-pooled-baseline",
      "diagnostics",
      "Pooled K = 1 baseline",
      "Read-only pooled structural reference. It is not a selectable latent segment solution.",
      ["Target", "Target kind", "Estimate"],
      analysis.candidates.flatMap((candidate) =>
        candidate.method.kind === "pooled_baseline_v1"
          ? candidate.pooled_parameters.map((parameter) => [
              parameter.target_id,
              parameter.target_kind,
              formatNumber(parameter.estimate),
            ])
          : [],
      ),
    ),
    table(
      "heterogeneity-criteria",
      "diagnostics",
      "Candidate criteria",
      "Long-form actual optimized-likelihood or objective criteria retained by algorithm and K.",
      ["Algorithm", "K", "Criterion", "Value"],
      criterionRows,
    ),
    table(
      "heterogeneity-shares",
      "diagnostics",
      "Class or segment shares",
      "Aligned class or segment proportions for each candidate partition.",
      ["Algorithm", "K", "Aligned class or segment", "Share"],
      shareRows,
    ),
    table(
      "heterogeneity-blockers",
      "exclusions",
      "Candidate and comparability blockers",
      "Candidate failures plus segment contrasts withheld by the pooled common-metric gate.",
      ["Scope", "State", "Reason"],
      [
        ...blockerRows,
        ...blockedContrasts.map((contrast) => [
          `Class ${contrast.left_class_id} − ${contrast.right_class_id}: ${contrast.target_id}`,
          "Inference blocked",
          "Pooled common-metric configural/compositional comparability was not established.",
        ]),
      ],
    ),
  ];
  if (!scientificReady) return tables;
  tables.push(
    table(
      "heterogeneity-parameters",
      "estimates",
      "Class or segment parameters",
      "Completed class-specific paths, interaction coefficients, slopes, residual variances, and related parameters on the declared metric.",
      [
        "Class or segment",
        "Metric",
        "Target",
        "Kind",
        "Estimate",
        "Standard error",
        "p value",
        ...INTERVAL_COLUMNS,
      ],
      analysis.parameters.map((row) => [
        String(row.class_id),
        row.metric,
        row.parameter.target_id,
        humanize(row.parameter.target_kind),
        formatNumber(row.parameter.estimate),
        formatNumber(row.parameter.standard_error),
        formatProbability(row.parameter.p_value),
        ...intervalCells(row.parameter.interval),
      ]),
      analysis.parameters.map(
        (row) => `${row.class_id}:${row.parameter.target_id}`,
      ),
    ),
  );
  tables.push(
    table(
      "heterogeneity-contrasts",
      "inference",
      "Common-metric class or segment contrasts",
      "Only comparability-admitted contrasts expose effect inference. Blocked contrasts remain listed under Exclusions without their scientific difference.",
      [
        "Left class",
        "Right class",
        "Target",
        "Difference",
        "p value",
        ...INTERVAL_COLUMNS,
        "Comparable",
        "Inference blocked",
      ],
      analysis.contrasts
        .filter((contrast) => !contrast.inferential_interpretation_blocked)
        .map((contrast) => [
          String(contrast.left_class_id),
          String(contrast.right_class_id),
          contrast.target_id,
          formatNumber(contrast.difference),
          formatProbability(contrast.p_value),
          ...intervalCells(contrast.interval),
          yesNo(contrast.common_metric_comparability_satisfied),
          yesNo(contrast.inferential_interpretation_blocked),
        ]),
    ),
  );
  return tables;
}

function conditionalTables(
  result: Extract<
    MultiModAnalysisResultV1,
    { kind: "general_sem_conditional_process_result_v2" }
  >,
  scientificReady: boolean,
): MultiModResultTableV1[] {
  const analysis = result.analysis;
  const probeRows = analysis.targets.flatMap((target) =>
    Object.entries(target.probe_values).map(([moderator, value]) => [
      target.target_id,
      moderator,
      formatNumber(value),
    ]),
  );
  const derivativeRows = analysis.targets.flatMap((target) =>
    target.derivative_variables.map((variable, index) => [
      target.target_id,
      String(index + 1),
      variable,
    ]),
  );
  const tables: MultiModResultTableV1[] = [
    table(
      "conditional-profile",
      "eligibility",
      "Conditional-process profile",
      "The exact profile and shared-ledger state govern which estimands and alternatives may be interpreted.",
      ["Field", "Value", "Interpretation"],
      [
        [
          "Profile",
          humanize(analysis.profile_id),
          "Exact bounded structural/inference envelope",
        ],
        [
          "Explicit targets",
          NUMBER_FORMAT.format(analysis.targets.length),
          "QuickPLS never guesses an indirect path",
        ],
        [
          "Shared ledger",
          analysis.replicate_ledger.complete ? "Complete" : "Incomplete",
          `${analysis.replicate_ledger.usable} usable of ${analysis.replicate_ledger.requested}`,
        ],
      ],
    ),
    table(
      "conditional-probes",
      "diagnostics",
      "Frozen probe values",
      "Original-sample probe anchors reused unchanged across all resamples.",
      ["Target", "Moderator", "Probe value"],
      probeRows,
    ),
    table(
      "conditional-derivatives",
      "diagnostics",
      "Derivative identities",
      "Ordered moderator identities for local first, second, and cross derivatives.",
      ["Target", "Derivative position", "Moderator"],
      derivativeRows,
    ),
    table(
      "conditional-warnings",
      "exclusions",
      "Conditional-process warnings",
      "Complete result-level warning inventory without silently simplifying the requested model.",
      ["Warning", "Scope"],
      analysis.warnings.map((warning) => [
        warning,
        "Exact authored profile and target inventory",
      ]),
    ),
  ];
  if (!scientificReady) return tables;
  tables.push(
    table(
      "conditional-estimates",
      "estimates",
      "Conditional-process estimates",
      "Explicit conditional indirect, total, derivative, scalar-index, probe, and group-contrast point estimates.",
      [
        "Target",
        "Kind",
        "Path",
        "Group",
        "Probe tuple",
        "Derivative variables",
        "Estimate",
      ],
      analysis.targets.map((target) => [
        target.target_id,
        humanize(target.kind),
        target.path_id,
        target.group_id ?? "Not applicable",
        Object.entries(target.probe_values)
          .map(([id, value]) => `${id} = ${formatNumber(value)}`)
          .join("; ") || "Not applicable",
        target.derivative_variables.join(" → ") || "Not applicable",
        formatNumber(target.estimate),
      ]),
      analysis.targets.map((target) => target.target_id),
    ),
  );
  tables.push(
    table(
      "conditional-inference",
      "inference",
      "Conditional-process inference",
      "Every target uses the same bootstrap replicate ledger. One-sided intervals retain only their mathematically defined bound.",
      [
        "Target",
        "Kind",
        "Path",
        "Estimate",
        "p value",
        ...INTERVAL_COLUMNS,
        "Usable replicates",
      ],
      analysis.targets.map((target) => [
        target.target_id,
        humanize(target.kind),
        target.path_id,
        formatNumber(target.estimate),
        formatProbability(target.p_value),
        ...intervalCells(target.interval),
        NUMBER_FORMAT.format(target.usable_replicates),
      ]),
      analysis.targets.map((target) => target.target_id),
    ),
  );
  return tables;
}

function causalTables(
  result: Extract<
    MultiModAnalysisResultV1,
    { kind: "interventional_mediation_result_v1" }
  >,
  scientificReady: boolean,
): MultiModResultTableV1[] {
  const analysis = result.analysis;
  const unsupported = analysis.positivity.filter(
    (diagnostic) => !diagnostic.supported,
  );
  const tables: MultiModResultTableV1[] = [
    table(
      "causal-identification",
      "eligibility",
      "Identification assumptions",
      "Analyst-declared assumptions required for the observed-data parametric g-computation interpretation.",
      ["Assumption", "Interpretation label"],
      analysis.identification_assumptions.map((assumption) => [
        assumption,
        analysis.interpretation_label,
      ]),
    ),
    table(
      "causal-positivity",
      "diagnostics",
      "Positivity diagnostics",
      "Every requested treatment or moderator value is checked against observed support.",
      [
        "Variable",
        "Observed minimum",
        "Observed maximum",
        "Requested value",
        "Support count",
        "Required",
        "Rule",
        "Supported",
      ],
      analysis.positivity.map((diagnostic) => [
        diagnostic.variable_id,
        formatNumber(diagnostic.observed_minimum),
        formatNumber(diagnostic.observed_maximum),
        formatNumber(diagnostic.requested_value),
        formatNumber(diagnostic.support_count),
        formatNumber(diagnostic.minimum_required_count),
        diagnostic.support_rule,
        yesNo(diagnostic.supported),
      ]),
    ),
    table(
      "causal-exclusions",
      "exclusions",
      "Causal interpretation boundaries",
      "The first profile excludes natural/cross-world effects and fails closed outside observed support.",
      ["Scope", "State", "Reason"],
      [
        [
          "All effects",
          "Assumption-dependent",
          "The result never says causality is established.",
        ],
        [
          "Natural or cross-world effects",
          "Unsupported",
          "Only interventional effects are admitted.",
        ],
        [
          "Recanting witness or exposure-induced mediator–outcome confounding",
          "Unsupported",
          "These settings are outside the first qualified profile.",
        ],
        ...unsupported.map((diagnostic) => [
          diagnostic.variable_id,
          "Outside observed support",
          `Requested ${formatNumber(diagnostic.requested_value)}; observed range ${formatNumber(diagnostic.observed_minimum)} to ${formatNumber(diagnostic.observed_maximum)}.`,
        ]),
      ],
    ),
  ];
  if (!scientificReady) return tables;
  tables.push(
    table(
      "causal-estimates",
      "estimates",
      "Interventional mediation estimates",
      "Observed-data parametric g-computation point estimates with the mandatory cautious interpretation label.",
      ["Target", "Path", "Estimand", "Estimate", "Interpretation"],
      analysis.effects.map((effect) => [
        effect.target_id,
        effect.path_id,
        humanize(effect.estimand),
        formatNumber(effect.estimate),
        analysis.interpretation_label,
      ]),
      analysis.effects.map((effect) => effect.target_id),
    ),
  );
  tables.push(
    table(
      "causal-inference",
      "inference",
      "Interventional mediation inference",
      "Bootstrap inference remains assumption-dependent and does not establish causality.",
      [
        "Target",
        "Path",
        "Estimand",
        "Estimate",
        "p value",
        ...INTERVAL_COLUMNS,
      ],
      analysis.effects.map((effect) => [
        effect.target_id,
        effect.path_id,
        humanize(effect.estimand),
        formatNumber(effect.estimate),
        formatProbability(effect.p_value),
        ...intervalCells(effect.interval),
      ]),
      analysis.effects.map((effect) => effect.target_id),
    ),
  );
  return tables;
}

function buildView(
  attachment: MultiModResultAttachmentV1,
  readiness: NativeMultiModResultReadinessV1,
): MultiModResultsViewV1 {
  const result = attachment.result;
  const common = commonTables(attachment, readiness);
  if (result.kind === "pls_multigroup_analysis_v1") {
    return {
      familyLabel: "PLS multigroup analysis",
      profileLabel: `${humanize(result.analysis.profile)} · ${humanize(result.analysis.multiplicity)}`,
      provenance: result.analysis.provenance,
      tables: [
        ...common,
        ...mgaTables(result, readiness.completeScientificResult),
      ],
    };
  }
  if (result.kind === "pls_heterogeneity_analysis_v2") {
    const lock =
      result.analysis.locked_algorithm && result.analysis.locked_k
        ? `${humanize(result.analysis.locked_algorithm)}, K = ${result.analysis.locked_k}`
        : "Discovery diagnostics · no algorithm/K lock";
    return {
      familyLabel: "PLS unobserved heterogeneity",
      profileLabel: `${humanize(result.analysis.profile)} · ${lock}`,
      provenance: result.analysis.provenance,
      tables: [
        ...common,
        ...heterogeneityTables(result, readiness.completeScientificResult),
      ],
    };
  }
  if (result.kind === "general_sem_conditional_process_result_v2") {
    return {
      familyLabel: "General SEM conditional process",
      profileLabel: humanize(result.analysis.profile_id),
      provenance: result.analysis.provenance,
      tables: [
        ...common,
        ...conditionalTables(result, readiness.completeScientificResult),
      ],
    };
  }
  return {
    familyLabel: "Interventional causal mediation",
    profileLabel: result.analysis.interpretation_label,
    provenance: result.analysis.provenance,
    tables: [
      ...common,
      ...causalTables(result, readiness.completeScientificResult),
    ],
  };
}

function PaginatedMultiModResultTable({
  table: source,
  resultId,
  initialPageSize,
}: {
  readonly table: MultiModResultTableV1;
  readonly resultId: string;
  readonly initialPageSize: 25 | 50 | 100;
}) {
  const instanceId = useId().replaceAll(":", "");
  const [pageSize, setPageSize] = useState<number>(initialPageSize);
  const [requestedPage, setRequestedPage] = useState(0);
  const [requestedWindowStart, setRequestedWindowStart] = useState(0);
  const pagination = paginateMultiModRowsV1(
    source.rows,
    requestedPage,
    pageSize,
  );
  const headingId = `nd-multimod-results-${instanceId}-heading`;
  const descriptionId = `nd-multimod-results-${instanceId}-description`;
  const pageStatusId = `nd-multimod-results-${instanceId}-page-status`;
  const window = windowMultiModRowsV1(
    pagination.rows,
    requestedWindowStart,
  );
  const absoluteWindowStart = pagination.start + window.start;
  const pageRows = window.rows.map((row) => [...row]);
  const rowHints = window.rows.map((_row, windowIndex) => ({
    key:
      source.rowKeys?.[absoluteWindowStart + windowIndex] ??
      `${source.id}:${absoluteWindowStart + windowIndex}`,
  }));
  const nativeTable: ResultTable = {
    id: source.id,
    title: source.title,
    status: "experimental",
    warning:
      "MultiMod Labs result. Qualification state and scientific boundaries remain visible in the surrounding result view.",
    columns: [...source.columns],
    rows: pageRows,
    presentation: { rows: rowHints },
  };
  const hasRows = source.rows.length > 0;
  const isPaginated = source.rows.length > pageSize;
  const isWindowed = pagination.rows.length > window.size;

  useEffect(() => {
    if (requestedPage !== pagination.page) setRequestedPage(pagination.page);
  }, [pagination.page, requestedPage]);

  useEffect(() => {
    setRequestedWindowStart(0);
  }, [pageSize, pagination.page]);

  return (
    <section
      className="nd-multimod-result-card"
      data-dom-row-cap={MULTIMOD_RESULT_DOM_ROW_CAP_V1}
      aria-labelledby={headingId}
      aria-describedby={descriptionId}
    >
      <header>
        <div>
          <h4 id={headingId}>{source.title}</h4>
          <p id={descriptionId}>{source.description}</p>
        </div>
        <span className="nd-multimod-result-row-count">
          {NUMBER_FORMAT.format(source.rows.length)} row
          {source.rows.length === 1 ? "" : "s"}
        </span>
      </header>
      {!hasRows ? (
        <p className="nd-multimod-result-empty" role="status">
          No rows were retained for this table.
        </p>
      ) : (
        <>
          <div className="nd-multimod-result-pagebar">
            <p
              id={pageStatusId}
              role="status"
              aria-live="polite"
              aria-atomic="true"
            >
              Showing {NUMBER_FORMAT.format(pagination.start + 1)}–
              {NUMBER_FORMAT.format(pagination.end)} of{" "}
              {NUMBER_FORMAT.format(source.rows.length)} rows. Page{" "}
              {pagination.page + 1} of {pagination.pageCount}.
              {source.rows.length > MULTIMOD_RESULT_DOM_ROW_CAP_V1
                ? ` Pages are bounded at ${MULTIMOD_RESULT_DOM_ROW_CAP_V1} rows.`
                : ""}
              {pagination.rows.length > MULTIMOD_RESULT_VIRTUAL_WINDOW_ROWS_V1
                ? ` The virtual keyboard grid currently renders absolute rows ${NUMBER_FORMAT.format(absoluteWindowStart + 1)}–${NUMBER_FORMAT.format(pagination.start + window.end)}.`
                : ""}
            </p>
            {source.rows.length > 25 ? (
              <label htmlFor={`nd-multimod-results-${instanceId}-page-size`}>
                Rows per page
                <select
                  id={`nd-multimod-results-${instanceId}-page-size`}
                  value={pageSize}
                  onChange={(event) => {
                    setPageSize(Number(event.target.value));
                    setRequestedPage(0);
                    setRequestedWindowStart(0);
                  }}
                >
                  <option value={25}>25</option>
                  <option value={50}>50</option>
                  <option value={100}>100</option>
                </select>
              </label>
            ) : null}
          </div>
          <NativeResultTable
            table={nativeTable}
            gridKey={`${resultId}:${source.id}:${pagination.page}:${pageSize}:${window.start}`}
            headingId={headingId}
            totalRowCount={source.rows.length}
            rowIndexOffset={absoluteWindowStart}
          />
          {isWindowed ? (
            <nav
              className="nd-multimod-result-pagination"
              aria-label={`${source.title} virtual row windows`}
              aria-describedby={pageStatusId}
            >
              <button
                type="button"
                disabled={window.start === 0}
                onClick={() =>
                  setRequestedWindowStart((current) =>
                    Math.max(0, current - window.size),
                  )
                }
                aria-label={`Previous row window of ${source.title}`}
              >
                Previous rows
              </button>
              <span aria-hidden="true">
                {NUMBER_FORMAT.format(absoluteWindowStart + 1)}–
                {NUMBER_FORMAT.format(pagination.start + window.end)}
              </span>
              <button
                type="button"
                disabled={window.end >= pagination.rows.length}
                onClick={() =>
                  setRequestedWindowStart((current) =>
                    Math.min(
                      Math.max(0, pagination.rows.length - window.size),
                      current + window.size,
                    ),
                  )
                }
                aria-label={`Next row window of ${source.title}`}
              >
                Next rows
              </button>
            </nav>
          ) : null}
          {isPaginated ? (
            <nav
              className="nd-multimod-result-pagination"
              aria-label={`${source.title} pages`}
              aria-describedby={pageStatusId}
            >
              <button
                type="button"
                disabled={pagination.page === 0}
                onClick={() => setRequestedPage(0)}
                aria-label={`First page of ${source.title}`}
              >
                First
              </button>
              <button
                type="button"
                disabled={pagination.page === 0}
                onClick={() =>
                  setRequestedPage((current) => Math.max(0, current - 1))
                }
                aria-label={`Previous page of ${source.title}`}
              >
                Previous
              </button>
              <span aria-hidden="true">
                {pagination.page + 1} / {pagination.pageCount}
              </span>
              <button
                type="button"
                disabled={pagination.page + 1 >= pagination.pageCount}
                onClick={() =>
                  setRequestedPage((current) =>
                    Math.min(pagination.pageCount - 1, current + 1),
                  )
                }
                aria-label={`Next page of ${source.title}`}
              >
                Next
              </button>
              <button
                type="button"
                disabled={pagination.page + 1 >= pagination.pageCount}
                onClick={() => setRequestedPage(pagination.pageCount - 1)}
                aria-label={`Last page of ${source.title}`}
              >
                Last
              </button>
            </nav>
          ) : null}
        </>
      )}
    </section>
  );
}

function InvalidMultiModResult({ error }: { readonly error: unknown }) {
  const message =
    error instanceof MultiModContractErrorV1
      ? `${error.message} (${error.code} at ${error.path})`
      : error instanceof Error
        ? error.message
        : "The result attachment failed local validation.";
  return (
    <section
      className="nd-multimod-results nd-multimod-results-invalid"
      data-multimod-results="invalid"
      aria-labelledby="nd-multimod-results-invalid-heading"
    >
      <header>
        <ShieldAlert size={22} aria-hidden="true" />
        <div>
          <h2 id="nd-multimod-results-invalid-heading">
            MultiMod result withheld
          </h2>
          <p>No scientific values were rendered.</p>
        </div>
      </header>
      <div className="nd-form-error" role="alert">
        <strong>Strict result validation failed.</strong>
        <span>{message}</span>
      </div>
    </section>
  );
}

export function NativeMultiModResultsV1({
  validatedResult,
  canonicalDocument,
  rawSidecarExportAuthority,
  initialTab = "eligibility",
  initialPageSize = 50,
}: NativeMultiModResultsV1Props) {
  const validation = useMemo(() => {
    try {
      return {
        result: parseMultiModResultAttachmentV1(
          validatedResult,
          "validated_result",
        ),
        error: null,
      };
    } catch (error) {
      return { result: null, error };
    }
  }, [validatedResult]);
  if (!validation.result)
    return <InvalidMultiModResult error={validation.error} />;
  return (
    <ValidatedNativeMultiModResultsV1
      result={validation.result}
      canonicalDocument={canonicalDocument}
      rawSidecarExportAuthority={rawSidecarExportAuthority}
      initialTab={initialTab}
      initialPageSize={initialPageSize}
    />
  );
}

function ValidatedNativeMultiModResultsV1({
  result,
  canonicalDocument,
  rawSidecarExportAuthority,
  initialTab,
  initialPageSize,
}: {
  readonly result: MultiModResultAttachmentV1;
  readonly canonicalDocument?: CanonicalResultDocumentV2;
  readonly rawSidecarExportAuthority?: NativeMultiModRawSidecarExportAuthorityV1;
  readonly initialTab: MultiModResultsTabV1;
  readonly initialPageSize: 25 | 50 | 100;
}) {
  const readiness = useMemo(
    () => nativeMultiModResultReadinessV1(result.result),
    [result.result],
  );
  const view = useMemo(() => buildView(result, readiness), [readiness, result]);
  const disabledTabs = useMemo(
    () =>
      new Set<MultiModResultsTabV1>(
        readiness.completeScientificResult ? [] : ["estimates", "inference"],
      ),
    [readiness.completeScientificResult],
  );
  const safeInitialTab = disabledTabs.has(initialTab)
    ? "eligibility"
    : initialTab;
  const [activeTab, setActiveTab] =
    useState<MultiModResultsTabV1>(safeInitialTab);
  const activeTables = view.tables.filter(
    (entry) => entry.category === activeTab,
  );

  useEffect(() => {
    if (disabledTabs.has(activeTab)) setActiveTab("eligibility");
  }, [activeTab, disabledTabs]);

  const activateWithKeyboard = (event: KeyboardEvent<HTMLDivElement>) => {
    const next = nextMultiModResultsTabV1(activeTab, event.key, disabledTabs);
    if (!next) return;
    event.preventDefault();
    setActiveTab(next);
    document.getElementById(`nd-multimod-results-tab-${next}`)?.focus();
  };
  const qualificationLabel = humanize(view.provenance.qualification);
  const headingId = `nd-multimod-results-heading-${result.result_id.replace(/[^a-zA-Z0-9_-]/gu, "-")}`;

  return (
    <>
    <section
      className="nd-multimod-results"
      data-multimod-results="v1"
      data-scientific-result-ready={readiness.completeScientificResult}
      aria-labelledby={headingId}
    >
      <header className="nd-multimod-results-header">
        <div>
          <span
            className={`nd-multimod-result-status ${readiness.completeScientificResult ? "complete" : "withheld"}`}
          >
            {readiness.completeScientificResult ? (
              <CheckCircle2 size={14} aria-hidden="true" />
            ) : (
              <AlertTriangle size={14} aria-hidden="true" />
            )}
            {readiness.completeScientificResult
              ? "Complete result payload"
              : "Scientific estimates withheld"}
          </span>
          <h2 id={headingId}>{view.familyLabel}</h2>
          <p>
            {view.profileLabel} · {qualificationLabel} · Result{" "}
            <code>{result.result_id}</code>
          </p>
        </div>
        <Database size={24} aria-hidden="true" />
      </header>
      <div
        className={`nd-multimod-result-callout ${readiness.completeScientificResult ? "complete" : "withheld"}`}
        role={readiness.completeScientificResult ? "note" : "alert"}
      >
        <strong>
          {readiness.completeScientificResult
            ? "Result-level gates passed."
            : "Partial scientific output is not displayed."}
        </strong>
        <span>
          {readiness.completeScientificResult
            ? "Estimates and inference below come only from the strict completed attachment. Labs qualification and scope warnings still apply."
            : readiness.reasons.join(" ")}
        </span>
      </div>
      <div
        className="nd-multimod-results-tabs"
        role="tablist"
        aria-label="MultiMod result sections"
        onKeyDown={activateWithKeyboard}
      >
        {RESULT_TABS.map((tab) => {
          const disabled = disabledTabs.has(tab.id);
          const rowCount = view.tables
            .filter((entry) => entry.category === tab.id)
            .reduce((total, entry) => total + entry.rows.length, 0);
          return (
            <button
              key={tab.id}
              id={`nd-multimod-results-tab-${tab.id}`}
              type="button"
              role="tab"
              aria-selected={activeTab === tab.id}
              aria-controls={`nd-multimod-results-panel-${tab.id}`}
              aria-disabled={disabled}
              disabled={disabled}
              tabIndex={activeTab === tab.id ? 0 : -1}
              onClick={() => setActiveTab(tab.id)}
            >
              <strong>
                {tab.label}
                <span>{NUMBER_FORMAT.format(rowCount)}</span>
              </strong>
              <small>
                {disabled ? "Withheld: incomplete result" : tab.description}
              </small>
            </button>
          );
        })}
      </div>
      {RESULT_TABS.map((tab) => {
        const selected = activeTab === tab.id;
        return (
          <section
            key={tab.id}
            id={`nd-multimod-results-panel-${tab.id}`}
            className="nd-multimod-results-panel"
            role="tabpanel"
            tabIndex={selected ? 0 : -1}
            aria-labelledby={`nd-multimod-results-tab-${tab.id}`}
            hidden={!selected}
          >
            {selected ? (
              <>
                <h3>{tab.label}</h3>
                {activeTables.length ? (
                  activeTables.map((entry) => (
                    <PaginatedMultiModResultTable
                      key={entry.id}
                      table={entry}
                      resultId={result.result_id}
                      initialPageSize={initialPageSize}
                    />
                  ))
                ) : (
                  <p className="nd-multimod-result-empty" role="status">
                    No rows were retained for this result section.
                  </p>
                )}
                {activeTab === "sidecars" ? (
                  <NativeMultiModRawSidecarExportPanelV1
                    result={result}
                    authority={rawSidecarExportAuthority}
                  />
                ) : null}
              </>
            ) : null}
          </section>
        );
      })}
    </section>
    {readiness.completeScientificResult && canonicalDocument ? (
      <CanonicalResultExportPanelV2
        document={canonicalDocument}
        nativeDesktop
        publicationBlockedReason={view.provenance.qualification === "release_qualified_candidate"
          ? undefined
          : "This exact MultiMod profile is not release-qualified. CSV, XLSX, JSON, HTML, PDF, SVG, and PNG publication remain disabled until live qualification evidence is bound to the final build."}
      />
    ) : readiness.completeScientificResult ? (
      <p className="nd-inline-warning" role="note">
        Canonical export is withheld because no strict post-reopen projection is attached.
      </p>
    ) : null}
    </>
  );
}

function rawExportErrorMessage(reason: unknown): string {
  if (reason instanceof Error) return reason.message;
  if (reason && typeof reason === "object") {
    const value = reason as { code?: unknown; message?: unknown; correctiveAction?: unknown };
    const code = typeof value.code === "string" ? ` [${value.code}]` : "";
    const message = typeof value.message === "string"
      ? value.message
      : "The raw Arrow evidence export was rejected.";
    const corrective = typeof value.correctiveAction === "string"
      ? ` ${value.correctiveAction}`
      : "";
    return `${message}${code}${corrective}`;
  }
  return String(reason);
}

function NativeMultiModRawSidecarExportPanelV1({
  result,
  authority,
}: {
  readonly result: MultiModResultAttachmentV1;
  readonly authority?: NativeMultiModRawSidecarExportAuthorityV1;
}) {
  const eligible = result.sidecars.filter((descriptor) =>
    isNativeMultiModRawSidecarExportableV1(result.result_id, descriptor),
  );
  const [busyEntry, setBusyEntry] = useState<string | null>(null);
  const [feedback, setFeedback] = useState<string | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  if (!eligible.length) {
    return (
      <p role="note">
        This completed result has no posterior, membership, assignment,
        replicate-ledger, or target-vector Arrow payload eligible for optional
        raw export.
      </p>
    );
  }
  if (!authority) {
    return (
      <p
        className="nd-inline-warning"
        role="note"
        data-multimod-raw-export-blocker="multimod.export.raw_sidecar_authority_unavailable"
      >
        Raw Arrow evidence export is blocked because strict Archive V6 reopen
        authority is unavailable. No payload bytes are exposed from the result
        document alone.
      </p>
    );
  }
  return (
    <section
      className="nd-multimod-raw-sidecar-export"
      aria-labelledby="nd-multimod-raw-sidecar-export-heading"
      aria-busy={busyEntry !== null}
    >
      <h4 id="nd-multimod-raw-sidecar-export-heading">
        Optional validated Arrow evidence
      </h4>
      <p>
        Each export strictly reopens the unchanged archive, revalidates the
        descriptor, schema, identity, and payload digest, then publishes exact
        bytes without replacing an existing file.
      </p>
      <div className="nd-cbsem-v4-actions">
        {eligible.map((descriptor) => {
          const label = descriptor.entry_name.slice(
            descriptor.entry_name.lastIndexOf("/") + 1,
          );
          return (
            <button
              key={descriptor.entry_name}
              type="button"
              disabled={busyEntry !== null}
              title={descriptor.entry_name}
              onClick={() => {
                setFailure(null);
                setFeedback(null);
                setBusyEntry(descriptor.entry_name);
                void publishNativeMultiModRawSidecarV1(
                  authority,
                  result,
                  descriptor,
                )
                  .then((receipt) => {
                    setFeedback(
                      receipt
                        ? `Saved ${receipt.path}. Strict Arrow evidence readback and SHA-256 binding passed.`
                        : "Raw evidence export cancelled; no file was published.",
                    );
                  })
                  .catch((reason) => setFailure(rawExportErrorMessage(reason)))
                  .finally(() => setBusyEntry(null));
              }}
            >
              <Download size={15} aria-hidden="true" />
              {busyEntry === descriptor.entry_name
                ? `Exporting ${label}…`
                : `Export ${label}`}
            </button>
          );
        })}
      </div>
      {feedback ? <p role="status" aria-live="polite">{feedback}</p> : null}
      {failure ? <p className="nd-form-error" role="alert">{failure}</p> : null}
    </section>
  );
}

export default NativeMultiModResultsV1;
