import {
  AlertTriangle,
  Archive,
  FlaskConical,
  Play,
  ShieldCheck,
} from "lucide-react";
import { useEffect, useMemo, useState, type KeyboardEvent } from "react";
import {
  MultiModContractErrorV1,
  micomConfiguralChecklistCompleteV1,
  multiModSidecarCostStateV1,
  parseGeneralSemConditionalProcessConfigV2,
  parseInterventionalCausalMediationConfigV1,
  parseMgaMultigroupV1,
  parsePlsUnobservedHeterogeneityConfigV2,
  predictMultiModSidecarBytesV1,
  type CausalIdentificationChecklistV1,
  type CausalLinearTermV1,
  type ConditionalProcessEstimandsV2,
  type ConditionalProcessProfileV2,
  type ConditionalRawProbeFitMetricReceiptV2,
  type GeneralSemConditionalProcessConfigV2,
  type HeterogeneityAlgorithmV2,
  type HeterogeneityInferenceLockReceiptV2,
  type HeterogeneityInteractionProfileV2,
  type InterventionalCausalMediationConfigV1,
  type MgaModelProfileV1,
  type MgaMultigroupV1,
  type MgaProcedureV1,
  type MicomConfiguralChecklistV1,
  type MultiModResultAttachmentV1,
  type MultiModRecipeConfigV1,
  type ObservedCausalPathV1,
  type PlsUnobservedHeterogeneityConfigV2,
  type SelectedGroupV1,
  type TypedGroupValueV1,
} from "../domain/multimodContractsV1";
import type { SemModelV4 } from "../domain/semModelV4";
import type { CanonicalResultDocumentV2 } from "../domain/canonicalResultDocumentV2";
import type { NativeMultiModRawSidecarExportAuthorityV1 } from "./nativeMultiModRawSidecarExportV1";
import type { NativeMgaGroupEligibilityV1 } from "./nativeMultiModJobV1";
import { NativeMultiModResultsV1 } from "./NativeMultiModResultsV1";

export interface NativeMultiModGroupOptionV1 {
  readonly groupId: string;
  readonly label: string;
  readonly value: TypedGroupValueV1;
  readonly selectedRows?: number;
  readonly completeCases?: number;
}

export interface NativeMultiModGroupingColumnV1 {
  readonly column: string;
  readonly label: string;
  readonly usedAsIndicator: boolean;
  readonly groups: readonly NativeMultiModGroupOptionV1[];
}

export type NativeMultiModRunnerAvailabilityV1 =
  | {
      readonly state: "absent" | "blocked";
      readonly reason: string;
      readonly mgaGroupEligibility?: NativeMgaGroupEligibilityV1 | null;
    }
  | {
      readonly state: "executable";
      readonly capabilityCellId: string;
      readonly mgaGroupEligibility?: NativeMgaGroupEligibilityV1 | null;
    };

export interface NativeMultiModLabsWorkspaceProps {
  readonly model: SemModelV4;
  readonly caseCount: number;
  readonly groupingColumns?: readonly NativeMultiModGroupingColumnV1[];
  readonly runnerAvailability?: NativeMultiModRunnerAvailabilityV1;
  readonly operationPending?: boolean;
  readonly initialTab?: MultiModTabV1;
  /** Strictly parsed, completed-or-fail-closed archive attachment; absent means no result is rendered. */
  readonly validatedResult?: MultiModResultAttachmentV1;
  /** Canonical projection built only after strict Archive V6 reopen. */
  readonly canonicalResultDocument?: CanonicalResultDocumentV2;
  readonly rawSidecarExportAuthority?: NativeMultiModRawSidecarExportAuthorityV1;
  /** Resolves native support for this exact parsed request; stale decisions must not enable Calculate. */
  readonly onAssessRuntime?: (
    request: MultiModRecipeConfigV1,
  ) => Promise<NativeMultiModRunnerAvailabilityV1>;
  readonly onStageRecipe?: (
    request: MultiModRecipeConfigV1,
  ) => void | Promise<void>;
  readonly onExecute?: (
    request: MultiModRecipeConfigV1,
  ) => void | Promise<void>;
  readonly onPrepareRawProbeMetrics?: (
    config: GeneralSemConditionalProcessConfigV2,
    moderatorId: string,
    orientationSign: -1 | 1,
  ) => Promise<readonly ConditionalRawProbeFitMetricReceiptV2[]>;
}

export interface NativeMultiModPathCandidateV1 {
  readonly pathId: string;
  readonly orderedRelationIds: readonly string[];
  readonly orderedVariableIds: readonly string[];
  readonly label: string;
}

export interface NativeMultiModInteractionCandidateV1 {
  readonly id: string;
  readonly label: string;
  readonly moderatorIds: readonly string[];
  readonly order: 2 | 3;
}

export type NativeMultiModMgaParameterFamilyV1 =
  | "structural_path"
  | "outer_loading"
  | "outer_weight"
  | "interaction_gamma"
  | "three_way_delta"
  | "conditional_interaction"
  | "simple_slope";

export interface NativeMultiModMgaParameterCandidateV1 {
  readonly id: string;
  readonly label: string;
  readonly family: NativeMultiModMgaParameterFamilyV1;
  readonly profiles: readonly MgaModelProfileV1[];
}

export interface NativeMultiModModelInventoryV1 {
  readonly variables: readonly {
    id: string;
    label: string;
    observed: boolean;
    scale: string | null;
  }[];
  readonly paths: readonly NativeMultiModPathCandidateV1[];
  readonly interactions: readonly NativeMultiModInteractionCandidateV1[];
  readonly hocIds: readonly string[];
  readonly parameterIds: readonly string[];
  readonly mgaParameters: readonly NativeMultiModMgaParameterCandidateV1[];
}

const ORDINARY_MGA_PROFILES: readonly MgaModelProfileV1[] = [
  "general_sem_pls",
  "multiple_nonnested_hoc",
  "case_weighted_pls",
  "frequency_weighted_pls",
  "reflective_plsc",
];

const INTERACTION_MGA_PROFILES: readonly MgaModelProfileV1[] = [
  "multiple_two_way_moderation",
  "bounded_three_way_moderation",
  "bounded_two_way_moderated_mediation",
];

const EMPTY_MICOM_CHECKLIST: MicomConfiguralChecklistV1 = {
  identical_indicators_and_coding: false,
  identical_data_treatment: false,
  identical_algorithm_settings: false,
  identical_model_specification: false,
  deterministic_sign_orientation_reviewed: false,
  analyst_review_confirmed: false,
};

const EMPTY_CAUSAL_CHECKLIST: CausalIdentificationChecklistV1 = {
  temporal_order_declared: false,
  adjustment_set_justified: false,
  consistency_assumption_acknowledged: false,
  no_unmeasured_treatment_outcome_confounding_acknowledged: false,
  no_unmeasured_treatment_mediator_confounding_acknowledged: false,
  no_unmeasured_mediator_outcome_confounding_acknowledged: false,
  no_exposure_induced_mediator_outcome_confounder_confirmed: false,
  no_recanting_witness_confirmed: false,
  linear_model_specification_reviewed: false,
  positivity_reviewed: false,
};

const LABELS = {
  identical_indicators_and_coding:
    "Indicators, coding, and data types are identical",
  identical_data_treatment:
    "Missing-data and preprocessing treatment are identical",
  identical_algorithm_settings:
    "Algorithm and resampling settings are identical",
  identical_model_specification:
    "Structural and measurement specifications are identical",
  deterministic_sign_orientation_reviewed:
    "Construct sign orientation has been reviewed",
  analyst_review_confirmed: "I completed the MICOM Step 1 configural review",
} satisfies Record<keyof MicomConfiguralChecklistV1, string>;

const CAUSAL_LABELS = {
  temporal_order_declared: "Temporal order is declared and defensible",
  adjustment_set_justified: "The adjustment set is justified",
  consistency_assumption_acknowledged: "Consistency is acknowledged",
  no_unmeasured_treatment_outcome_confounding_acknowledged:
    "No unmeasured treatment–outcome confounding is assumed",
  no_unmeasured_treatment_mediator_confounding_acknowledged:
    "No unmeasured treatment–mediator confounding is assumed",
  no_unmeasured_mediator_outcome_confounding_acknowledged:
    "No unmeasured mediator–outcome confounding is assumed",
  no_exposure_induced_mediator_outcome_confounder_confirmed:
    "No exposure-induced mediator–outcome confounder is present",
  no_recanting_witness_confirmed: "No recanting-witness setting is present",
  linear_model_specification_reviewed:
    "The explicit observed-data linear equations are scientifically justified",
  positivity_reviewed: "Positivity has been reviewed at the requested contrast",
} satisfies Record<keyof CausalIdentificationChecklistV1, string>;

function unique<T>(values: readonly T[]): T[] {
  return [...new Set(values)];
}

function utf8ByteLengthV1(value: string): number {
  return new TextEncoder().encode(value).byteLength;
}

const HETEROGENEITY_ALGORITHM_ORDER_V2: readonly HeterogeneityAlgorithmV2[] = [
  "fimix_pls_v2",
  "pls_pos_published_v2",
  "pls_pos_destination_scored_interactions_v2",
];

interface NativeHeterogeneityLockCandidateV2 {
  readonly algorithm: HeterogeneityAlgorithmV2;
  readonly k: number;
}

export type NativeHeterogeneityDiscoveryLockSourceV2 =
  | { readonly ready: false; readonly reason: string }
  | {
      readonly ready: true;
      readonly discoveryResultIdentitySha256: string;
      readonly profile: HeterogeneityInteractionProfileV2;
      readonly candidateK: readonly number[];
      readonly algorithms: readonly HeterogeneityAlgorithmV2[];
      readonly selectable: readonly NativeHeterogeneityLockCandidateV2[];
    };

export function nativeHeterogeneityDiscoveryLockSourceV2(
  attachment: MultiModResultAttachmentV1 | undefined,
): NativeHeterogeneityDiscoveryLockSourceV2 {
  if (!attachment)
    return {
      ready: false,
      reason: "Complete and strictly reopen a discovery run before locking inference.",
    };
  if (attachment.result.kind !== "pls_heterogeneity_analysis_v2")
    return {
      ready: false,
      reason: "The completed result is not a heterogeneity discovery result.",
    };
  const analysis = attachment.result.analysis;
  if (
    analysis.inference_lock ||
    analysis.locked_algorithm !== undefined ||
    analysis.locked_k !== undefined ||
    analysis.bootstrap_ledger
  )
    return {
      ready: false,
      reason: "Only an unlocked discovery result can authorize a new fixed-K inference run.",
    };
  const segmentation = analysis.candidates.filter(
    (candidate) => candidate.method.kind === "segmentation",
  );
  const candidateK = unique(segmentation.map((candidate) => candidate.k)).sort(
    (left, right) => left - right,
  );
  const algorithms = unique(
    segmentation.flatMap((candidate) =>
      candidate.method.kind === "segmentation"
        ? [candidate.method.algorithm]
        : [],
    ),
  ).sort(
    (left, right) =>
      HETEROGENEITY_ALGORITHM_ORDER_V2.indexOf(left) -
      HETEROGENEITY_ALGORITHM_ORDER_V2.indexOf(right),
  );
  const completeInventory =
    candidateK.length > 0 &&
    algorithms.length > 0 &&
    segmentation.length === candidateK.length * algorithms.length &&
    algorithms.every((algorithm) =>
      candidateK.every((k) =>
        segmentation.some(
          (candidate) =>
            candidate.method.kind === "segmentation" &&
            candidate.method.algorithm === algorithm &&
            candidate.k === k,
        ),
      ),
    );
  if (!completeInventory)
    return {
      ready: false,
      reason: "The discovery result does not contain one complete algorithm-by-K candidate table.",
    };
  const selectable = segmentation.flatMap<NativeHeterogeneityLockCandidateV2>(
    (candidate) =>
      candidate.method.kind === "segmentation" &&
      candidate.state === "converged_stable"
        ? [{ algorithm: candidate.method.algorithm, k: candidate.k }]
        : [],
  );
  if (!selectable.length)
    return {
      ready: false,
      reason: "No discovery candidate reproduced a converged stable solution that can be locked.",
    };
  return {
    ready: true,
    discoveryResultIdentitySha256:
      analysis.discovery_result_identity_sha256,
    profile: analysis.profile,
    candidateK,
    algorithms,
    selectable,
  };
}

export function nativeMultiModModelInventoryV1(
  model: SemModelV4,
): NativeMultiModModelInventoryV1 {
  const labels = new Map(
    model.variables.map((variable) => [variable.id, variable.label]),
  );
  const outgoing = new Map<
    string,
    Array<{ target: string; relationId: string }>
  >();
  model.relations.forEach((relation) => {
    if (
      relation.kind !== "structural" ||
      relation.role === "control" ||
      relation.source === relation.target
    )
      return;
    outgoing.set(relation.source, [
      ...(outgoing.get(relation.source) ?? []),
      { target: relation.target, relationId: relation.id },
    ]);
  });
  outgoing.forEach((relations) =>
    relations.sort((left, right) =>
      left.relationId.localeCompare(right.relationId),
    ),
  );
  const paths: NativeMultiModPathCandidateV1[] = [];
  const seenPaths = new Set<string>();
  const visit = (
    source: string,
    current: string,
    variables: string[],
    relations: string[],
  ) => {
    if (relations.length >= 6 || paths.length >= 128) return;
    for (const edge of outgoing.get(current) ?? []) {
      if (variables.includes(edge.target)) continue;
      const nextVariables = [...variables, edge.target];
      const nextRelations = [...relations, edge.relationId];
      if (nextRelations.length >= 2) {
        const key = nextRelations.join("\u0000");
        if (!seenPaths.has(key)) {
          seenPaths.add(key);
          paths.push({
            pathId: `path:${nextRelations.join(">")}`,
            orderedRelationIds: nextRelations,
            orderedVariableIds: nextVariables,
            label: nextVariables.map((id) => labels.get(id) ?? id).join(" → "),
          });
        }
      }
      visit(source, edge.target, nextVariables, nextRelations);
    }
  };
  [...outgoing.keys()]
    .sort()
    .forEach((source) => visit(source, source, [source], []));

  const interactions = model.derived_terms
    .flatMap<NativeMultiModInteractionCandidateV1>((term) => {
      if (term.kind === "interaction") {
        return [
          {
            id: term.id,
            label: `${labels.get(term.predictor) ?? term.predictor} × ${labels.get(term.moderator) ?? term.moderator}`,
            moderatorIds: [term.moderator],
            order: 2,
          },
        ];
      }
      if (
        term.kind !== "interaction_v2" ||
        (term.operands.length !== 2 && term.operands.length !== 3)
      )
        return [];
      return [
        {
          id: term.id,
          label: term.operands.map((id) => labels.get(id) ?? id).join(" × "),
          moderatorIds: term.operands.slice(1),
          order: term.operands.length as 2 | 3,
        },
      ];
    })
    .sort((left, right) => left.id.localeCompare(right.id));

  const parameterLabels = new Map(
    model.parameters.map((parameter) => [parameter.id, parameter.label]),
  );
  const interactionOutputs = new Set(
    model.derived_terms.flatMap((term) =>
      term.kind === "interaction" || term.kind === "interaction_v2"
        ? [term.output]
        : [],
    ),
  );
  const mgaParameters = new Map<
    string,
    NativeMultiModMgaParameterCandidateV1
  >();
  const addMgaParameter = (
    candidate: NativeMultiModMgaParameterCandidateV1,
  ) => {
    const previous = mgaParameters.get(candidate.id);
    if (!previous) {
      mgaParameters.set(candidate.id, candidate);
      return;
    }
    mgaParameters.set(candidate.id, {
      ...previous,
      profiles: unique([...previous.profiles, ...candidate.profiles]),
    });
  };
  model.relations.forEach((relation) => {
    if (relation.kind === "structural") {
      if (interactionOutputs.has(relation.source)) return;
      addMgaParameter({
        id: relation.parameter,
        label:
          parameterLabels.get(relation.parameter) ??
          `${labels.get(relation.source) ?? relation.source} → ${labels.get(relation.target) ?? relation.target}`,
        family: "structural_path",
        profiles: [...ORDINARY_MGA_PROFILES, ...INTERACTION_MGA_PROFILES],
      });
      return;
    }
    if (relation.kind === "measurement_effect") {
      addMgaParameter({
        id: relation.parameter,
        label:
          parameterLabels.get(relation.parameter) ??
          `Loading · ${labels.get(relation.construct) ?? relation.construct} → ${labels.get(relation.indicator) ?? relation.indicator}`,
        family: "outer_loading",
        profiles: ORDINARY_MGA_PROFILES,
      });
      return;
    }
    if (relation.kind === "measurement_causal") {
      addMgaParameter({
        id: relation.parameter,
        label:
          parameterLabels.get(relation.parameter) ??
          `Weight · ${labels.get(relation.indicator) ?? relation.indicator} → ${labels.get(relation.composite) ?? relation.composite}`,
        family: "outer_weight",
        profiles: ORDINARY_MGA_PROFILES,
      });
    }
  });

  const interactionEffectRelation = (
    output: string,
    focalRelationId: string,
  ) => {
    const focal = model.relations.find(
      (relation) =>
        relation.kind === "structural" && relation.id === focalRelationId,
    );
    if (!focal || focal.kind !== "structural") return null;
    return (
      model.relations.find(
        (relation) =>
          relation.kind === "structural" &&
          relation.source === output &&
          relation.target === focal.target,
      ) ?? null
    );
  };
  const moderatorProbeCount = (moderatorId: string): 2 | 3 => {
    const indicators = model.relations.flatMap((relation) => {
      if (
        relation.kind === "measurement_effect" &&
        relation.construct === moderatorId
      )
        return [relation.indicator];
      if (
        relation.kind === "measurement_causal" &&
        relation.composite === moderatorId
      )
        return [relation.indicator];
      return [];
    });
    const observedId = indicators.length === 1 ? indicators[0] : moderatorId;
    const observed = model.variables.find(
      (variable) => variable.id === observedId && variable.kind === "observed",
    );
    if (!observed || observed.kind !== "observed" || observed.scale !== "binary")
      return 3;
    const categories = observed.categories
      .map((category) => Number(category.trim()))
      .filter(Number.isFinite)
      .sort((left, right) => left - right);
    return indicators.length <= 1 &&
      categories.length === 2 &&
      categories[0] === 0 &&
      categories[1] === 1
      ? 2
      : 3;
  };
  model.derived_terms.forEach((term) => {
    if (term.kind !== "interaction" && term.kind !== "interaction_v2") return;
    const operands =
      term.kind === "interaction"
        ? [term.predictor, term.moderator]
        : term.operands;
    const effect = interactionEffectRelation(term.output, term.focal_relation);
    if (operands.length === 2) {
      if (effect) {
        addMgaParameter({
          id: effect.id,
          label: `Interaction gamma · ${operands.map((id) => labels.get(id) ?? id).join(" × ")}`,
          family: "interaction_gamma",
          profiles: INTERACTION_MGA_PROFILES,
        });
      }
      const moderatorId = operands[1];
      (["minus_1", "zero", "plus_1"] as const).forEach((probe) =>
        addMgaParameter({
          id: `simple_slope:${term.id}:${moderatorId}:${probe}`,
          label: `Simple slope · ${labels.get(operands[0]) ?? operands[0]} at ${labels.get(moderatorId) ?? moderatorId} ${probe.replace("_", " ")}`,
          family: "simple_slope",
          profiles: [
            "multiple_two_way_moderation",
            "bounded_two_way_moderated_mediation",
          ],
        }),
      );
      return;
    }
    if (operands.length !== 3) return;
    addMgaParameter({
      id: `three_way_delta:${term.id}`,
      label: `Three-way delta · ${operands.map((id) => labels.get(id) ?? id).join(" × ")}`,
      family: "three_way_delta",
      profiles: ["bounded_three_way_moderation"],
    });
    const firstCount = moderatorProbeCount(operands[1]);
    const secondCount = moderatorProbeCount(operands[2]);
    for (let z = 0; z < secondCount; z += 1) {
      addMgaParameter({
        id: `three_way_conditional_xw:${term.id}:z${z}`,
        label: `Conditional X×W · ${term.id} · Z probe ${z + 1}`,
        family: "conditional_interaction",
        profiles: ["bounded_three_way_moderation"],
      });
      for (let w = 0; w < firstCount; w += 1) {
        addMgaParameter({
          id: `three_way_simple_x:${term.id}:w${w}:z${z}`,
          label: `Three-way simple slope · ${term.id} · W${w + 1}/Z${z + 1}`,
          family: "simple_slope",
          profiles: ["bounded_three_way_moderation"],
        });
      }
    }
  });

  return {
    variables: model.variables
      .map((variable) => ({
        id: variable.id,
        label: variable.label,
        observed: variable.kind === "observed",
        scale: variable.kind === "observed" ? variable.scale : null,
      }))
      .sort((left, right) => left.id.localeCompare(right.id)),
    paths: paths.sort((left, right) => left.pathId.localeCompare(right.pathId)),
    interactions,
    hocIds: model.derived_terms
      .filter((term) => term.kind === "higher_order")
      .map((term) => term.id)
      .sort(),
    parameterIds: model.parameters.map((parameter) => parameter.id).sort(),
    mgaParameters: [...mgaParameters.values()].sort((left, right) =>
      left.id.localeCompare(right.id),
    ),
  };
}

function fallbackGroupingColumns(
  model: SemModelV4,
): NativeMultiModGroupingColumnV1[] {
  if (model.group.kind !== "observed_groups") return [];
  const group = model.group;
  return [
    {
      column: group.grouping_variable,
      label:
        model.variables.find(
          (variable) => variable.id === group.grouping_variable,
        )?.label ?? group.grouping_variable,
      usedAsIndicator: model.relations.some(
        (relation) =>
          (relation.kind === "measurement_effect" ||
            relation.kind === "measurement_causal") &&
          relation.indicator === group.grouping_variable,
      ),
      groups: group.levels.map((level) => ({
        groupId: level.id,
        label: level.label,
        value: { kind: "text", value: level.value },
      })),
    },
  ];
}

function contractAssessment<T>(build: () => T): {
  value: T | null;
  error: MultiModContractErrorV1 | null;
} {
  try {
    return { value: build(), error: null };
  } catch (error) {
    if (error instanceof MultiModContractErrorV1) return { value: null, error };
    throw error;
  }
}

function MultiModChecklist({
  value,
  onChange,
  idPrefix,
}: {
  value: MicomConfiguralChecklistV1;
  onChange: (value: MicomConfiguralChecklistV1) => void;
  idPrefix: string;
}) {
  return (
    <fieldset className="nd-multimod-checklist">
      <legend>MICOM Step 1 — configural invariance review</legend>
      {(Object.keys(LABELS) as Array<keyof MicomConfiguralChecklistV1>).map(
        (field) => (
          <label key={field} htmlFor={`${idPrefix}-${field}`}>
            <input
              id={`${idPrefix}-${field}`}
              type="checkbox"
              checked={value[field]}
              onChange={(event) =>
                onChange({ ...value, [field]: event.target.checked })
              }
            />
            <span>{LABELS[field]}</span>
          </label>
        ),
      )}
    </fieldset>
  );
}

function CostAndActions({
  request,
  error,
  predictedBytes,
  runnerAvailability,
  operationPending,
  onAssessRuntime,
  onStageRecipe,
  onExecute,
  onRuntimeAssessed,
}: {
  request: MultiModRecipeConfigV1 | null;
  error: MultiModContractErrorV1 | null;
  predictedBytes: number;
  runnerAvailability?: NativeMultiModRunnerAvailabilityV1;
  operationPending: boolean;
  onAssessRuntime?: NativeMultiModLabsWorkspaceProps["onAssessRuntime"];
  onStageRecipe?: NativeMultiModLabsWorkspaceProps["onStageRecipe"];
  onExecute?: NativeMultiModLabsWorkspaceProps["onExecute"];
  onRuntimeAssessed?: (availability: NativeMultiModRunnerAvailabilityV1) => void;
}) {
  const cost = multiModSidecarCostStateV1(predictedBytes);
  const requestKey = request ? JSON.stringify(request) : "";
  const [assessedAvailability, setAssessedAvailability] = useState<
    NativeMultiModRunnerAvailabilityV1 | undefined
  >(runnerAvailability);
  const [runtimeAssessmentPending, setRuntimeAssessmentPending] =
    useState(false);
  useEffect(() => {
    let live = true;
    if (!request || !onAssessRuntime) {
      setAssessedAvailability(runnerAvailability);
      if (runnerAvailability) onRuntimeAssessed?.(runnerAvailability);
      setRuntimeAssessmentPending(false);
      return () => {
        live = false;
      };
    }
    setAssessedAvailability({
      state: "blocked",
      reason: "Checking native support for this exact profile…",
    });
    setRuntimeAssessmentPending(true);
    void onAssessRuntime(request)
      .then((availability) => {
        if (live) {
          setAssessedAvailability(availability);
          onRuntimeAssessed?.(availability);
        }
      })
      .catch((reason) => {
        if (live) {
          const unavailable: NativeMultiModRunnerAvailabilityV1 = {
            state: "blocked",
            reason: reason instanceof Error ? reason.message : String(reason),
          };
          setAssessedAvailability(unavailable);
          onRuntimeAssessed?.(unavailable);
        }
      })
      .finally(() => {
        if (live) setRuntimeAssessmentPending(false);
      });
    return () => {
      live = false;
    };
    // The parsed request is an immutable value object. Its JSON identity binds
    // the asynchronous decision to the exact profile shown by this form.
  }, [onAssessRuntime, onRuntimeAssessed, requestKey, runnerAvailability]);
  const runnerExecutable = assessedAvailability?.state === "executable";
  const unavailableReason =
    assessedAvailability?.state === "absent" ||
    assessedAvailability?.state === "blocked"
      ? assessedAvailability.reason
      : "The MultiMod native runner command is not connected in this build.";
  const pending = operationPending || runtimeAssessmentPending;
  const [message, setMessage] = useState<string | null>(null);
  const submit = async (kind: "stage" | "execute") => {
    if (!request) return;
    setMessage(null);
    try {
      if (kind === "stage") await onStageRecipe?.(request);
      else await onExecute?.(request);
      setMessage(
        kind === "stage"
          ? "Recipe V4 configuration staged."
          : "MultiMod calculation submitted.",
      );
    } catch (reason) {
      setMessage(reason instanceof Error ? reason.message : String(reason));
    }
  };
  return (
    <footer className="nd-multimod-actions">
      <div className={`nd-multimod-cost ${cost}`} role="status">
        <strong>Predicted result sidecar</strong>
        <span>
          {(predictedBytes / (1024 * 1024)).toFixed(1)} MiB ·{" "}
          {cost === "blocked"
            ? "exceeds the 512 MiB run cap"
            : cost === "warning"
              ? "review before running"
              : "within the normal envelope"}
        </span>
      </div>
      {error ? (
        <div className="nd-form-error" role="alert">
          <strong>{error.message}</strong>
          <span>
            <code>{error.code}</code> at <code>{error.path}</code>
          </span>
        </div>
      ) : null}
      <div className="nd-cbsem-v4-actions">
        <button
          type="button"
          disabled={!request || !onStageRecipe || pending || cost === "blocked"}
          onClick={() => void submit("stage")}
        >
          <Archive size={15} aria-hidden="true" />
          Stage Recipe V4
        </button>
        <button
          className="primary"
          type="button"
          disabled={
            !request ||
            !runnerExecutable ||
            !onExecute ||
            pending ||
            cost === "blocked"
          }
          onClick={() => void submit("execute")}
          title={!runnerExecutable ? unavailableReason : undefined}
        >
          <Play size={15} aria-hidden="true" />
          {pending ? "Checking…" : "Calculate"}
        </button>
      </div>
      {!runnerExecutable ? (
        <p className="nd-dialog-note" role="note">
          Configuration and cost review are available in Labs. Calculation
          remains disabled until native availability says this exact profile is
          executable. {unavailableReason}
        </p>
      ) : null}
      {message ? (
        <p role="status" aria-live="polite">
          {message}
        </p>
      ) : null}
    </footer>
  );
}

function MgaFlow(
  props: NativeMultiModLabsWorkspaceProps & {
    columns: readonly NativeMultiModGroupingColumnV1[];
    inventory: NativeMultiModModelInventoryV1;
  },
) {
  const eligibleColumns = props.columns.filter(
    (column) => !column.usedAsIndicator,
  );
  const [columnId, setColumnId] = useState(eligibleColumns[0]?.column ?? "");
  const activeColumn =
    eligibleColumns.find((column) => column.column === columnId) ?? null;
  const [selectedGroupIds, setSelectedGroupIds] = useState<string[]>(
    activeColumn?.groups.slice(0, 2).map((group) => group.groupId) ?? [],
  );
  const [profile, setProfile] = useState<MgaModelProfileV1>("general_sem_pls");
  const [procedures, setProcedures] = useState<MgaProcedureV1[]>([
    "micom_pairwise",
    "pairwise_permutation",
  ]);
  const [weightColumn, setWeightColumn] = useState("");
  const [exactRuntimeAvailability, setExactRuntimeAvailability] = useState<
    NativeMultiModRunnerAvailabilityV1 | undefined
  >(undefined);
  const [samples, setSamples] = useState(5_000);
  const [checklist, setChecklist] = useState<MicomConfiguralChecklistV1>(
    EMPTY_MICOM_CHECKLIST,
  );
  const [allPairsConfirmed, setAllPairsConfirmed] = useState(false);
  const [alternative, setAlternative] = useState<
    "two_sided" | "less" | "greater"
  >("two_sided");
  const [multiplicity, setMultiplicity] =
    useState<MgaMultigroupV1["multiplicity"]>("holm");
  const availableParameters = useMemo(
    () =>
      props.inventory.mgaParameters.filter((parameter) =>
        parameter.profiles.includes(profile),
      ),
    [profile, props.inventory.mgaParameters],
  );
  const [selectedParameterIds, setSelectedParameterIds] = useState<string[]>(
    () => availableParameters.map((parameter) => parameter.id),
  );
  useEffect(() => {
    setSelectedParameterIds(availableParameters.map((parameter) => parameter.id));
  }, [availableParameters]);
  const selectedGroupOptions = (activeColumn?.groups ?? []).filter((group) =>
    selectedGroupIds.includes(group.groupId),
  );
  const groups: SelectedGroupV1[] = selectedGroupOptions.map((group) => ({
    group_id: group.groupId,
    label: group.label,
    value: group.value,
  }));
  const resolvedProcedures = unique([
    ...procedures.filter(
      (procedure) =>
        groups.length > 2 || procedure !== "parametric_wald_omnibus",
    ),
    ...(groups.length > 2 ? ["omnibus_max_spread_permutation" as const] : []),
  ]);
  const knownCompleteCases = selectedGroupOptions
    .map((group) => group.completeCases)
    .filter((count): count is number => count !== undefined);
  const imbalance =
    knownCompleteCases.length === groups.length && knownCompleteCases.length > 1
      ? Math.max(...knownCompleteCases) / Math.min(...knownCompleteCases)
      : null;
  const groupPreflightError = profile === "frequency_weighted_pls"
    ? null
    : knownCompleteCases.some((count) => count < 10)
    ? new MultiModContractErrorV1(
        "mga_multigroup_v1.minimum_group_size",
        "mga_multigroup.groups",
        "Every selected group requires at least 10 complete cases.",
      )
    : imbalance !== null && imbalance > 10
      ? new MultiModContractErrorV1(
          "mga_multigroup_v1.group_imbalance",
          "mga_multigroup.groups",
          "Selected groups exceed the maximum 10:1 complete-case imbalance.",
        )
      : null;
  const parameterPreflightError = !availableParameters.length
    ? new MultiModContractErrorV1(
        "mga_multigroup_v1.parameter_inventory_empty",
        "mga_multigroup.selected_parameter_ids",
        "The exact MGA refitter has no publishable parameter identity for this profile.",
      )
    : !selectedParameterIds.length
      ? new MultiModContractErrorV1(
          "mga_multigroup_v1.parameters_empty",
          "mga_multigroup.selected_parameter_ids",
          "Select at least one exact path, loading, weight, gamma, delta, or slope target.",
        )
      : null;
  const assessment = contractAssessment(() =>
    parseMgaMultigroupV1({
      schema_version: 1,
      profile,
      grouping_column: columnId,
      groups,
      comparison_plan: {
        kind: "all_pairs",
        heavy_run_confirmed: allPairsConfirmed,
      },
      procedures: resolvedProcedures,
      permutation_samples: samples,
      bootstrap_samples: samples,
      seed: 42,
      confidence_level: 0.95,
      alpha: 0.05,
      alternative,
      multiplicity,
      configural_checklist: checklist,
      ...(profile === "case_weighted_pls"
        ? { weight: { kind: "case", column: weightColumn } }
        : {}),
      ...(profile === "frequency_weighted_pls"
        ? { weight: { kind: "frequency", column: weightColumn } }
        : {}),
      selected_parameter_ids: selectedParameterIds,
    }),
  );
  const pairCount = (groups.length * (groups.length - 1)) / 2;
  const request =
    assessment.value && !groupPreflightError && !parameterPreflightError
      ? ({ kind: "mga_multigroup_v1", config: assessment.value } as const)
      : null;
  return (
    <div className="nd-multimod-flow">
      <ol className="nd-multimod-steps" aria-label="MGA workflow">
        <li className={columnId ? "ready" : "blocked"}>Groups</li>
        <li className={groups.length >= 2 ? "ready" : "blocked"}>
          Comparisons
        </li>
        <li
          className={
            micomConfiguralChecklistCompleteV1(checklist) ? "ready" : "blocked"
          }
        >
          MICOM review
        </li>
        <li>Inference</li>
        <li>Cost</li>
      </ol>
      <div className="nd-multimod-grid">
        <fieldset>
          <legend>1. Groups</legend>
          <label htmlFor="nd-multimod-mga-column">
            Grouping column
            <select
              id="nd-multimod-mga-column"
              value={columnId}
              onChange={(event) => {
                const next = event.target.value;
                setColumnId(next);
                setSelectedGroupIds(
                  eligibleColumns
                    .find((column) => column.column === next)
                    ?.groups.slice(0, 2)
                    .map((group) => group.groupId) ?? [],
                );
              }}
            >
              <option value="">Choose a non-indicator column</option>
              {eligibleColumns.map((column) => (
                <option key={column.column} value={column.column}>
                  {column.label} [{column.column}]
                </option>
              ))}
            </select>
          </label>
          <div
            className="nd-multimod-option-list"
            role="group"
            aria-label="Selected groups"
          >
            {(activeColumn?.groups ?? []).map((group) => (
              <label key={group.groupId}>
                <input
                  type="checkbox"
                  checked={selectedGroupIds.includes(group.groupId)}
                  onChange={(event) =>
                    setSelectedGroupIds((current) =>
                      event.target.checked
                        ? unique([...current, group.groupId])
                        : current.filter((id) => id !== group.groupId),
                    )
                  }
                />
                <span>
                  {group.label}
                  <small>
                    {group.completeCases == null
                      ? "Physical complete rows verified at native preflight"
                      : `${group.completeCases.toLocaleString()} physical complete rows`}
                  </small>
                </span>
              </label>
            ))}
          </div>
          {!eligibleColumns.length ? (
            <p className="nd-form-error" role="alert">
              No profiled grouping column is available outside the model
              indicators.
            </p>
          ) : null}
        </fieldset>
        <fieldset>
          <legend>2. Comparison and profile</legend>
          <label htmlFor="nd-multimod-mga-profile">
            Labs profile envelope
            <select
              id="nd-multimod-mga-profile"
              value={profile}
              onChange={(event) =>
                setProfile(event.target.value as MgaModelProfileV1)
              }
            >
              <option value="general_sem_pls">
                Ordinary recursive General SEM PLS
              </option>
              <option value="multiple_two_way_moderation">
                Multiple two-way moderation
              </option>
              <option value="bounded_three_way_moderation">
                Bounded three-way moderation
              </option>
              <option value="bounded_two_way_moderated_mediation">
                Bounded two-way moderated mediation
              </option>
              <option value="multiple_nonnested_hoc">
                Multiple nonnested HOCs
              </option>
              <option value="case_weighted_pls">
                Positive case-weighted PLS
              </option>
              <option value="frequency_weighted_pls">
                Positive-integer frequency-weighted PLS
              </option>
              <option value="reflective_plsc">Reflective PLSc</option>
            </select>
          </label>
          {profile === "case_weighted_pls" ||
          profile === "frequency_weighted_pls" ? (
            <label htmlFor="nd-multimod-mga-weight">
              {profile === "case_weighted_pls"
                ? "Positive case-weight column"
                : "Positive integer frequency column"}
              <input
                id="nd-multimod-mga-weight"
                value={weightColumn}
                onChange={(event) => setWeightColumn(event.target.value)}
              />
            </label>
          ) : null}
          <dl className="nd-property-list">
            <div>
              <dt>Pairwise comparisons</dt>
              <dd>{pairCount}</dd>
            </div>
            <div>
              <dt>Omnibus</dt>
              <dd>
                {groups.length > 2
                  ? "Max-spread permutation first"
                  : "Not applicable to two groups"}
              </dd>
            </div>
          </dl>
          {groups.length === 20 ? (
            <label className="nd-checkbox-row">
              <input
                type="checkbox"
                checked={allPairsConfirmed}
                onChange={(event) => setAllPairsConfirmed(event.target.checked)}
              />
              <span>I confirm the heavy run containing all 190 pairs.</span>
            </label>
          ) : null}
        </fieldset>
      </div>
      <MultiModChecklist
        value={checklist}
        onChange={setChecklist}
        idPrefix="nd-multimod-mga-micom"
      />
      <fieldset className="nd-multimod-inline-fields">
        <legend>4. Inference and multiplicity</legend>
        <div
          className="nd-multimod-option-list"
          role="group"
          aria-label="MGA procedures"
        >
          {(
            [
              ["micom_pairwise", "MICOM Steps 2–3, pairwise"],
              ["pairwise_permutation", "Pairwise size-preserving permutation"],
              ["henseler_pls_mga", "Henseler PLS-MGA directional probability"],
              ["bootstrap_difference_bc", "Bootstrap difference · BC interval"],
              ["parametric_pooled_variance", "Parametric pooled variance"],
              ["parametric_welch_satterthwaite", "Welch–Satterthwaite"],
              ["parametric_wald_omnibus", "K-group inverse-variance Wald"],
            ] as const
          ).map(([procedure, label]) => {
            const disabled =
              procedure === "parametric_wald_omnibus" && groups.length < 3;
            return (
              <label
                key={procedure}
                className={disabled ? "disabled" : undefined}
              >
                <input
                  type="checkbox"
                  disabled={disabled}
                  checked={!disabled && procedures.includes(procedure)}
                  onChange={(event) =>
                    setProcedures((current) =>
                      event.target.checked
                        ? unique([...current, procedure])
                        : current.filter((value) => value !== procedure),
                    )
                  }
                />
                <span>{label}</span>
              </label>
            );
          })}
          <p className="nd-dialog-note">
            {groups.length > 2
              ? "The max-spread omnibus permutation is included before pairwise follow-up."
              : "Omnibus procedures require at least three selected groups."}
          </p>
        </div>
        <label htmlFor="nd-multimod-mga-samples">
          Permutation and bootstrap draws
          <input
            id="nd-multimod-mga-samples"
            type="number"
            min={5_000}
            max={10_000}
            step={500}
            value={samples}
            onChange={(event) => setSamples(Number(event.target.value))}
          />
        </label>
        <label htmlFor="nd-multimod-mga-tail">
          Alternative
          <select
            id="nd-multimod-mga-tail"
            value={alternative}
            onChange={(event) =>
              setAlternative(event.target.value as typeof alternative)
            }
          >
            <option value="two_sided">Two-sided</option>
            <option value="less">A minus B &lt; 0 (predeclared)</option>
            <option value="greater">A minus B &gt; 0 (predeclared)</option>
          </select>
        </label>
        <label htmlFor="nd-multimod-mga-multiplicity">
          Multiplicity
          <select
            id="nd-multimod-mga-multiplicity"
            value={multiplicity}
            onChange={(event) =>
              setMultiplicity(event.target.value as typeof multiplicity)
            }
          >
            <option value="holm">Holm FWER (confirmatory default)</option>
            <option value="bonferroni">Bonferroni</option>
            <option value="sidak">Sidak</option>
            <option value="benjamini_hochberg_exploratory">
              Benjamini–Hochberg (exploratory)
            </option>
            <option value="none_explicit">None (explicit)</option>
          </select>
        </label>
      </fieldset>
      <fieldset>
        <legend>Parameters to compare</legend>
        <p className="nd-dialog-note">
          Only identities emitted by the selected MGA point refitter are
          listed. Intercepts, variances, covariances, thresholds, means, and
          constraint-only parameters are excluded.
        </p>
        <div
          className="nd-multimod-option-list"
          role="group"
          aria-label="MGA parameters to compare"
        >
          {availableParameters.map((parameter) => (
            <label key={parameter.id}>
              <input
                type="checkbox"
                checked={selectedParameterIds.includes(parameter.id)}
                onChange={(event) =>
                  setSelectedParameterIds((current) =>
                    event.target.checked
                      ? unique([...current, parameter.id])
                      : current.filter((id) => id !== parameter.id),
                  )
                }
              />
              <span>
                {parameter.label}
                <small>
                  {parameter.family.replaceAll("_", " ")} · {parameter.id}
                </small>
              </span>
            </label>
          ))}
        </div>
        {parameterPreflightError ? (
          <p className="nd-form-error" role="alert">
            {parameterPreflightError.message}
          </p>
        ) : null}
      </fieldset>
      {profile === "frequency_weighted_pls" ? (
        exactRuntimeAvailability?.mgaGroupEligibility?.countBasis ===
          "frequency_expanded_cases" ? (
          <div
            className={`nd-multimod-result-callout ${
              exactRuntimeAvailability.mgaGroupEligibility.eligible
                ? "complete"
                : "blocked"
            }`}
            role={
              exactRuntimeAvailability.mgaGroupEligibility.eligible
                ? "status"
                : "alert"
            }
          >
            <strong>Frequency-expanded group eligibility</strong>
            <span>
              {exactRuntimeAvailability.mgaGroupEligibility.groups
                .map(
                  (group) =>
                    `${group.label}: ${group.effectiveCompleteCases.toLocaleString()} effective cases from ${group.physicalCompleteRows.toLocaleString()} physical rows`,
                )
                .join("; ")}
              . Maximum imbalance{" "}
              {exactRuntimeAvailability.mgaGroupEligibility.maximumImbalanceRatio ==
              null
                ? "not defined"
                : `${exactRuntimeAvailability.mgaGroupEligibility.maximumImbalanceRatio.toFixed(2)}:1`}
              .{" "}
              {exactRuntimeAvailability.mgaGroupEligibility.eligible
                ? "The exact count-space design is eligible."
                : `Calculation is blocked: ${exactRuntimeAvailability.mgaGroupEligibility.blockerCodes.join(", ")}.`}
            </span>
          </div>
        ) : (
          <p
            className="nd-inline-warning"
            role="note"
            data-multimod-blocker="multimod.mga.frequency_effective_counts_pending"
          >
            <AlertTriangle size={15} aria-hidden="true" />
            Generic grouping counts above are physical rows only. Calculate
            remains disabled until native preflight validates every selected
            frequency as a positive integer and recomputes 10/30-case and
            2:1/10:1 rules from frequency-expanded totals.
          </p>
        )
      ) : knownCompleteCases.length !== groups.length ? (
        <p className="nd-inline-warning" role="note">
          <AlertTriangle size={15} aria-hidden="true" />
          Complete-case size and imbalance will be rechecked by native preflight
          before execution.
        </p>
      ) : knownCompleteCases.some((count) => count < 30) ||
        (imbalance !== null && imbalance > 2) ? (
        <p className="nd-inline-warning" role="note">
          <AlertTriangle size={15} aria-hidden="true" />
          Review warning: at least one group has fewer than 30 complete cases or
          imbalance exceeds 2:1.
        </p>
      ) : null}
      <p className="nd-inline-warning" role="note">
        <AlertTriangle size={15} aria-hidden="true" />
        Partial measurement invariance is required before affected structural
        differences can be interpreted. Henseler directional probabilities are
        not ordinary two-sided p-values.
      </p>
      <CostAndActions
        request={request}
        error={
          groupPreflightError ?? parameterPreflightError ?? assessment.error
        }
        predictedBytes={predictMultiModSidecarBytesV1({
          kind: "mga",
          groupRows: selectedGroupOptions.map(
            (group) => group.completeCases ?? props.caseCount,
          ),
          profile,
          procedures: resolvedProcedures,
          pairCount,
          permutationSamples: samples,
          bootstrapSamples: samples,
          targets: Math.max(selectedParameterIds.length, 1),
          maximumTargetIdBytes: Math.max(
            ...selectedParameterIds.map(utf8ByteLengthV1),
            1,
          ),
          micomConstructs: Math.max(
            props.inventory.variables.filter((variable) => !variable.observed)
              .length,
            1,
          ),
        })}
        runnerAvailability={props.runnerAvailability}
        operationPending={Boolean(props.operationPending)}
        onAssessRuntime={props.onAssessRuntime}
        onStageRecipe={props.onStageRecipe}
        onExecute={props.onExecute}
        onRuntimeAssessed={setExactRuntimeAvailability}
      />
    </div>
  );
}

function HeterogeneityFlow(
  props: NativeMultiModLabsWorkspaceProps & {
    inventory: NativeMultiModModelInventoryV1;
  },
) {
  const [phase, setPhase] = useState<"discovery" | "inference">("discovery");
  const [profile, setProfile] = useState<HeterogeneityInteractionProfileV2>(
    props.inventory.interactions.length ? "p2_multi_two_way" : "p0_structural",
  );
  const [discoveryAlgorithms, setDiscoveryAlgorithms] = useState<
    HeterogeneityAlgorithmV2[]
  >(["fimix_pls_v2"]);
  const [candidateK, setCandidateK] = useState<number[]>([2, 3]);
  const [lockSelectionKey, setLockSelectionKey] = useState("");
  const [lockConfirmed, setLockConfirmed] = useState(false);
  const [bootstrap, setBootstrap] = useState(1_000);
  const [requestContrasts, setRequestContrasts] = useState(false);
  const [checklist, setChecklist] = useState<MicomConfiguralChecklistV1>(
    EMPTY_MICOM_CHECKLIST,
  );
  const discoverySource = useMemo(
    () => nativeHeterogeneityDiscoveryLockSourceV2(props.validatedResult),
    [props.validatedResult],
  );
  const lockKey = (candidate: NativeHeterogeneityLockCandidateV2) =>
    `${candidate.algorithm}:${candidate.k}`;
  const selectedLockCandidate = discoverySource.ready
    ? (discoverySource.selectable.find(
        (candidate) => lockKey(candidate) === lockSelectionKey,
      ) ?? discoverySource.selectable[0])
    : undefined;
  const lockReceipt: HeterogeneityInferenceLockReceiptV2 | null =
    discoverySource.ready && selectedLockCandidate && lockConfirmed
      ? {
          schema_version: 1,
          discovery_result_identity_sha256:
            discoverySource.discoveryResultIdentitySha256,
          discovery_candidate_k: [...discoverySource.candidateK],
          discovery_algorithms: [...discoverySource.algorithms],
          selected_algorithm: selectedLockCandidate.algorithm,
          selected_k: selectedLockCandidate.k,
          analyst_lock_confirmed: true,
          tandem_fimix_same_k_start_required:
            selectedLockCandidate.algorithm !== "fimix_pls_v2" &&
            discoverySource.algorithms.includes("fimix_pls_v2"),
        }
      : null;
  const phaseContract =
    phase === "discovery"
      ? {
          kind: "discovery",
          candidate_k: candidateK,
          algorithms: discoveryAlgorithms,
        }
      : { kind: "inference", lock: lockReceipt };
  const activeAlgorithms =
    phase === "discovery"
      ? discoveryAlgorithms
      : selectedLockCandidate
        ? [selectedLockCandidate.algorithm]
        : [];
  const activeProfile =
    phase === "inference" && discoverySource.ready
      ? discoverySource.profile
      : profile;
  const assessment = contractAssessment(() =>
    parsePlsUnobservedHeterogeneityConfigV2({
      schema_version: 2,
      profile: activeProfile,
      phase: phaseContract,
      seed: 42,
      fimix: {
        starts: 30,
        max_iterations: 5_000,
        relative_log_likelihood_tolerance: 1e-10,
        consecutive_converged_iterations: 3,
        likelihood_decrease_tolerance: 1e-9,
        residual_variance_floor: 1e-8,
        rank_tolerance: 1e-11,
        minimum_class_share: 0.05,
        required_reproducing_starts: 2,
        optimum_relative_log_likelihood_tolerance: 1e-8,
        optimum_maximum_coefficient_difference: 1e-6,
        optimum_mean_posterior_difference: 1e-4,
      },
      pls_pos: {
        starts: 10,
        strict_improvement_tolerance: 1e-10,
        stable_objective_tolerance: 1e-10,
        minimum_reproducing_starts: 2,
      },
      ...(activeAlgorithms.some((value) => value.startsWith("pls_pos")) &&
      requestContrasts
        ? {
            pos_common_metric: {
              schema_version: 1,
              request_segment_contrasts: true,
              permutation_samples: 5_000,
              configural_checklist: checklist,
              require_partial_compositional_invariance: true,
            },
          }
        : {}),
      ...(phase === "inference"
        ? {
            bootstrap: {
              resamples: bootstrap,
              seed: 42,
              confidence_level: 0.95,
            },
          }
        : {}),
    }),
  );
  const request = assessment.value
    ? ({
        kind: "pls_unobserved_heterogeneity_v2",
        config: assessment.value,
      } as const)
    : null;
  return (
    <div className="nd-multimod-flow">
      <ol
        className="nd-multimod-steps"
        aria-label="Latent segmentation workflow"
      >
        <li className="ready">Candidate K</li>
        <li className={phase === "discovery" ? "ready" : "blocked"}>
          Inspect diagnostics
        </li>
        <li className={phase === "inference" ? "ready" : "blocked"}>
          Lock method and K
        </li>
        <li>Bootstrap</li>
      </ol>
      <div className="nd-multimod-grid">
        <fieldset>
          <legend>Segmentation stage</legend>
          <label>
            <input
              type="radio"
              name="nd-multimod-heterogeneity-phase"
              checked={phase === "discovery"}
              onChange={() => setPhase("discovery")}
            />
            Discover candidates (never auto-select K)
          </label>
          <label>
            <input
              type="radio"
              name="nd-multimod-heterogeneity-phase"
              checked={phase === "inference"}
              disabled={!discoverySource.ready}
              aria-describedby="nd-multimod-discovery-lock-status"
              onChange={() => {
                if (!discoverySource.ready) return;
                setPhase("inference");
                setProfile(discoverySource.profile);
                setLockSelectionKey(
                  lockKey(discoverySource.selectable[0]),
                );
                setLockConfirmed(false);
              }}
            />
            Lock reviewed algorithm and K for inference
          </label>
          <p
            id="nd-multimod-discovery-lock-status"
            className={
              discoverySource.ready
                ? "nd-dialog-note"
                : "nd-form-error"
            }
            role="status"
          >
            {discoverySource.ready
              ? `Validated discovery ${discoverySource.discoveryResultIdentitySha256.slice(0, 12)}… exposes ${discoverySource.selectable.length} stable lock candidate(s).`
              : discoverySource.reason}
          </p>
        </fieldset>
        <fieldset>
          <legend>Scientific profile</legend>
          <label htmlFor="nd-multimod-heterogeneity-profile">
            Interaction profile
            <select
              id="nd-multimod-heterogeneity-profile"
              value={activeProfile}
              disabled={phase === "inference"}
              onChange={(event) => {
                const next = event.target
                  .value as HeterogeneityInteractionProfileV2;
                setProfile(next);
                setDiscoveryAlgorithms((current) =>
                  current.filter(
                    (value) =>
                      value === "fimix_pls_v2" ||
                      (next === "p0_structural"
                        ? value === "pls_pos_published_v2"
                        : value ===
                          "pls_pos_destination_scored_interactions_v2"),
                  ),
                );
              }}
            >
              <option value="p0_structural">P0 — structural only</option>
              <option value="p2_multi_two_way">
                P2 — multiple two-way interactions
              </option>
              <option value="p23_all_current">
                P23 — bounded three-way closure
              </option>
            </select>
          </label>
          {phase === "inference" ? (
            <label htmlFor="nd-multimod-heterogeneity-algorithm">
              Locked algorithm
              <select
                id="nd-multimod-heterogeneity-algorithm"
                value={selectedLockCandidate?.algorithm ?? ""}
                disabled={!discoverySource.ready}
                onChange={(event) => {
                  if (!discoverySource.ready) return;
                  const algorithm = event.target
                    .value as HeterogeneityAlgorithmV2;
                  const first = discoverySource.selectable.find(
                    (candidate) => candidate.algorithm === algorithm,
                  );
                  if (first) setLockSelectionKey(lockKey(first));
                  setLockConfirmed(false);
                }}
              >
                {(discoverySource.ready
                  ? unique(
                      discoverySource.selectable.map(
                        (candidate) => candidate.algorithm,
                      ),
                    )
                  : []
                ).map((algorithm) => (
                  <option key={algorithm} value={algorithm}>
                    {algorithm === "fimix_pls_v2"
                      ? "FIMIX-PLS V2"
                      : algorithm === "pls_pos_published_v2"
                        ? "PLS-POS published V2"
                        : "PLS-POS destination-scored interactions V2"}
                  </option>
                ))}
              </select>
            </label>
          ) : (
            <div
              className="nd-multimod-option-list"
              role="group"
              aria-label="Discovery algorithms"
            >
              {(
                [
                  "fimix_pls_v2",
                  profile === "p0_structural"
                    ? "pls_pos_published_v2"
                    : "pls_pos_destination_scored_interactions_v2",
                ] as HeterogeneityAlgorithmV2[]
              ).map((value) => (
                <label key={value}>
                  <input
                    type="checkbox"
                    checked={discoveryAlgorithms.includes(value)}
                    onChange={(event) =>
                      setDiscoveryAlgorithms((current) =>
                        event.target.checked
                          ? unique([...current, value])
                          : current.filter((entry) => entry !== value),
                      )
                    }
                  />
                  <span>
                    {value === "fimix_pls_v2"
                      ? "FIMIX-PLS V2"
                      : value === "pls_pos_published_v2"
                        ? "PLS-POS published V2"
                        : "PLS-POS destination-scored interactions V2"}
                  </span>
                </label>
              ))}
            </div>
          )}
        </fieldset>
      </div>
      {phase === "discovery" ? (
        <fieldset className="nd-multimod-option-list">
          <legend>Candidate K (K=1 remains the pooled baseline)</legend>
          {[2, 3, 4, 5].map((k) => (
            <label key={k}>
              <input
                type="checkbox"
                checked={candidateK.includes(k)}
                onChange={(event) =>
                  setCandidateK((current) =>
                    event.target.checked
                      ? [...current, k].sort()
                      : current.filter((value) => value !== k),
                  )
                }
              />
              <span>K = {k}</span>
            </label>
          ))}
        </fieldset>
      ) : (
        <fieldset className="nd-multimod-inline-fields">
          <legend>Locked inference</legend>
          <label htmlFor="nd-multimod-locked-k">
            Locked K
            <select
              id="nd-multimod-locked-k"
              value={selectedLockCandidate?.k ?? ""}
              disabled={!discoverySource.ready || !selectedLockCandidate}
              onChange={(event) => {
                if (!discoverySource.ready || !selectedLockCandidate) return;
                const k = Number(event.target.value);
                const selected = discoverySource.selectable.find(
                  (candidate) =>
                    candidate.algorithm === selectedLockCandidate.algorithm &&
                    candidate.k === k,
                );
                if (selected) setLockSelectionKey(lockKey(selected));
                setLockConfirmed(false);
              }}
            >
              {(discoverySource.ready && selectedLockCandidate
                ? discoverySource.selectable.filter(
                    (candidate) =>
                      candidate.algorithm === selectedLockCandidate.algorithm,
                  )
                : []
              ).map((candidate) => (
                <option key={lockKey(candidate)} value={candidate.k}>
                  K = {candidate.k}
                </option>
              ))}
            </select>
          </label>
          <label htmlFor="nd-multimod-seg-bootstrap">
            Bootstrap replicates
            <input
              id="nd-multimod-seg-bootstrap"
              type="number"
              min={500}
              max={10_000}
              step={100}
              value={bootstrap}
              onChange={(event) => setBootstrap(Number(event.target.value))}
            />
          </label>
          <label className="nd-checkbox-row">
            <input
              type="checkbox"
              checked={lockConfirmed}
              disabled={!discoverySource.ready || !selectedLockCandidate}
              onChange={(event) => setLockConfirmed(event.target.checked)}
            />
            <span>
              I reviewed the completed discovery diagnostics and explicitly
              lock this exact algorithm and K for fixed-model bootstrap.
            </span>
          </label>
          {discoverySource.ready ? (
            <p className="nd-dialog-note" role="note">
              The inference recipe will persist discovery identity{" "}
              <code>{discoverySource.discoveryResultIdentitySha256}</code>, all
              candidate K values, all discovery algorithms, and this explicit
              selection. K = 1 remains a nonselectable pooled baseline.
            </p>
          ) : null}
        </fieldset>
      )}
      {activeAlgorithms.some((value) => value.startsWith("pls_pos")) ? (
        <>
          <label className="nd-checkbox-row">
            <input
              type="checkbox"
              checked={requestContrasts}
              onChange={(event) => setRequestContrasts(event.target.checked)}
            />
            <span>
              Request segment-to-segment interaction contrasts on the pooled
              common metric
            </span>
          </label>
          {requestContrasts ? (
            <MultiModChecklist
              value={checklist}
              onChange={setChecklist}
              idPrefix="nd-multimod-pos-micom"
            />
          ) : null}
        </>
      ) : null}
      <p className="nd-inline-warning" role="note">
        <AlertTriangle size={15} aria-hidden="true" />
        Discovery and inference remain separate. Bootstrap reruns the fixed
        algorithm and K; ambiguous label matches are rejected, not replaced.
      </p>
      <CostAndActions
        request={request}
        error={assessment.error}
        predictedBytes={predictMultiModSidecarBytesV1({
          kind: "heterogeneity",
          rows: props.caseCount,
          candidateK:
            phase === "inference" && discoverySource.ready
              ? discoverySource.candidateK
              : candidateK,
          algorithms:
            phase === "inference" && discoverySource.ready
              ? discoverySource.algorithms
              : discoveryAlgorithms,
          fimixStarts: 30,
          fimixMaxIterations: 5_000,
          posStarts: 10,
          targets: Math.max(props.inventory.parameterIds.length, 1),
          ...(phase === "inference" && selectedLockCandidate
            ? {
                inference: {
                  selectedK: selectedLockCandidate.k,
                  bootstrapResamples: bootstrap,
                  requestCommonMetricContrasts: requestContrasts,
                  commonMetricPermutationSamples: 5_000,
                },
              }
            : {}),
        })}
        runnerAvailability={props.runnerAvailability}
        operationPending={Boolean(props.operationPending)}
        onAssessRuntime={props.onAssessRuntime}
        onStageRecipe={props.onStageRecipe}
        onExecute={props.onExecute}
      />
    </div>
  );
}

function parseProbeValues(value: string): number[] {
  return unique(
    value
      .split(",")
      .map((entry) => Number(entry.trim()))
      .filter(Number.isFinite),
  );
}

function parseJointProbeTuples(
  value: string,
): Array<{ tuple_id: string; values_by_moderator: Record<string, number> }> {
  return value
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      const separator = line.indexOf(":");
      const tupleId = separator >= 0 ? line.slice(0, separator).trim() : "";
      const assignments = (separator >= 0 ? line.slice(separator + 1) : line)
        .split(",")
        .map((entry) => entry.trim())
        .filter(Boolean);
      const values: Record<string, number> = {};
      assignments.forEach((assignment, index) => {
        const equals = assignment.indexOf("=");
        const moderatorId =
          equals >= 0 ? assignment.slice(0, equals).trim() : "";
        const parsed = Number(
          equals >= 0 ? assignment.slice(equals + 1).trim() : Number.NaN,
        );
        if (
          !moderatorId ||
          Object.prototype.hasOwnProperty.call(values, moderatorId)
        )
          values[`invalid:${index}`] = Number.NaN;
        else values[moderatorId] = parsed;
      });
      return { tuple_id: tupleId, values_by_moderator: values };
    });
}

function parseNamedPairs(
  value: string,
  leftKey: "left_tuple_id" | "left_group_id",
  rightKey: "right_tuple_id" | "right_group_id",
) {
  return value
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      const separator = line.indexOf(":");
      const id = separator >= 0 ? line.slice(0, separator).trim() : "";
      const pair = (separator >= 0 ? line.slice(separator + 1) : line)
        .split(">")
        .map((entry) => entry.trim());
      return {
        contrast_id: id,
        [leftKey]: pair[0] ?? "",
        [rightKey]: pair[1] ?? "",
      };
    });
}

function ConditionalProcessFlow(
  props: NativeMultiModLabsWorkspaceProps & {
    columns: readonly NativeMultiModGroupingColumnV1[];
    inventory: NativeMultiModModelInventoryV1;
  },
) {
  const [profile, setProfile] = useState<ConditionalProcessProfileV2>(
    "multi_two_way_percentile",
  );
  const [selectedPaths, setSelectedPaths] = useState<string[]>([]);
  const [selectedInteractions, setSelectedInteractions] = useState<string[]>(
    [],
  );
  const [selectedHocs, setSelectedHocs] = useState<string[]>([]);
  const [probeText, setProbeText] = useState<Record<string, string>>({});
  const [rawProbeModerators, setRawProbeModerators] = useState<string[]>([]);
  const [rawProbeOrientations, setRawProbeOrientations] = useState<Record<string, -1 | 1>>({});
  const [rawProbeReceipts, setRawProbeReceipts] = useState<Record<string, readonly ConditionalRawProbeFitMetricReceiptV2[]>>({});
  const [rawProbePending, setRawProbePending] = useState<string | null>(null);
  const [rawProbeFailure, setRawProbeFailure] = useState<string | null>(null);
  const [jointTupleLines, setJointTupleLines] = useState("");
  const [probeContrastLines, setProbeContrastLines] = useState("");
  const [groupContrastLines, setGroupContrastLines] = useState("");
  const [estimands, setEstimands] = useState<ConditionalProcessEstimandsV2>({
    conditional_specific_indirect: true,
    conditional_total_indirect: false,
    conditional_total_effect: false,
    scalar_index_when_affine: false,
    local_first_derivatives: false,
    local_second_and_cross_derivatives: false,
    finite_probe_contrasts: false,
  });
  const [outerResamples, setOuterResamples] = useState(5_000);
  const [innerResamples, setInnerResamples] = useState(200);
  const [alternative, setAlternative] = useState<
    "two_sided" | "less" | "greater"
  >("two_sided");
  const [groupColumn, setGroupColumn] = useState(
    props.columns.find((column) => !column.usedAsIndicator)?.column ?? "",
  );
  const [groupIds, setGroupIds] = useState<string[]>([]);
  const [weightColumn, setWeightColumn] = useState("");
  const selectedInteractionRows = props.inventory.interactions.filter(
    (interaction) => selectedInteractions.includes(interaction.id),
  );
  const moderatorIds = unique(
    selectedInteractionRows.flatMap((interaction) => interaction.moderatorIds),
  );
  const selectedPathRows = props.inventory.paths.filter((path) =>
    selectedPaths.includes(path.pathId),
  );
  const activeGroupColumn =
    props.columns.find((column) => column.column === groupColumn) ?? null;
  const groups = (activeGroupColumn?.groups ?? [])
    .filter((group) => groupIds.includes(group.groupId))
    .map((group) => ({
      group_id: group.groupId,
      label: group.label,
      value: group.value,
    }));
  const interval =
    profile === "multi_two_way_bca"
      ? "bca"
      : profile === "multi_two_way_studentized"
        ? "studentized"
        : "percentile";
  const jointTuples = parseJointProbeTuples(jointTupleLines);
  const probeContrasts = parseNamedPairs(
    probeContrastLines,
    "left_tuple_id",
    "right_tuple_id",
  );
  const groupContrasts = parseNamedPairs(
    groupContrastLines,
    "left_group_id",
    "right_group_id",
  );
  const requestedEstimands = {
    ...estimands,
    finite_probe_contrasts:
      estimands.finite_probe_contrasts || probeContrasts.length > 0,
  };
  const standardizedAssessment = contractAssessment(() =>
    parseGeneralSemConditionalProcessConfigV2({
      schema_version: 2,
      profile,
      paths: selectedPathRows.map((path) => ({
        path_id: path.pathId,
        ordered_relation_ids: path.orderedRelationIds,
      })),
      declared_interaction_ids: selectedInteractions,
      ...(profile === "bounded_three_way_percentile"
        ? {
            three_way_interaction_id: selectedInteractionRows.find(
              (interaction) => interaction.order === 3,
            )?.id,
          }
        : {}),
      hoc_ids: profile === "multiple_hoc_percentile" ? selectedHocs : [],
      moderator_ids: moderatorIds,
      probes: moderatorIds.map((moderatorId) => ({
        probe_id: `probe:${moderatorId}`,
        moderator_id: moderatorId,
        scale: "standardized_score",
        values: parseProbeValues(probeText[moderatorId] ?? "-1,0,1"),
      })),
      explicit_joint_tuples: jointTuples,
      probe_contrasts: probeContrasts,
      ...(profile === "grouped_percentile"
        ? { grouping_column: groupColumn }
        : {}),
      groups: profile === "grouped_percentile" ? groups : [],
      group_contrasts: profile === "grouped_percentile" ? groupContrasts : [],
      ...(profile === "case_weighted_percentile"
        ? { weight: { kind: "case", column: weightColumn } }
        : {}),
      ...(profile === "frequency_weighted_percentile"
        ? { weight: { kind: "frequency", column: weightColumn } }
        : {}),
      estimands: requestedEstimands,
      inference: {
        interval,
        alternative,
        outer_resamples:
          profile === "multi_two_way_studentized"
            ? Math.min(outerResamples, 5_000)
            : outerResamples,
        inner_resamples:
          profile === "multi_two_way_studentized" ? innerResamples : 0,
        seed: 42,
        confidence_level: 0.95,
      },
    }),
  );
  const assessment = contractAssessment(() => {
    if (!standardizedAssessment.value) throw standardizedAssessment.error;
    return parseGeneralSemConditionalProcessConfigV2({
      ...standardizedAssessment.value,
      probes: standardizedAssessment.value.probes.map((probe) =>
        rawProbeModerators.includes(probe.moderator_id)
          ? {
              ...probe,
              scale: "raw_observed_with_transformation_receipt",
              raw_fit_metric_receipts: rawProbeReceipts[probe.moderator_id] ?? [],
            }
          : probe),
    });
  });
  const request = assessment.value
    ? ({
        kind: "general_sem_conditional_process_v2",
        config: assessment.value,
      } as const)
    : null;
  const probePoints = moderatorIds.reduce(
    (total, id) =>
      total * Math.max(parseProbeValues(probeText[id] ?? "-1,0,1").length, 1),
    1,
  );
  const conditionalCells =
    Math.max(selectedPathRows.length, 1) *
    Math.max(jointTuples.length || probePoints, 1) *
    Math.max(profile === "grouped_percentile" ? groups.length : 1, 1);
  const derivativeMultiplier =
    1 +
    Number(requestedEstimands.local_first_derivatives) * moderatorIds.length +
    (Number(requestedEstimands.local_second_and_cross_derivatives) *
      moderatorIds.length *
      (moderatorIds.length + 1)) /
      2;
  const targetEstimate =
    conditionalCells * derivativeMultiplier +
    probeContrasts.length +
    (profile === "grouped_percentile" ? groupContrasts.length : 0);
  const rawMetricAuthorityKey = JSON.stringify({
    profile,
    selectedPaths,
    selectedInteractions,
    selectedHocs,
    groupColumn,
    groupIds,
    weightColumn,
  });
  useEffect(() => {
    setRawProbeReceipts({});
    setRawProbeFailure(null);
  }, [rawMetricAuthorityKey]);
  const prepareRawMetric = async (moderatorId: string) => {
    if (!props.onPrepareRawProbeMetrics || !standardizedAssessment.value) {
      setRawProbeFailure(
        "multimod.conditional.raw_probe_authority_unavailable: native fit-metric preparation is not available for this exact request.",
      );
      return;
    }
    setRawProbePending(moderatorId);
    setRawProbeFailure(null);
    try {
      const receipts = await props.onPrepareRawProbeMetrics(
        standardizedAssessment.value,
        moderatorId,
        rawProbeOrientations[moderatorId] ?? 1,
      );
      setRawProbeReceipts((current) => ({ ...current, [moderatorId]: receipts }));
    } catch (reason) {
      setRawProbeReceipts((current) => {
        const next = { ...current };
        delete next[moderatorId];
        return next;
      });
      setRawProbeFailure(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setRawProbePending(null);
    }
  };
  return (
    <div className="nd-multimod-flow">
      <ol
        className="nd-multimod-steps"
        aria-label="Conditional-process workflow"
      >
        <li className={selectedPaths.length ? "ready" : "blocked"}>
          Select paths
        </li>
        <li className={selectedInteractions.length ? "ready" : "blocked"}>
          Attach moderators
        </li>
        <li>Define probes</li>
        <li>Preview targets</li>
        <li>Calculate</li>
      </ol>
      <label htmlFor="nd-multimod-conditional-profile">
        Labs profile envelope
        <select
          id="nd-multimod-conditional-profile"
          value={profile}
          onChange={(event) => {
            const next = event.target.value as ConditionalProcessProfileV2;
            setProfile(next);
            if (
              [
                "multiple_hoc_percentile",
                "grouped_percentile",
                "case_weighted_percentile",
                "frequency_weighted_percentile",
              ].includes(next)
            )
              setAlternative("two_sided");
          }}
        >
          <option value="multi_two_way_percentile">
            Multiple two-way · percentile
          </option>
          <option value="multi_two_way_bca">
            Multiple two-way · full delete-one BCa
          </option>
          <option value="multi_two_way_studentized">
            Studentized bounded profile
          </option>
          <option value="bounded_three_way_percentile">
            Bounded three-way · percentile
          </option>
          <option value="multiple_hoc_percentile">
            Multiple HOC · percentile
          </option>
          <option value="grouped_percentile">
            Grouped · stratified percentile
          </option>
          <option value="case_weighted_percentile">
            Positive case weighted
          </option>
          <option value="frequency_weighted_percentile">
            Positive-integer frequency weighted
          </option>
        </select>
      </label>
      <div className="nd-multimod-grid">
        <fieldset className="nd-multimod-option-list">
          <legend>1. Explicit indirect paths</legend>
          {props.inventory.paths.map((path) => (
            <label key={path.pathId}>
              <input
                type="checkbox"
                checked={selectedPaths.includes(path.pathId)}
                onChange={(event) =>
                  setSelectedPaths((current) =>
                    event.target.checked
                      ? [...current, path.pathId]
                      : current.filter((id) => id !== path.pathId),
                  )
                }
              />
              <span>
                {path.label}
                <small>
                  {path.orderedRelationIds.length} edges · {path.pathId}
                </small>
              </span>
            </label>
          ))}
          {!props.inventory.paths.length ? (
            <p className="nd-form-error" role="alert">
              The current model has no directed indirect path of two through six
              edges.
            </p>
          ) : null}
        </fieldset>
        <fieldset className="nd-multimod-option-list">
          <legend>2. Authored interaction terms</legend>
          {props.inventory.interactions.map((interaction) => (
            <label key={interaction.id}>
              <input
                type="checkbox"
                checked={selectedInteractions.includes(interaction.id)}
                onChange={(event) =>
                  setSelectedInteractions((current) =>
                    event.target.checked
                      ? [...current, interaction.id]
                      : current.filter((id) => id !== interaction.id),
                  )
                }
              />
              <span>
                {interaction.label}
                <small>
                  {interaction.order === 3
                    ? "Three-way with required lower-order closure"
                    : "Two-way"}{" "}
                  · {interaction.id}
                </small>
              </span>
            </label>
          ))}
          {!props.inventory.interactions.length ? (
            <p className="nd-form-error" role="alert">
              Author one or more supported interaction terms on the Canvas
              first.
            </p>
          ) : null}
        </fieldset>
      </div>
      {moderatorIds.length ? (
        <fieldset className="nd-multimod-inline-fields">
          <legend>3. Frozen original-sample probes</legend>
          {moderatorIds.map((id) => {
            const variable = props.inventory.variables.find((candidate) => candidate.id === id);
            const rawEligible = variable?.observed === true
              && (variable.scale === "continuous" || variable.scale === "binary");
            const rawSelected = rawProbeModerators.includes(id);
            const receipts = rawProbeReceipts[id] ?? [];
            return <div className="nd-multimod-probe-card" key={id}>
              <label htmlFor={`nd-multimod-probe-${id}`}>
                {variable?.label ?? id}
                <input
                  id={`nd-multimod-probe-${id}`}
                  value={probeText[id] ?? "-1,0,1"}
                  onChange={(event) =>
                    setProbeText((current) => ({
                      ...current,
                      [id]: event.target.value,
                    }))
                  }
                  aria-describedby={`nd-multimod-probe-${id}-help`}
                />
                <small id={`nd-multimod-probe-${id}-help`}>
                  Comma-separated {rawSelected ? "raw observed" : "standardized"} values; up to five.
                </small>
              </label>
              <label htmlFor={`nd-multimod-probe-scale-${id}`}>
                Probe metric
                <select
                  id={`nd-multimod-probe-scale-${id}`}
                  value={rawSelected ? "raw" : "standardized"}
                  onChange={(event) => {
                    const raw = event.target.value === "raw";
                    setRawProbeModerators((current) => raw
                      ? unique([...current, id])
                      : current.filter((value) => value !== id));
                    setRawProbeReceipts((current) => {
                      const next = { ...current };
                      delete next[id];
                      return next;
                    });
                  }}
                >
                  <option value="standardized">Standardized score</option>
                  <option value="raw" disabled={!rawEligible}>Raw observed units</option>
                </select>
                {!rawEligible ? <small data-multimod-blocker="multimod.conditional.raw_probe_observed_single_indicator_required">
                  Raw units require an observed continuous or binary single-indicator moderator.
                </small> : null}
              </label>
              {rawSelected ? <>
                <label htmlFor={`nd-multimod-probe-orientation-${id}`}>
                  Fitted-score orientation
                  <select
                    id={`nd-multimod-probe-orientation-${id}`}
                    value={rawProbeOrientations[id] ?? 1}
                    onChange={(event) => {
                      const orientation = Number(event.target.value) as -1 | 1;
                      setRawProbeOrientations((current) => ({ ...current, [id]: orientation }));
                      setRawProbeReceipts((current) => {
                        const next = { ...current };
                        delete next[id];
                        return next;
                      });
                    }}
                  >
                    <option value={1}>+1</option>
                    <option value={-1}>−1</option>
                  </select>
                </label>
                <button
                  type="button"
                  disabled={rawProbePending !== null || Boolean(props.operationPending) || !rawEligible}
                  onClick={() => void prepareRawMetric(id)}
                >
                  {rawProbePending === id ? "Preparing metric…" : receipts.length ? "Refresh exact metric" : "Prepare exact metric"}
                </button>
                {receipts.length ? <p role="status">
                  {receipts.length} dataset-bound fit metric{receipts.length === 1 ? "" : "s"} prepared · row mask <code>{receipts[0]!.analysis_row_mask_sha256.slice(0, 12)}</code> · {receipts.map((receipt) => receipt.fit_scope.kind === "analysis_fit" ? "analysis fit" : `group ${receipt.fit_scope.group_id}`).join(", ")}.
                </p> : <p className="nd-inline-warning" role="note" data-multimod-blocker="multimod.conditional.raw_probe_receipt_missing">
                  <AlertTriangle size={15} aria-hidden="true" />
                  Calculate remains blocked until native preflight returns the exact row-mask, mass, center, scale, orientation, and fit-scope receipts.
                </p>}
              </> : null}
            </div>;
          })}
          {!props.onPrepareRawProbeMetrics ? <p
            className="nd-inline-warning"
            role="note"
            data-multimod-blocker="multimod.conditional.raw_probe_authority_unavailable"
          >
            <AlertTriangle size={15} aria-hidden="true" />
            Raw-unit probes are unavailable because the native fit-metric authority is not connected.
          </p> : null}
          {rawProbeFailure ? <p className="nd-form-error" role="alert">{rawProbeFailure}</p> : null}
        </fieldset>
      ) : null}
      {moderatorIds.length ? (
        <div className="nd-multimod-grid">
          <label htmlFor="nd-multimod-joint-tuples">
            Explicit joint tuples
            <textarea
              id="nd-multimod-joint-tuples"
              rows={3}
              value={jointTupleLines}
              onChange={(event) => setJointTupleLines(event.target.value)}
              placeholder="high-low: w=1, v=-1"
            />
            <small>
              Optional, one stable tuple id per line. When present, tuples
              replace the Cartesian grid.
            </small>
          </label>
          <label htmlFor="nd-multimod-probe-contrasts">
            Finite probe contrasts
            <textarea
              id="nd-multimod-probe-contrasts"
              rows={3}
              value={probeContrastLines}
              onChange={(event) => setProbeContrastLines(event.target.value)}
              placeholder="contrast-1: high-low > low-high"
            />
            <small>
              Optional, one stable contrast id and two declared tuple ids per
              line.
            </small>
          </label>
        </div>
      ) : null}
      {profile === "multiple_hoc_percentile" ? (
        <fieldset className="nd-multimod-option-list">
          <legend>HOCs whose dependency stages precede products</legend>
          {props.inventory.hocIds.map((id) => (
            <label key={id}>
              <input
                type="checkbox"
                checked={selectedHocs.includes(id)}
                onChange={(event) =>
                  setSelectedHocs((current) =>
                    event.target.checked
                      ? [...current, id]
                      : current.filter((value) => value !== id),
                  )
                }
              />
              <span>{id}</span>
            </label>
          ))}
        </fieldset>
      ) : null}
      {profile === "grouped_percentile" ? (
        <fieldset>
          <legend>Stratified groups</legend>
          <label htmlFor="nd-multimod-conditional-group-column">
            Grouping column
            <select
              id="nd-multimod-conditional-group-column"
              value={groupColumn}
              onChange={(event) => {
                setGroupColumn(event.target.value);
                setGroupIds([]);
                setGroupContrastLines("");
              }}
            >
              {props.columns
                .filter((column) => !column.usedAsIndicator)
                .map((column) => (
                  <option key={column.column} value={column.column}>
                    {column.label}
                  </option>
                ))}
            </select>
          </label>
          <div className="nd-multimod-option-list">
            {(activeGroupColumn?.groups ?? []).map((group) => (
              <label key={group.groupId}>
                <input
                  type="checkbox"
                  checked={groupIds.includes(group.groupId)}
                  onChange={(event) =>
                    setGroupIds((current) =>
                      event.target.checked
                        ? [...current, group.groupId]
                        : current.filter((id) => id !== group.groupId),
                    )
                  }
                />
                <span>{group.label}</span>
              </label>
            ))}
          </div>
          <label htmlFor="nd-multimod-group-contrasts">
            Explicit group contrasts
            <textarea
              id="nd-multimod-group-contrasts"
              rows={3}
              value={groupContrastLines}
              onChange={(event) => setGroupContrastLines(event.target.value)}
              placeholder="public-private: public > private"
            />
            <small>Optional; at most 20 stable, directed group pairs.</small>
          </label>
        </fieldset>
      ) : null}
      {profile === "case_weighted_percentile" ||
      profile === "frequency_weighted_percentile" ? (
        <label htmlFor="nd-multimod-conditional-weight">
          {profile === "case_weighted_percentile"
            ? "Positive case-weight column"
            : "Positive integer frequency column"}
          <input
            id="nd-multimod-conditional-weight"
            value={weightColumn}
            onChange={(event) => setWeightColumn(event.target.value)}
          />
        </label>
      ) : null}
      <fieldset className="nd-multimod-option-list">
        <legend>Requested estimands</legend>
        {(
          [
            [
              "conditional_specific_indirect",
              "Conditional specific indirect effects",
            ],
            [
              "conditional_total_indirect",
              "Conditional total indirect effects",
            ],
            ["conditional_total_effect", "Conditional total effects"],
            [
              "scalar_index_when_affine",
              "Scalar index only when affine in one moderator",
            ],
            ["local_first_derivatives", "Local first derivatives"],
            [
              "local_second_and_cross_derivatives",
              "Local second and cross derivatives",
            ],
            ["finite_probe_contrasts", "Finite probe contrasts"],
          ] as const
        ).map(([field, label]) => (
          <label key={field}>
            <input
              type="checkbox"
              checked={requestedEstimands[field]}
              disabled={
                field === "finite_probe_contrasts" && probeContrasts.length > 0
              }
              onChange={(event) =>
                setEstimands((current) => ({
                  ...current,
                  [field]: event.target.checked,
                }))
              }
            />
            <span>{label}</span>
          </label>
        ))}
      </fieldset>
      <fieldset className="nd-multimod-inline-fields">
        <legend>Inference ledger</legend>
        <label htmlFor="nd-multimod-conditional-outer">
          Outer resamples
          <input
            id="nd-multimod-conditional-outer"
            type="number"
            min={500}
            max={profile === "multi_two_way_studentized" ? 5_000 : 10_000}
            step={100}
            value={outerResamples}
            onChange={(event) => setOuterResamples(Number(event.target.value))}
          />
        </label>
        {profile === "multi_two_way_studentized" ? (
          <label htmlFor="nd-multimod-conditional-inner">
            Inner resamples
            <input
              id="nd-multimod-conditional-inner"
              type="number"
              min={100}
              max={1_000}
              step={50}
              value={innerResamples}
              onChange={(event) =>
                setInnerResamples(Number(event.target.value))
              }
            />
          </label>
        ) : null}
        <label htmlFor="nd-multimod-conditional-alternative">
          Alternative
          <select
            id="nd-multimod-conditional-alternative"
            value={alternative}
            disabled={[
              "multiple_hoc_percentile",
              "grouped_percentile",
              "case_weighted_percentile",
              "frequency_weighted_percentile",
            ].includes(profile)}
            onChange={(event) =>
              setAlternative(event.target.value as typeof alternative)
            }
          >
            <option value="two_sided">Two-sided</option>
            <option value="less">Less (predeclared)</option>
            <option value="greater">Greater (predeclared)</option>
          </select>
        </label>
      </fieldset>
      <dl className="nd-property-list">
        <div>
          <dt>Conditional cells</dt>
          <dd>{conditionalCells}</dd>
        </div>
        <div>
          <dt>Inferential targets</dt>
          <dd>{targetEstimate}</dd>
        </div>
        <div>
          <dt>Interval</dt>
          <dd>
            {interval === "bca"
              ? "Full delete-one BCa"
              : interval === "studentized"
                ? "Nested studentized"
                : "Type-7 percentile"}
          </dd>
        </div>
        <div>
          <dt>Ledger</dt>
          <dd>One shared replicate ledger for every target</dd>
        </div>
      </dl>
      <p className="nd-inline-warning" role="note">
        <AlertTriangle size={15} aria-hidden="true" />
        QuickPLS reports a scalar index only when the selected indirect effect
        is affine in one moderator. Multiple, both-stage, and three-way cases
        use derivatives and finite contrasts instead. No causal interpretation
        is added.
      </p>
      <CostAndActions
        request={request}
        error={assessment.error}
        predictedBytes={predictMultiModSidecarBytesV1({
          kind: "conditional",
          rows: props.caseCount,
          outerResamples,
          innerResamples:
            profile === "multi_two_way_studentized" ? innerResamples : 0,
          targets: targetEstimate,
          profile,
        })}
        runnerAvailability={props.runnerAvailability}
        operationPending={Boolean(props.operationPending)}
        onAssessRuntime={props.onAssessRuntime}
        onStageRecipe={props.onStageRecipe}
        onExecute={props.onExecute}
      />
    </div>
  );
}

interface CausalPathDraftV1 {
  readonly pathId: string;
  readonly variables: readonly string[];
}

function parseCausalPathDrafts(value: string): CausalPathDraftV1[] {
  return value
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line, index) => {
      const separator = line.indexOf("=");
      const pathId =
        separator >= 0 ? line.slice(0, separator).trim() : `path:${index + 1}`;
      const variables = (separator >= 0 ? line.slice(separator + 1) : line)
        .split(">")
        .map((entry) => entry.trim())
        .filter(Boolean);
      return { pathId, variables };
    });
}

function causalEquationKey(pathId: string, outcome: string): string {
  return `${pathId}\u0000${outcome}`;
}

function causalAllowedMainFactors(
  path: CausalPathDraftV1,
  equationIndex: number,
  adjustment: readonly string[],
  moderators: readonly string[],
): string[] {
  return unique([
    ...path.variables.slice(0, equationIndex + 1),
    ...adjustment,
    ...moderators,
  ]);
}

function causalRequiredMainFactors(
  path: CausalPathDraftV1,
  equationIndex: number,
  adjustment: readonly string[],
): string[] {
  return unique(
    [path.variables[equationIndex], ...adjustment].filter(
      (value): value is string => Boolean(value),
    ),
  );
}

function causalPairwiseCandidates(
  mainFactors: readonly string[],
  mediators: readonly string[],
): string[][] {
  const pairs: string[][] = [];
  for (let left = 0; left < mainFactors.length; left += 1) {
    for (let right = left + 1; right < mainFactors.length; right += 1) {
      const factors = [mainFactors[left], mainFactors[right]];
      if (factors.filter((factor) => mediators.includes(factor)).length <= 1)
        pairs.push(factors);
    }
  }
  return pairs;
}

function causalTermKey(factors: readonly string[]): string {
  return [...factors].sort().join("\u0000");
}

function causalTerm(factors: readonly string[]): CausalLinearTermV1 {
  return {
    term_id: `term:${[...factors].sort().join("*")}`,
    factor_variable_ids: [...factors],
  };
}

function buildCausalPaths(
  drafts: readonly CausalPathDraftV1[],
  equationTerms: Readonly<Record<string, readonly CausalLinearTermV1[]>>,
): ObservedCausalPathV1[] {
  return drafts.map((path) => ({
    path_id: path.pathId,
    ordered_variable_ids: [...path.variables],
    equations: path.variables.slice(1).map((outcome, equationIndex) => ({
      equation_id: `equation:${path.pathId}:${outcome}`,
      outcome_variable_id: outcome,
      terms: [
        ...(equationTerms[causalEquationKey(path.pathId, outcome)] ?? []),
      ],
    })),
  }));
}

function CausalMediationFlow(
  props: NativeMultiModLabsWorkspaceProps & {
    inventory: NativeMultiModModelInventoryV1;
  },
) {
  const observed = props.inventory.variables.filter(
    (variable) =>
      variable.observed &&
      variable.scale !== "identifier" &&
      variable.scale !== "ordinal" &&
      variable.scale !== "nominal",
  );
  const [treatment, setTreatment] = useState("");
  const [outcome, setOutcome] = useState("");
  const [mediators, setMediators] = useState<string[]>([]);
  const [moderators, setModerators] = useState<string[]>([]);
  const [adjustment, setAdjustment] = useState<string[]>([]);
  const [contrastKind, setContrastKind] = useState<"binary" | "continuous">(
    "binary",
  );
  const [x0, setX0] = useState(0);
  const [x1, setX1] = useState(1);
  const [pathLines, setPathLines] = useState("");
  const pathDrafts = useMemo(
    () => parseCausalPathDrafts(pathLines),
    [pathLines],
  );
  const [equationTerms, setEquationTerms] = useState<
    Record<string, CausalLinearTermV1[]>
  >({});
  const [checklist, setChecklist] = useState<CausalIdentificationChecklistV1>(
    EMPTY_CAUSAL_CHECKLIST,
  );
  const [bootstrap, setBootstrap] = useState(1_000);
  useEffect(() => {
    setEquationTerms((current) => {
      const next: Record<string, CausalLinearTermV1[]> = {};
      pathDrafts.forEach((path) =>
        path.variables.slice(1).forEach((equationOutcome, equationIndex) => {
          const key = causalEquationKey(path.pathId, equationOutcome);
          const allowedMains = causalAllowedMainFactors(
            path,
            equationIndex,
            adjustment,
            moderators,
          );
          const requiredMains = causalRequiredMainFactors(
            path,
            equationIndex,
            adjustment,
          );
          const allowedMainSet = new Set(allowedMains);
          const allowedPairSet = new Set(
            causalPairwiseCandidates(allowedMains, mediators).map(
              causalTermKey,
            ),
          );
          const retained = (current[key] ?? []).filter((term) =>
            term.factor_variable_ids.length === 1
              ? allowedMainSet.has(term.factor_variable_ids[0])
              : term.factor_variable_ids.length === 2 &&
                allowedPairSet.has(causalTermKey(term.factor_variable_ids)),
          );
          const retainedKeys = new Set(
            retained.map((term) => causalTermKey(term.factor_variable_ids)),
          );
          const defaultMains = current[key] ? requiredMains : allowedMains;
          defaultMains.forEach((factor) => {
            if (!retainedKeys.has(factor)) retained.push(causalTerm([factor]));
          });
          next[key] = retained;
        }),
      );
      return next;
    });
  }, [adjustment, mediators, moderators, pathDrafts]);
  const configuredPaths = useMemo(
    () => buildCausalPaths(pathDrafts, equationTerms),
    [equationTerms, pathDrafts],
  );
  const assessment = contractAssessment(() =>
    parseInterventionalCausalMediationConfigV1({
      schema_version: 1,
      treatment,
      treatment_contrast:
        contrastKind === "binary"
          ? { kind: "binary", control: x0, treated: x1 }
          : { kind: "continuous", x0, x1 },
      outcome,
      mediators,
      baseline_moderators: moderators,
      adjustment_covariates: adjustment,
      paths: configuredPaths,
      positivity_policy: {
        minimum_binary_arm_count: 10,
        maximum_binary_arm_ratio: 10,
        positivity_strata_variable_ids: [],
        minimum_count_per_binary_stratum_arm: 1,
        continuous_neighborhood_fraction_of_range: 0.05,
        minimum_continuous_neighborhood_count: 5,
      },
      identification: checklist,
      bootstrap_resamples: bootstrap,
      seed: 42,
      confidence_level: 0.95,
    }),
  );
  const request = assessment.value
    ? ({
        kind: "interventional_causal_mediation_v1",
        config: assessment.value,
      } as const)
    : null;
  const unavailable = new Set(
    [treatment, outcome, ...mediators, ...moderators, ...adjustment].filter(
      Boolean,
    ),
  );
  const treatmentCandidates = observed.filter((variable) =>
    contrastKind === "binary"
      ? variable.scale === "binary"
      : variable.scale === "continuous",
  );
  const outcomeCandidates = observed.filter(
    (variable) => variable.scale === "continuous",
  );
  const toggleRole = (
    id: string,
    values: string[],
    setValues: (next: string[]) => void,
  ) =>
    setValues(
      values.includes(id)
        ? values.filter((value) => value !== id)
        : [...values, id],
    );
  const toggleEquationTerm = (
    key: string,
    factors: readonly string[],
    required: boolean,
  ) => {
    if (required) return;
    const selectedKey = causalTermKey(factors);
    setEquationTerms((current) => {
      const present = (current[key] ?? []).some(
        (term) => causalTermKey(term.factor_variable_ids) === selectedKey,
      );
      return {
        ...current,
        [key]: present
          ? (current[key] ?? []).filter(
              (term) =>
                causalTermKey(term.factor_variable_ids) !== selectedKey &&
                !(
                  factors.length === 1 &&
                  term.factor_variable_ids.length > 1 &&
                  term.factor_variable_ids.includes(factors[0])
                ),
            )
          : [...(current[key] ?? []), causalTerm(factors)],
      };
    });
  };
  return (
    <div className="nd-multimod-flow">
      <p className="nd-multimod-causal-label" role="note">
        <ShieldCheck size={16} aria-hidden="true" />
        <strong>Separate observed-data workflow.</strong> Results are labelled
        “assumption-dependent interventional estimate”; QuickPLS never says
        causality is established.
      </p>
      <div className="nd-multimod-grid">
        <fieldset>
          <legend>Observed treatment and outcome</legend>
          <label htmlFor="nd-multimod-causal-treatment">
            Treatment
            <select
              id="nd-multimod-causal-treatment"
              value={treatment}
              onChange={(event) => setTreatment(event.target.value)}
            >
              <option value="">Choose explicitly</option>
              {treatmentCandidates
                .filter(
                  (variable) =>
                    variable.id === treatment || !unavailable.has(variable.id),
                )
                .map((variable) => (
                  <option key={variable.id} value={variable.id}>
                    {variable.label} [{variable.id}]
                  </option>
                ))}
            </select>
          </label>
          <label htmlFor="nd-multimod-causal-outcome">
            Continuous outcome
            <select
              id="nd-multimod-causal-outcome"
              value={outcome}
              onChange={(event) => setOutcome(event.target.value)}
            >
              <option value="">Choose explicitly</option>
              {outcomeCandidates
                .filter(
                  (variable) =>
                    variable.id === outcome || !unavailable.has(variable.id),
                )
                .map((variable) => (
                  <option key={variable.id} value={variable.id}>
                    {variable.label} [{variable.id}]
                  </option>
                ))}
            </select>
          </label>
          <label htmlFor="nd-multimod-causal-kind">
            Treatment contrast
            <select
              id="nd-multimod-causal-kind"
              value={contrastKind}
              onChange={(event) => {
                setContrastKind(event.target.value as typeof contrastKind);
                setTreatment("");
              }}
            >
              <option value="binary">Binary control → treated</option>
              <option value="continuous">Continuous x0 → x1</option>
            </select>
          </label>
          <div className="nd-multimod-inline-fields">
            <label htmlFor="nd-multimod-causal-x0">
              {contrastKind === "binary" ? "Control" : "x0"}
              <input
                id="nd-multimod-causal-x0"
                type="number"
                value={x0}
                onChange={(event) => setX0(Number(event.target.value))}
              />
            </label>
            <label htmlFor="nd-multimod-causal-x1">
              {contrastKind === "binary" ? "Treated" : "x1"}
              <input
                id="nd-multimod-causal-x1"
                type="number"
                value={x1}
                onChange={(event) => setX1(Number(event.target.value))}
              />
            </label>
          </div>
        </fieldset>
        <fieldset className="nd-multimod-option-list">
          <legend>Observed continuous mediators</legend>
          {observed
            .filter((variable) => variable.scale === "continuous")
            .map((variable) => (
              <label
                key={variable.id}
                className={
                  unavailable.has(variable.id) &&
                  !mediators.includes(variable.id)
                    ? "disabled"
                    : undefined
                }
              >
                <input
                  type="checkbox"
                  disabled={
                    unavailable.has(variable.id) &&
                    !mediators.includes(variable.id)
                  }
                  checked={mediators.includes(variable.id)}
                  onChange={() =>
                    toggleRole(variable.id, mediators, setMediators)
                  }
                />
                <span>
                  {variable.label} [{variable.id}]
                </span>
              </label>
            ))}
        </fieldset>
      </div>
      <div className="nd-multimod-grid">
        <fieldset className="nd-multimod-option-list">
          <legend>Baseline numeric/binary moderators</legend>
          {observed.map((variable) => (
            <label
              key={variable.id}
              className={
                unavailable.has(variable.id) &&
                !moderators.includes(variable.id)
                  ? "disabled"
                  : undefined
              }
            >
              <input
                type="checkbox"
                disabled={
                  unavailable.has(variable.id) &&
                  !moderators.includes(variable.id)
                }
                checked={moderators.includes(variable.id)}
                onChange={() =>
                  toggleRole(variable.id, moderators, setModerators)
                }
              />
              <span>
                {variable.label} [{variable.id}]
              </span>
            </label>
          ))}
        </fieldset>
        <fieldset className="nd-multimod-option-list">
          <legend>Required nonempty adjustment set</legend>
          {observed.map((variable) => (
            <label
              key={variable.id}
              className={
                unavailable.has(variable.id) &&
                !adjustment.includes(variable.id)
                  ? "disabled"
                  : undefined
              }
            >
              <input
                type="checkbox"
                disabled={
                  unavailable.has(variable.id) &&
                  !adjustment.includes(variable.id)
                }
                checked={adjustment.includes(variable.id)}
                onChange={() =>
                  toggleRole(variable.id, adjustment, setAdjustment)
                }
              />
              <span>
                {variable.label} [{variable.id}]
              </span>
            </label>
          ))}
          {!adjustment.length ? (
            <p className="nd-form-error" role="alert">
              Select at least one explicit adjustment covariate; QuickPLS does
              not infer an empty adjustment set.{" "}
              <code>
                interventional_causal_mediation_v1.adjustment_set_missing
              </code>
            </p>
          ) : null}
        </fieldset>
      </div>
      <label htmlFor="nd-multimod-causal-paths">
        Explicit causal paths
        <textarea
          id="nd-multimod-causal-paths"
          rows={4}
          value={pathLines}
          onChange={(event) => setPathLines(event.target.value)}
          placeholder="path:treatment-mediator-outcome = treatment_id > mediator_id > outcome_id"
          aria-describedby="nd-multimod-causal-paths-help"
        />
        <small id="nd-multimod-causal-paths-help">
          One path per line. Put the stable path id before = and list 2–4
          directed edges with &gt;. QuickPLS will not infer an indirect path or
          hide its equation terms.
        </small>
      </label>
      <section
        className="nd-multimod-equation-editor"
        aria-labelledby="nd-multimod-causal-equations-heading"
      >
        <h3 id="nd-multimod-causal-equations-heading">
          Explicit observed-data equations
        </h3>
        <p>
          Every equation includes a fitted intercept. Main effects and allowed
          pairwise products are shown below exactly as they will be staged.
          Required predecessor and adjustment terms cannot be removed.
        </p>
        {!pathDrafts.length ? (
          <p role="note">
            Enter at least one causal path to define its ordered equations.
          </p>
        ) : null}
        {pathDrafts.flatMap((path) =>
          path.variables.slice(1).map((equationOutcome, equationIndex) => {
            const key = causalEquationKey(path.pathId, equationOutcome);
            const allowedMains = causalAllowedMainFactors(
              path,
              equationIndex,
              adjustment,
              moderators,
            );
            const requiredMains = new Set(
              causalRequiredMainFactors(path, equationIndex, adjustment),
            );
            const selected = new Set(
              (equationTerms[key] ?? []).map((term) =>
                causalTermKey(term.factor_variable_ids),
              ),
            );
            const pairs = causalPairwiseCandidates(allowedMains, mediators);
            return (
              <fieldset key={key} className="nd-multimod-equation-card">
                <legend>
                  {path.pathId}: outcome{" "}
                  <code>{equationOutcome || "(missing)"}</code>
                </legend>
                <label className="nd-multimod-required-term">
                  <input type="checkbox" checked disabled />
                  <span>Intercept (required by the V1 linear equation)</span>
                </label>
                <div
                  className="nd-multimod-equation-terms"
                  role="group"
                  aria-label={`Main effects for ${equationOutcome}`}
                >
                  <strong>Main effects</strong>
                  {allowedMains.map((factor) => (
                    <label key={`${key}:main:${factor}`}>
                      <input
                        type="checkbox"
                        checked={selected.has(factor)}
                        disabled={requiredMains.has(factor)}
                        onChange={() =>
                          toggleEquationTerm(
                            key,
                            [factor],
                            requiredMains.has(factor),
                          )
                        }
                      />
                      <span>
                        <code>{factor}</code>
                        {requiredMains.has(factor) ? " (required)" : ""}
                      </span>
                    </label>
                  ))}
                </div>
                <div
                  className="nd-multimod-equation-terms"
                  role="group"
                  aria-label={`Pairwise terms for ${equationOutcome}`}
                >
                  <strong>Allowed pairwise terms</strong>
                  {!pairs.length ? (
                    <span>No recursive pairwise term is available.</span>
                  ) : (
                    pairs.map((factors) => {
                      const pairKey = causalTermKey(factors);
                      const hierarchyReady = factors.every((factor) =>
                        selected.has(factor),
                      );
                      return (
                        <label
                          key={`${key}:pair:${pairKey}`}
                          className={!hierarchyReady ? "disabled" : undefined}
                        >
                          <input
                            type="checkbox"
                            checked={selected.has(pairKey)}
                            disabled={!hierarchyReady}
                            onChange={() =>
                              toggleEquationTerm(key, factors, false)
                            }
                          />
                          <span>
                            <code>{factors.join(" × ")}</code>
                            {!hierarchyReady
                              ? " (select both main effects first)"
                              : ""}
                          </span>
                        </label>
                      );
                    })
                  )}
                </div>
                <output aria-live="polite">
                  <strong>Staged equation:</strong>{" "}
                  <code>
                    {equationOutcome || "?"} = intercept +{" "}
                    {(equationTerms[key] ?? [])
                      .map((term) => term.factor_variable_ids.join(" × "))
                      .join(" + ") || "(no predictors selected)"}
                  </code>
                </output>
              </fieldset>
            );
          }),
        )}
      </section>
      <fieldset className="nd-multimod-checklist">
        <legend>Identification and positivity declarations</legend>
        {(
          Object.keys(CAUSAL_LABELS) as Array<
            keyof CausalIdentificationChecklistV1
          >
        ).map((field) => (
          <label key={field} htmlFor={`nd-multimod-causal-${field}`}>
            <input
              id={`nd-multimod-causal-${field}`}
              type="checkbox"
              checked={checklist[field]}
              onChange={(event) =>
                setChecklist((current) => ({
                  ...current,
                  [field]: event.target.checked,
                }))
              }
            />
            <span>{CAUSAL_LABELS[field]}</span>
          </label>
        ))}
      </fieldset>
      <label htmlFor="nd-multimod-causal-bootstrap">
        Bootstrap replicates
        <input
          id="nd-multimod-causal-bootstrap"
          type="number"
          min={500}
          max={10_000}
          step={100}
          value={bootstrap}
          onChange={(event) => setBootstrap(Number(event.target.value))}
        />
      </label>
      <CostAndActions
        request={request}
        error={assessment.error}
        predictedBytes={predictMultiModSidecarBytesV1({
          kind: "causal",
          rows: props.caseCount,
          resamples: bootstrap,
          targets: Math.max(configuredPaths.length, 1) * 3,
        })}
        runnerAvailability={props.runnerAvailability}
        operationPending={Boolean(props.operationPending)}
        onAssessRuntime={props.onAssessRuntime}
        onStageRecipe={props.onStageRecipe}
        onExecute={props.onExecute}
      />
    </div>
  );
}

export type MultiModTabV1 = "mga" | "segmentation" | "conditional" | "causal";

const TABS: readonly {
  id: MultiModTabV1;
  label: string;
  description: string;
}[] = [
  {
    id: "mga",
    label: "Categorical moderation",
    description: "MICOM and 2–20-group MGA",
  },
  {
    id: "segmentation",
    label: "Latent segmentation",
    description: "FIMIX-PLS and PLS-POS V2",
  },
  {
    id: "conditional",
    label: "Conditional process",
    description: "Broader moderated mediation",
  },
  {
    id: "causal",
    label: "Interventional mediation",
    description: "Observed-data g-computation",
  },
];

export function NativeMultiModLabsWorkspace(
  props: NativeMultiModLabsWorkspaceProps,
) {
  const [activeTab, setActiveTab] = useState<MultiModTabV1>(
    props.initialTab ?? "mga",
  );
  const inventory = useMemo(
    () => nativeMultiModModelInventoryV1(props.model),
    [props.model],
  );
  const columns = props.groupingColumns ?? fallbackGroupingColumns(props.model);
  const activateWithKeyboard = (event: KeyboardEvent<HTMLDivElement>) => {
    if (
      event.key !== "ArrowLeft" &&
      event.key !== "ArrowRight" &&
      event.key !== "Home" &&
      event.key !== "End"
    )
      return;
    event.preventDefault();
    const current = TABS.findIndex((tab) => tab.id === activeTab);
    const next =
      event.key === "Home"
        ? 0
        : event.key === "End"
          ? TABS.length - 1
          : (current + (event.key === "ArrowRight" ? 1 : -1) + TABS.length) %
            TABS.length;
    setActiveTab(TABS[next].id);
    document.getElementById(`nd-multimod-tab-${TABS[next].id}`)?.focus();
  };
  return (
    <section
      className="nd-cbsem-v4-workspace nd-multimod-workspace"
      aria-labelledby="nd-multimod-heading"
    >
      <header className="nd-cbsem-v4-header">
        <div>
          <span className="nd-multimod-labs-badge">
            Experimental Labs · Unqualified
          </span>
          <h2 id="nd-multimod-heading">Moderation and heterogeneity suite</h2>
          <p>
            Additive Recipe V4 contracts · offline-only execution boundary ·
            continuous moderation V1 remains unchanged
          </p>
        </div>
        <FlaskConical size={24} aria-hidden="true" />
      </header>
      {props.validatedResult ? (
        <NativeMultiModResultsV1
          validatedResult={props.validatedResult}
          canonicalDocument={props.canonicalResultDocument}
          rawSidecarExportAuthority={props.rawSidecarExportAuthority}
        />
      ) : null}
      <div
        className="nd-multimod-tabs"
        role="tablist"
        aria-label="MultiMod analysis family"
        onKeyDown={activateWithKeyboard}
      >
        {TABS.map((tab) => (
          <button
            key={tab.id}
            id={`nd-multimod-tab-${tab.id}`}
            type="button"
            role="tab"
            aria-selected={activeTab === tab.id}
            aria-controls={`nd-multimod-panel-${tab.id}`}
            tabIndex={activeTab === tab.id ? 0 : -1}
            onClick={() => setActiveTab(tab.id)}
          >
            <strong>{tab.label}</strong>
            <span>{tab.description}</span>
          </button>
        ))}
      </div>
      <section
        id={`nd-multimod-panel-${activeTab}`}
        role="tabpanel"
        tabIndex={0}
        aria-labelledby={`nd-multimod-tab-${activeTab}`}
      >
        {activeTab === "mga" ? (
          <MgaFlow {...props} columns={columns} inventory={inventory} />
        ) : null}
        {activeTab === "segmentation" ? (
          <HeterogeneityFlow {...props} inventory={inventory} />
        ) : null}
        {activeTab === "conditional" ? (
          <ConditionalProcessFlow
            {...props}
            columns={columns}
            inventory={inventory}
          />
        ) : null}
        {activeTab === "causal" ? (
          <CausalMediationFlow {...props} inventory={inventory} />
        ) : null}
      </section>
    </section>
  );
}

export default NativeMultiModLabsWorkspace;
