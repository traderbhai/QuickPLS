import type {
  AnalysisUiSettings,
  Dataset,
  DatasetGroupProfile,
  DatasetGroupProfileValue,
} from "../types";

export const NATIVE_MGA_MIN_COMPLETE_CASES = 10;
export const NATIVE_MGA_MIN_PERMUTATIONS = 5_000;
export const NATIVE_MGA_MAX_PERMUTATIONS = 10_000;

export interface NativeMgaProfileAssessment {
  canRun: boolean;
  blockers: string[];
  warnings: string[];
  groupA: DatasetGroupProfileValue | null;
  groupB: DatasetGroupProfileValue | null;
}

export function nativeEligibleGroupColumns(
  dataset: Readonly<Dataset>,
  analysisColumns: readonly string[],
): string[] {
  const indicators = new Set(analysisColumns);
  return dataset.columns.filter((column) => !indicators.has(column));
}

export function residentDatasetGroupProfile(
  dataset: Readonly<Dataset>,
  columnName: string,
  analysisColumns: readonly string[],
): DatasetGroupProfile | null {
  const rowCount = dataset.rowCount ?? dataset.rows.length;
  if (!columnName || !dataset.columns.includes(columnName) || dataset.rows.length < rowCount) return null;

  const metadata = dataset.columnMetadata?.find((column) => column.name === columnName);
  const groups = new Map<string, { observations: number; completeCases: number }>();
  let missingCount = 0;
  let unsupportedCount = 0;

  for (const row of dataset.rows) {
    const value = normalizedGroupValue(row[columnName]);
    if (value === null) {
      missingCount += 1;
      continue;
    }
    if (value === undefined) {
      unsupportedCount += 1;
      continue;
    }
    const current = groups.get(value) ?? { observations: 0, completeCases: 0 };
    current.observations += 1;
    if (analysisColumns.every((column) => finiteResidentValue(row[column]))) current.completeCases += 1;
    groups.set(value, current);
  }

  return {
    datasetId: dataset.id,
    columnName,
    rowCount,
    missingCount,
    unsupportedCount,
    truncated: false,
    groups: [...groups.entries()]
      .sort(([left], [right]) => left < right ? -1 : left > right ? 1 : 0)
      .map(([value, counts]) => ({
        value,
        label: metadata?.value_labels?.[value]?.trim() || null,
        ...counts,
      })),
  };
}

export function nativeMgaProfileAssessment(
  profile: DatasetGroupProfile | null,
  settings: Readonly<AnalysisUiSettings>,
): NativeMgaProfileAssessment {
  return assessNativeGroups(profile, settings, true);
}

/** Validates A/B selection without applying calculation-only MICOM/MGA gates. */
export function nativeGroupSelectionAssessment(
  profile: DatasetGroupProfile | null,
  settings: Readonly<AnalysisUiSettings>,
): NativeMgaProfileAssessment {
  return assessNativeGroups(profile, settings, false);
}

function assessNativeGroups(
  profile: DatasetGroupProfile | null,
  settings: Readonly<AnalysisUiSettings>,
  requireCalculationSetup: boolean,
): NativeMgaProfileAssessment {
  const blockers: string[] = [];
  const warnings: string[] = [];
  const groupAValue = settings.groupAValue?.trim() ?? "";
  const groupBValue = settings.groupBValue?.trim() ?? "";
  const groupA = profile?.groups.find((group) => group.value === groupAValue) ?? null;
  const groupB = profile?.groups.find((group) => group.value === groupBValue) ?? null;

  if (!profile) blockers.push("Load the complete dataset group profile before starting MICOM.");
  else {
    if (profile.columnName !== (settings.groupColumn?.trim() ?? "")) {
      blockers.push("The group profile is stale for the selected grouping variable.");
    }
    if (profile.truncated) blockers.push("This variable has too many distinct values for the native two-group selector.");
    if (profile.unsupportedCount > 0) blockers.push(`${profile.unsupportedCount} group value${profile.unsupportedCount === 1 ? " is" : "s are"} not supported.`);
    if (profile.groups.length < 2) blockers.push("The selected variable must contain at least two observed group values.");
    if (!groupA) blockers.push("Choose an observed value for Group A.");
    if (!groupB) blockers.push("Choose an observed value for Group B.");
    if (groupAValue && groupAValue === groupBValue) blockers.push("Group A and Group B must use different values.");
    if (groupA && groupA.completeCases < NATIVE_MGA_MIN_COMPLETE_CASES) {
      blockers.push(`Group A has ${groupA.completeCases} complete model cases; at least ${NATIVE_MGA_MIN_COMPLETE_CASES} are required.`);
    }
    if (groupB && groupB.completeCases < NATIVE_MGA_MIN_COMPLETE_CASES) {
      blockers.push(`Group B has ${groupB.completeCases} complete model cases; at least ${NATIVE_MGA_MIN_COMPLETE_CASES} are required.`);
    }
    if (
      groupA
      && groupB
      && Math.max(groupA.completeCases, groupB.completeCases)
        > Math.min(groupA.completeCases, groupB.completeCases) * 10
    ) {
      blockers.push(`micom.extreme_group_imbalance: selected complete-case sizes ${groupA.completeCases} and ${groupB.completeCases} exceed the bounded 10:1 ratio.`);
    }
    if (profile.missingCount > 0) warnings.push(`${profile.missingCount} row${profile.missingCount === 1 ? " has" : "s have"} a missing group value and will be excluded.`);
    const selectedObservations = (groupA?.observations ?? 0) + (groupB?.observations ?? 0);
    const otherObserved = Math.max(0, profile.rowCount - profile.missingCount - profile.unsupportedCount - selectedObservations);
    if (otherObserved > 0) warnings.push(`${otherObserved} row${otherObserved === 1 ? " belongs" : "s belong"} to unselected group values and will be excluded.`);
  }

  if (requireCalculationSetup) {
    const methods = (settings.groupMethods ?? "")
      .split(",")
      .map((method) => method.trim())
      .filter(Boolean);
    if (methods.length !== 2 || methods[0] !== "micom" || methods[1] !== "mga_permutation") {
      blockers.push("The group workflow requires both MICOM and structural-path permutation MGA.");
    }
    if (settings.micomConfiguralConfirmed !== true) {
      blockers.push("Confirm MICOM Step 1 through explicit researcher review before calculation.");
    }

    const samples = settings.groupPermutationSamples ?? NATIVE_MGA_MIN_PERMUTATIONS;
    if (
      !Number.isInteger(samples)
      || samples < NATIVE_MGA_MIN_PERMUTATIONS
      || samples > NATIVE_MGA_MAX_PERMUTATIONS
    ) {
      blockers.push(`MICOM requires ${NATIVE_MGA_MIN_PERMUTATIONS.toLocaleString("en-US")} to ${NATIVE_MGA_MAX_PERMUTATIONS.toLocaleString("en-US")} permutations.`);
    }
  }

  return { canRun: blockers.length === 0, blockers, warnings, groupA, groupB };
}

export function nativeGroupOptionLabel(group: DatasetGroupProfileValue): string {
  const identity = group.label && group.label !== group.value
    ? `${group.label} [${group.value}]`
    : group.value;
  return `${identity} — ${group.completeCases} complete of ${group.observations}`;
}

function finiteResidentValue(value: string | number | null | undefined): boolean {
  if (value === null || value === undefined || value === "") return false;
  const numeric = typeof value === "number" ? value : Number(value);
  return Number.isFinite(numeric);
}

function normalizedGroupValue(value: unknown): string | null | undefined {
  if (value === null || value === undefined) return null;
  if (typeof value === "string") return value.trim() || null;
  if (typeof value === "number") {
    if (!Number.isFinite(value)) return undefined;
    return Number.isInteger(value) ? value.toFixed(0) : String(value);
  }
  if (typeof value === "boolean") return String(value);
  return undefined;
}
