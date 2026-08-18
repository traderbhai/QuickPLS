import { AlertTriangle, Info, LoaderCircle } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { profileNativeDatasetGroups } from "../services/projectService";
import type { AnalysisUiSettings, Dataset, DatasetGroupProfile } from "../types";
import {
  nativeEligibleGroupColumns,
  nativeGroupSelectionAssessment,
  nativeGroupOptionLabel,
  residentDatasetGroupProfile,
} from "./nativeMga";

export type NativeGroupSetupPatch = Pick<
  AnalysisUiSettings,
  "groupColumn" | "groupAValue" | "groupBValue"
>;

interface NativeGroupSetupDialogProps {
  dataset: Dataset;
  analysisColumns: readonly string[];
  initialColumn?: string | null;
  settings: AnalysisUiSettings;
  nativeDesktop: boolean;
  projectWritable: boolean;
  apply: (patch: NativeGroupSetupPatch) => void;
  close: () => void;
}

interface GroupProfileState {
  key: string;
  status: "idle" | "loading" | "ready" | "error";
  profile: DatasetGroupProfile | null;
  error?: string;
}

export function nativeInitialGroupingColumn(
  dataset: Readonly<Dataset>,
  analysisColumns: readonly string[],
  requestedColumn?: string | null,
  configuredColumn?: string | null,
): string {
  const eligible = new Set(nativeEligibleGroupColumns(dataset, analysisColumns));
  const requested = requestedColumn?.trim() ?? "";
  const configured = configuredColumn?.trim() ?? "";
  if (requested && eligible.has(requested)) return requested;
  if (configured && eligible.has(configured)) return configured;
  return [...eligible][0] ?? "";
}

export default function NativeGroupSetupDialog({
  dataset,
  analysisColumns,
  initialColumn,
  settings,
  nativeDesktop,
  projectWritable,
  apply,
  close,
}: NativeGroupSetupDialogProps) {
  const analysisColumnKey = [...new Set(analysisColumns)].sort().join("\u0000");
  const stableAnalysisColumns = useMemo(
    () => analysisColumnKey ? analysisColumnKey.split("\u0000") : [],
    [analysisColumnKey],
  );
  const eligibleColumns = useMemo(
    () => nativeEligibleGroupColumns(dataset, stableAnalysisColumns),
    [dataset, stableAnalysisColumns],
  );
  const configuredColumn = settings.groupColumn?.trim() ?? "";
  const initialGroupingColumn = nativeInitialGroupingColumn(
    dataset,
    stableAnalysisColumns,
    initialColumn,
    configuredColumn,
  );
  const [groupColumn, setGroupColumn] = useState(initialGroupingColumn);
  const [groupAValue, setGroupAValue] = useState<string | null>(
    initialGroupingColumn === configuredColumn ? settings.groupAValue?.trim() || null : null,
  );
  const [groupBValue, setGroupBValue] = useState<string | null>(
    initialGroupingColumn === configuredColumn ? settings.groupBValue?.trim() || null : null,
  );
  const profileKey = `${dataset.id}\u0000${groupColumn}\u0000${analysisColumnKey}`;
  const residentProfile = useMemo(
    () => groupColumn
      ? residentDatasetGroupProfile(dataset, groupColumn, stableAnalysisColumns)
      : null,
    [dataset, groupColumn, stableAnalysisColumns],
  );
  const [profileState, setProfileState] = useState<GroupProfileState>(() => ({
    key: profileKey,
    status: residentProfile ? "ready" : groupColumn ? "idle" : "idle",
    profile: residentProfile,
  }));

  useEffect(() => {
    if (!groupColumn) {
      setProfileState({ key: profileKey, status: "idle", profile: null });
      return;
    }
    const resident = residentDatasetGroupProfile(dataset, groupColumn, stableAnalysisColumns);
    if (resident) {
      setProfileState({ key: profileKey, status: "ready", profile: resident });
      return;
    }
    if (!nativeDesktop) {
      setProfileState({
        key: profileKey,
        status: "error",
        profile: null,
        error: "Open the installed Windows app to profile all rows in this dataset.",
      });
      return;
    }

    let active = true;
    setProfileState({ key: profileKey, status: "loading", profile: null });
    void profileNativeDatasetGroups(dataset.id, groupColumn, stableAnalysisColumns)
      .then((profile) => {
        if (!active) return;
        if (profile.datasetId !== dataset.id || profile.columnName !== groupColumn) {
          throw new Error("The dataset or grouping variable changed while its profile was loading.");
        }
        setProfileState({ key: profileKey, status: "ready", profile });
      })
      .catch((reason: unknown) => {
        if (!active) return;
        setProfileState({
          key: profileKey,
          status: "error",
          profile: null,
          error: reason instanceof Error ? reason.message : String(reason),
        });
      });
    return () => { active = false; };
  }, [dataset, groupColumn, nativeDesktop, profileKey, stableAnalysisColumns]);

  const currentProfileState = profileState.key === profileKey
    ? profileState
    : { key: profileKey, status: "loading" as const, profile: null };
  const profile = currentProfileState.status === "ready" ? currentProfileState.profile : null;

  useEffect(() => {
    if (!profile?.groups.length) return;
    const values = profile.groups.map((group) => group.value);
    const effectiveA = groupAValue && values.includes(groupAValue) ? groupAValue : values[0] ?? null;
    const effectiveB = groupBValue && values.includes(groupBValue) && groupBValue !== effectiveA
      ? groupBValue
      : values.find((value) => value !== effectiveA) ?? null;
    if (effectiveA !== groupAValue) setGroupAValue(effectiveA);
    if (effectiveB !== groupBValue) setGroupBValue(effectiveB);
  }, [groupAValue, groupBValue, profile]);

  const assessment = nativeGroupSelectionAssessment(profile, {
    ...settings,
    groupColumn: groupColumn || null,
    groupAValue,
    groupBValue,
  });
  const groupColumnEligible = Boolean(groupColumn && eligibleColumns.includes(groupColumn));
  const canApply = projectWritable
    && groupColumnEligible
    && currentProfileState.status === "ready"
    && assessment.canRun;
  const fieldPrefix = `nd-groups-${dataset.id.replace(/[^a-zA-Z0-9_-]/g, "-")}`;

  const selectColumn = (column: string) => {
    setGroupColumn(column);
    if (column === configuredColumn) {
      setGroupAValue(settings.groupAValue?.trim() || null);
      setGroupBValue(settings.groupBValue?.trim() || null);
    } else {
      setGroupAValue(null);
      setGroupBValue(null);
    }
  };

  const submit = () => {
    if (!canApply) return;
    apply({ groupColumn, groupAValue, groupBValue });
    close();
  };

  const clear = () => {
    if (!projectWritable || !configuredColumn) return;
    apply({ groupColumn: null, groupAValue: null, groupBValue: null });
    close();
  };

  return <form className="nd-group-setup-dialog" onSubmit={(event) => { event.preventDefault(); submit(); }}>
    <div className="nd-group-setup-content">
      {!projectWritable ? <p className="nd-group-notice" role="status"><Info size={14} aria-hidden="true" />This project is read-only. Save a writable copy before changing groups.</p> : null}
      {!eligibleColumns.length ? <p className="nd-form-error" role="alert">No unassigned variable is available. Remove a candidate variable from all constructs first.</p> : null}
      <label className="nd-group-column" htmlFor={`${fieldPrefix}-column`}>Grouping variable
        <select
          id={`${fieldPrefix}-column`}
          autoFocus
          value={groupColumn}
          disabled={!projectWritable || !eligibleColumns.length}
          onChange={(event) => selectColumn(event.target.value)}
        >
          {!groupColumn ? <option value="">Choose a variable</option> : null}
          {eligibleColumns.map((column) => {
            const label = dataset.columnMetadata?.find((item) => item.name === column)?.label?.trim();
            return <option key={column} value={column}>{label && label !== column ? `${label} [${column}]` : column}</option>;
          })}
        </select>
      </label>

      {currentProfileState.status === "loading" || currentProfileState.status === "idle" && groupColumn ? <p className="nd-group-profile-state" role="status" aria-live="polite"><LoaderCircle className="nd-spin" size={14} aria-hidden="true" />Profiling all dataset rows...</p> : null}
      {currentProfileState.status === "error" ? <p className="nd-form-error" role="alert">Could not load groups. {currentProfileState.error}</p> : null}

      {profile ? <>
        <div className="nd-group-selectors">
          <label htmlFor={`${fieldPrefix}-a`}>Group A
            <select id={`${fieldPrefix}-a`} value={groupAValue ?? ""} onChange={(event) => {
              const value = event.target.value || null;
              setGroupAValue(value);
              if (value && value === groupBValue) setGroupBValue(null);
            }}>
              <option value="">Choose an observed value</option>
              {profile.groups.map((group) => <option key={group.value} value={group.value} disabled={group.value === groupBValue}>{nativeGroupOptionLabel(group)}</option>)}
            </select>
          </label>
          <label htmlFor={`${fieldPrefix}-b`}>Group B
            <select id={`${fieldPrefix}-b`} value={groupBValue ?? ""} onChange={(event) => {
              const value = event.target.value || null;
              setGroupBValue(value);
              if (value && value === groupAValue) setGroupAValue(null);
            }}>
              <option value="">Choose an observed value</option>
              {profile.groups.map((group) => <option key={group.value} value={group.value} disabled={group.value === groupAValue}>{nativeGroupOptionLabel(group)}</option>)}
            </select>
          </label>
        </div>

        {assessment.groupA || assessment.groupB ? <table className="nd-group-counts">
          <caption className="nd-sr-only">Selected group case counts</caption>
          <thead><tr><th>Role</th><th>Value</th><th>Observed</th><th>Complete model cases</th></tr></thead>
          <tbody>
            {assessment.groupA ? <tr><th scope="row">A</th><td>{assessment.groupA.label || assessment.groupA.value}</td><td>{assessment.groupA.observations.toLocaleString()}</td><td>{assessment.groupA.completeCases.toLocaleString()}</td></tr> : null}
            {assessment.groupB ? <tr><th scope="row">B</th><td>{assessment.groupB.label || assessment.groupB.value}</td><td>{assessment.groupB.observations.toLocaleString()}</td><td>{assessment.groupB.completeCases.toLocaleString()}</td></tr> : null}
          </tbody>
        </table> : null}

        {assessment.warnings.length ? <ul className="nd-group-warnings" aria-label="Group exclusions">{assessment.warnings.map((warning) => <li key={warning}><AlertTriangle size={13} aria-hidden="true" />{warning}</li>)}</ul> : null}
        {assessment.blockers.length ? <div className="nd-form-error" role="alert"><strong>Groups cannot be applied</strong><ul>{assessment.blockers.map((blocker) => <li key={blocker}>{blocker}</li>)}</ul></div> : null}
      </> : null}

    </div>
    <footer>
      {configuredColumn ? <button type="button" className="danger" disabled={!projectWritable} onClick={clear}>Clear groups</button> : null}
      <span className="spacer" />
      <button type="button" onClick={close}>Cancel</button>
      <button className="primary" type="submit" disabled={!canApply}>Apply Groups</button>
    </footer>
  </form>;
}
