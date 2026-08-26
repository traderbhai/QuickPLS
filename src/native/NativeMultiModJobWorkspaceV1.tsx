import {
  AlertTriangle,
  ArchiveRestore,
  CircleStop,
  RotateCcw,
  X,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  ConditionalRawProbeFitMetricReceiptV2,
  GeneralSemConditionalProcessConfigV2,
  MultiModRecipeConfigV1,
  MultiModResultAttachmentV1,
} from "../domain/multimodContractsV1";
import type { CanonicalResultDocumentV2 } from "../domain/canonicalResultDocumentV2";
import type { SemModelV4 } from "../domain/semModelV4";
import {
  NativeMultiModLabsWorkspace,
  type NativeMultiModGroupingColumnV1,
  type NativeMultiModRunnerAvailabilityV1,
  type MultiModTabV1,
} from "./NativeMultiModLabsWorkspace";
import {
  cancelNativeMultiModJobV1,
  dismissNativeMultiModJobV1,
  getNativeMultiModJobResultV1,
  getNativeMultiModJobV1,
  preflightNativeMultiModJobV1,
  prepareNativeConditionalRawProbeMetricsV2,
  profileNativeMultiModGroupingV1,
  resumeNativeMultiModRequestV1,
  stageNativeMultiModRequestV1,
  startNativeMultiModJobV1,
  type NativeMultiModArchiveAuthorityV1,
  type NativeMultiModAccessV1,
  type NativeMultiModCompletedResultV1,
  type NativeMultiModJobSnapshotV1,
  type NativeMultiModPreflightV1,
  type NativeMultiModStagedRequestV1,
} from "./nativeMultiModJobV1";

const TERMINAL_STATES = new Set(["completed", "failed", "cancelled"] as const);

interface StagedNativeRequestV1 {
  readonly requestKey: string;
  readonly staged: NativeMultiModStagedRequestV1;
  readonly preflight: NativeMultiModPreflightV1;
}

export interface NativeMultiModJobWorkspaceV1Props {
  readonly authority: NativeMultiModArchiveAuthorityV1;
  readonly access: NativeMultiModAccessV1;
  readonly model: SemModelV4;
  readonly caseCount: number;
  readonly groupingColumns?: readonly NativeMultiModGroupingColumnV1[];
  readonly initialTab?: MultiModTabV1;
  /** Every attachment/canonical pair obtained from the same strict Archive V6 reopen. */
  readonly residentResults?: readonly {
    readonly attachment: MultiModResultAttachmentV1;
    readonly canonicalDocument: CanonicalResultDocumentV2;
  }[];
  readonly onArchiveUpdated?: (
    result: NativeMultiModCompletedResultV1,
  ) => void | Promise<void>;
}

function messageFromUnknown(reason: unknown): string {
  if (reason instanceof Error) return reason.message;
  if (reason && typeof reason === "object") {
    const candidate = reason as {
      message?: unknown;
      correctiveAction?: unknown;
      code?: unknown;
    };
    const message =
      typeof candidate.message === "string"
        ? candidate.message
        : "The native MultiMod operation was rejected.";
    const code =
      typeof candidate.code === "string" ? ` [${candidate.code}]` : "";
    const corrective =
      typeof candidate.correctiveAction === "string"
        ? ` ${candidate.correctiveAction}`
        : "";
    return `${message}${code}${corrective}`;
  }
  return String(reason);
}

function availabilityFromPreflight(
  preflight: NativeMultiModPreflightV1,
): NativeMultiModRunnerAvailabilityV1 {
  if (preflight.mgaGroupEligibility?.eligible === false) {
    return {
      state: "blocked",
      reason: `The exact native MGA group-eligibility gate failed: ${preflight.mgaGroupEligibility.blockerCodes.join(", ")}.`,
      mgaGroupEligibility: preflight.mgaGroupEligibility,
    };
  }
  if (preflight.readiness === "built_in_from_dataset") {
    return {
      state: "executable",
      capabilityCellId: preflight.capabilityCellId,
      mgaGroupEligibility: preflight.mgaGroupEligibility,
    };
  }
  const reasons = preflight.stableReasonCodes.length
    ? preflight.stableReasonCodes.join(", ")
    : "multimod.runtime.unavailable";
  return {
    state: "blocked",
    reason:
      preflight.readiness === "prepared_adapter_required"
        ? `This exact profile needs a prepared-data adapter that is not wired in the shipped offline runtime. ${reasons}`
        : `This exact profile is outside the built-in runtime envelope. ${reasons}`,
  };
}

function progressMaximum(snapshot: NativeMultiModJobSnapshotV1): number {
  return Math.max(snapshot.totalUnits, 1);
}

function progressValue(snapshot: NativeMultiModJobSnapshotV1): number {
  return Math.min(snapshot.completedUnits, progressMaximum(snapshot));
}

export function NativeMultiModJobWorkspaceV1({
  authority,
  access,
  model,
  caseCount,
  groupingColumns,
  initialTab,
  residentResults = [],
  onArchiveUpdated,
}: NativeMultiModJobWorkspaceV1Props) {
  const authorityKey = useMemo(() => JSON.stringify(authority), [authority]);
  const accessKey = `${access.surface}:${String(access.experimentalLabsEnabled)}`;
  const incomingAuthority = useRef(authority);
  incomingAuthority.current = authority;
  const effectiveAuthority = useRef(authority);
  const stagedByIdentity = useRef(new Map<string, StagedNativeRequestV1>());
  const lastStaged = useRef<StagedNativeRequestV1 | null>(null);
  const lifecycleGeneration = useRef(0);
  const [snapshot, setSnapshot] = useState<NativeMultiModJobSnapshotV1 | null>(
    null,
  );
  const [completed, setCompleted] =
    useState<NativeMultiModCompletedResultV1 | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [profiledGroupingColumns, setProfiledGroupingColumns] = useState<
    readonly NativeMultiModGroupingColumnV1[] | null
  >(null);
  const [groupingProfileNotice, setGroupingProfileNotice] = useState<
    string | null
  >(null);
  const [operationPending, setOperationPending] = useState(false);
  const [selectedResidentResultId, setSelectedResidentResultId] = useState(
    residentResults[0]?.attachment.result_id ?? "",
  );
  const selectedResidentResult = residentResults.find(
    (candidate) => candidate.attachment.result_id === selectedResidentResultId,
  ) ?? residentResults[0];

  useEffect(() => {
    lifecycleGeneration.current += 1;
    effectiveAuthority.current = incomingAuthority.current;
    stagedByIdentity.current.clear();
    lastStaged.current = null;
    setSnapshot(null);
    setCompleted(null);
    setNotice(null);
    setFailure(null);
    setProfiledGroupingColumns(null);
    setGroupingProfileNotice(null);
    setOperationPending(false);
    setSelectedResidentResultId(residentResults[0]?.attachment.result_id ?? "");
  }, [accessKey, authorityKey, residentResults]);

  useEffect(() => {
    let live = true;
    if (groupingColumns) {
      setProfiledGroupingColumns(groupingColumns);
      return () => {
        live = false;
      };
    }
    void profileNativeMultiModGroupingV1(incomingAuthority.current, access)
      .then((profile) => {
        if (!live) return;
        setProfiledGroupingColumns(profile.columns);
        if (profile.omittedHighCardinalityColumns.length) {
          setGroupingProfileNotice(
            `High-cardinality columns omitted from group selection: ${profile.omittedHighCardinalityColumns.join(", ")}.`,
          );
        }
      })
      .catch((reason) => {
        if (live)
          setGroupingProfileNotice(
            `Typed grouping discovery is unavailable: ${messageFromUnknown(reason)}`,
          );
      });
    return () => {
      live = false;
    };
  }, [access, authorityKey, groupingColumns]);

  useEffect(
    () => () => {
      lifecycleGeneration.current += 1;
    },
    [],
  );

  const stageAndPreflight = useCallback(
    async (request: MultiModRecipeConfigV1): Promise<StagedNativeRequestV1> => {
      const currentAuthority = effectiveAuthority.current;
      const requestKey = `${accessKey}\n${JSON.stringify(currentAuthority)}\n${JSON.stringify(request)}`;
      const cached = stagedByIdentity.current.get(requestKey);
      if (cached) return cached;
      const staged = stageNativeMultiModRequestV1(
        currentAuthority,
        request,
        access,
      );
      const preflight = await preflightNativeMultiModJobV1(staged);
      if (preflight.stagedRecipeId !== staged.stagedRecipeId) {
        throw new Error(
          "Native preflight returned a different staged Recipe V4 identity.",
        );
      }
      const value = { requestKey, staged, preflight };
      stagedByIdentity.current.set(requestKey, value);
      return value;
    },
    [access, accessKey, authorityKey],
  );

  const assessRuntime = useCallback(
    async (
      request: MultiModRecipeConfigV1,
    ): Promise<NativeMultiModRunnerAvailabilityV1> => {
      const staged = await stageAndPreflight(request);
      return availabilityFromPreflight(staged.preflight);
    },
    [stageAndPreflight],
  );

  const prepareRawProbeMetrics = useCallback(
    async (
      config: GeneralSemConditionalProcessConfigV2,
      moderatorId: string,
      orientationSign: -1 | 1,
    ): Promise<readonly ConditionalRawProbeFitMetricReceiptV2[]> => {
      const staged = await stageAndPreflight({
        kind: "general_sem_conditional_process_v2",
        config,
      });
      if (staged.preflight.readiness !== "built_in_from_dataset") {
        throw new Error(
          `Raw-probe preparation is fail-closed for this profile: ${staged.preflight.stableReasonCodes.join(", ") || staged.preflight.readiness}.`,
        );
      }
      return prepareNativeConditionalRawProbeMetricsV2(
        staged.staged,
        moderatorId,
        orientationSign,
      );
    },
    [stageAndPreflight],
  );

  const stageRecipe = useCallback(
    async (request: MultiModRecipeConfigV1) => {
      setFailure(null);
      const staged = await stageAndPreflight(request);
      lastStaged.current = staged;
      const availability = availabilityFromPreflight(staged.preflight);
      setNotice(
        availability.state === "executable"
          ? `Recipe V4 ${staged.preflight.stagedRecipeId} passed strict compilation and is ready for its built-in offline engine.`
          : `Recipe V4 ${staged.preflight.stagedRecipeId} is staged for review but remains fail-closed. ${availability.reason}`,
      );
    },
    [stageAndPreflight],
  );

  const monitor = useCallback(
    async (
      initial: NativeMultiModJobSnapshotV1,
      generation: number,
    ): Promise<NativeMultiModJobSnapshotV1> => {
      let current = initial;
      while (
        !TERMINAL_STATES.has(
          current.state as "completed" | "failed" | "cancelled",
        )
      ) {
        await new Promise((resolve) => globalThis.setTimeout(resolve, 350));
        if (lifecycleGeneration.current !== generation) return current;
        current = await getNativeMultiModJobV1(initial.jobId);
        if (lifecycleGeneration.current === generation) setSnapshot(current);
      }
      return current;
    },
    [],
  );

  const executeStaged = useCallback(
    async (staged: StagedNativeRequestV1, resume = false) => {
      if (staged.preflight.readiness !== "built_in_from_dataset") {
        throw new Error(
          `Native execution is fail-closed for this profile: ${staged.preflight.stableReasonCodes.join(", ") || staged.preflight.readiness}.`,
        );
      }
      lifecycleGeneration.current += 1;
      const generation = lifecycleGeneration.current;
      setOperationPending(true);
      setCompleted(null);
      setFailure(null);
      setNotice(
        resume
          ? staged.staged.resumeCache?.stage === "mga_execution"
            ? "Resuming MGA estimation from the validated completed-shard cache."
            : "Resuming publication from the verified archive-ready external cache."
          : "Submitting the exact preflighted Recipe V4 to the offline native engine.",
      );
      try {
        const initial = await startNativeMultiModJobV1(staged.staged);
        if (lifecycleGeneration.current !== generation) return;
        setSnapshot(initial);
        const terminal = await monitor(initial, generation);
        if (lifecycleGeneration.current !== generation) return;
        if (terminal.state === "completed") {
          const result = await getNativeMultiModJobResultV1(terminal.jobId);
          if (lifecycleGeneration.current !== generation) return;
          effectiveAuthority.current = {
            ...effectiveAuthority.current,
            archivePath: result.archivePath,
            archiveSha256: result.archiveSha256,
          };
          stagedByIdentity.current.clear();
          lastStaged.current = null;
          setCompleted(result);
          setNotice(
            `Completed result ${result.attachment.result_id} was atomically attached and strictly reopened from Archive V6.`,
          );
          await onArchiveUpdated?.(result);
        } else if (terminal.state === "cancelled") {
          setCompleted(null);
          setNotice(
            terminal.resumeCache
              ? terminal.resumeCache.stage === "mga_execution"
                ? "The job was cancelled without publishing a partial result. Its validated completed MGA shards can be resumed."
                : "The job was cancelled without publishing a partial result. Its archive-ready cache can be resumed after receipt verification."
              : "The job was cancelled without publishing a partial scientific result.",
          );
        } else {
          setCompleted(null);
          setFailure(
            terminal.failure
              ? `${terminal.failure.message} [${terminal.failure.code}] ${terminal.failure.correctiveAction}`
              : "The native MultiMod job failed without a typed failure payload.",
          );
        }
      } catch (reason) {
        if (lifecycleGeneration.current === generation) {
          setCompleted(null);
          setFailure(messageFromUnknown(reason));
        }
      } finally {
        if (lifecycleGeneration.current === generation)
          setOperationPending(false);
      }
    },
    [monitor, onArchiveUpdated],
  );

  const execute = useCallback(
    async (request: MultiModRecipeConfigV1) => {
      const staged = await stageAndPreflight(request);
      lastStaged.current = staged;
      await executeStaged(staged);
    },
    [executeStaged, stageAndPreflight],
  );

  const cancel = useCallback(async () => {
    if (
      !snapshot ||
      !["queued", "running", "cancelling"].includes(snapshot.state)
    )
      return;
    try {
      setSnapshot(await cancelNativeMultiModJobV1(snapshot.jobId));
      setNotice(
        "Cancellation requested. QuickPLS will not publish a partial scientific result.",
      );
    } catch (reason) {
      setFailure(messageFromUnknown(reason));
    }
  }, [snapshot]);

  const resume = useCallback(async () => {
    if (!snapshot?.resumeCache || !lastStaged.current) return;
    const resumedRequest = resumeNativeMultiModRequestV1(
      lastStaged.current.staged,
      snapshot.resumeCache,
    );
    await executeStaged(
      { ...lastStaged.current, staged: resumedRequest },
      true,
    );
  }, [executeStaged, snapshot]);

  const dismiss = useCallback(async () => {
    if (
      !snapshot ||
      !TERMINAL_STATES.has(
        snapshot.state as "completed" | "failed" | "cancelled",
      )
    )
      return;
    try {
      await dismissNativeMultiModJobV1(snapshot.jobId);
      lifecycleGeneration.current += 1;
      setSnapshot(null);
      setFailure(null);
      setNotice(null);
      setOperationPending(false);
    } catch (reason) {
      setFailure(messageFromUnknown(reason));
    }
  }, [snapshot]);

  const jobActive = Boolean(
    snapshot &&
    !TERMINAL_STATES.has(
      snapshot.state as "completed" | "failed" | "cancelled",
    ),
  );
  return (
    <div data-native-multimod-job-workspace="v1">
      {snapshot ? (
        <section
          className="nd-multimod-job-monitor"
          aria-labelledby="nd-multimod-job-heading"
        >
          <div>
            <h3 id="nd-multimod-job-heading">MultiMod native job</h3>
            <span className={`nd-cbsem-v4-state ${snapshot.state}`}>
              {snapshot.state}
            </span>
          </div>
          <progress
            max={progressMaximum(snapshot)}
            value={progressValue(snapshot)}
          >
            {progressValue(snapshot)} of {progressMaximum(snapshot)}
          </progress>
          <p aria-live="polite" aria-atomic="true">
            {snapshot.phase}: {snapshot.completedUnits} of {snapshot.totalUnits}
            . {snapshot.message ?? ""}
          </p>
          {snapshot.warningCodes.length ? (
            <p role="note">
              <AlertTriangle size={15} aria-hidden="true" />{" "}
              {snapshot.warningCodes.join(", ")}
            </p>
          ) : null}
          {snapshot.resumeCache ? (
            <p className="nd-multimod-job-cache" role="note">
              {snapshot.resumeCache.stage === "mga_execution"
                ? "MGA execution cache"
                : "Archive-ready cache"}{" "}
              <code>{snapshot.resumeCache.cacheId}</code> · manifest{" "}
              <code>{snapshot.resumeCache.manifestSha256}</code>
            </p>
          ) : null}
          {snapshot.target !== "mga_multigroup_v1" &&
          ["queued", "running", "cancelling"].includes(snapshot.state) ? (
            <p role="note">
              Estimation resume is not available for this family yet.
              Cancellation before archive-ready publication restarts the exact
              execution.
            </p>
          ) : null}
          <div className="nd-cbsem-v4-actions">
            {jobActive && snapshot.state !== "publishing" ? (
              <button
                type="button"
                className="danger"
                onClick={() => void cancel()}
              >
                <CircleStop size={15} aria-hidden="true" />
                Cancel safely
              </button>
            ) : null}
            {snapshot.state === "cancelled" && snapshot.resumeCache ? (
              <button
                type="button"
                className="primary"
                disabled={operationPending || !lastStaged.current}
                onClick={() => void resume()}
              >
                <ArchiveRestore size={15} aria-hidden="true" />
                {snapshot.resumeCache.stage === "mga_execution"
                  ? "Resume MGA shards"
                  : "Resume publication"}
              </button>
            ) : null}
            {snapshot.state === "failed" && snapshot.resumeCache ? (
              <button
                type="button"
                className="primary"
                disabled={operationPending || !lastStaged.current}
                onClick={() => void resume()}
              >
                <ArchiveRestore size={15} aria-hidden="true" />
                {snapshot.resumeCache.stage === "mga_execution"
                  ? "Resume MGA shards"
                  : "Resume publication"}
              </button>
            ) : snapshot.state === "failed" ? (
              <button
                type="button"
                disabled={operationPending || !lastStaged.current}
                onClick={() =>
                  lastStaged.current && void executeStaged(lastStaged.current)
                }
              >
                <RotateCcw size={15} aria-hidden="true" />
                Retry exact request
              </button>
            ) : null}
            {TERMINAL_STATES.has(
              snapshot.state as "completed" | "failed" | "cancelled",
            ) ? (
              <button
                type="button"
                disabled={operationPending}
                onClick={() => void dismiss()}
              >
                <X size={15} aria-hidden="true" />
                Dismiss job state
              </button>
            ) : null}
          </div>
        </section>
      ) : null}
      {failure ? (
        <div className="nd-form-error" role="alert">
          <strong>MultiMod operation blocked.</strong>
          <span>{failure}</span>
        </div>
      ) : null}
      {groupingProfileNotice ? (
        <p className="nd-inline-warning" role="note">
          <AlertTriangle size={15} aria-hidden="true" />
          {groupingProfileNotice}
        </p>
      ) : null}
      {notice ? (
        <p role="status" aria-live="polite" aria-atomic="true">
          {notice}
        </p>
      ) : null}
      {residentResults.length ? (
        <label htmlFor="nd-multimod-resident-result-v1">
          Saved MultiMod result
          <select
            id="nd-multimod-resident-result-v1"
            value={selectedResidentResult?.attachment.result_id ?? ""}
            disabled={operationPending || jobActive || Boolean(completed)}
            onChange={(event) => setSelectedResidentResultId(event.target.value)}
          >
            {residentResults.map(({ attachment, canonicalDocument }) => (
              <option key={attachment.result_id} value={attachment.result_id}>
                {attachment.result.kind.replaceAll("_", " ")} · {canonicalDocument.provenance.completed_at} · {attachment.result_id}
              </option>
            ))}
          </select>
          <small>
            Select the exact strict-reopen result used for display, export, or a heterogeneity discovery lock.
          </small>
        </label>
      ) : null}
      <NativeMultiModLabsWorkspace
        access={access}
        model={model}
        caseCount={caseCount}
        groupingColumns={profiledGroupingColumns ?? groupingColumns}
        initialTab={initialTab}
        operationPending={operationPending || jobActive}
        onAssessRuntime={assessRuntime}
        onStageRecipe={stageRecipe}
        onExecute={execute}
        onPrepareRawProbeMetrics={prepareRawProbeMetrics}
        validatedResult={completed?.attachment ?? selectedResidentResult?.attachment}
        canonicalResultDocument={
          completed?.canonicalDocument ?? selectedResidentResult?.canonicalDocument
        }
        rawSidecarExportAuthority={
          completed || selectedResidentResult
            ? {
                archivePath: completed?.archivePath ?? authority.archivePath,
                archiveSha256:
                  completed?.archiveSha256 ?? authority.archiveSha256,
                projectId: completed?.projectId ?? authority.projectId,
                access,
              }
            : undefined
        }
      />
    </div>
  );
}

export default NativeMultiModJobWorkspaceV1;
