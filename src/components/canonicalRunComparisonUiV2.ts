import { useEffect, useState } from "react";
import {
  canonicalRunComparisonFromAnalysisRunsV2,
  type NativeCanonicalRunComparisonBuildV2,
} from "../native/nativeCanonicalRunComparisonV2";
import type { AnalysisRun } from "../types";

export type CanonicalRunComparisonUiStateV2 =
  | { status: "missing" }
  | { status: "loading" }
  | NativeCanonicalRunComparisonBuildV2;

interface ComparisonSnapshotV2 {
  key: string;
  result: NativeCanonicalRunComparisonBuildV2;
}

/**
 * Resolve an exact saved-run comparison while preventing a completed result
 * for an earlier selection from appearing under a newer pair of run names.
 */
export function useCanonicalRunComparisonV2(
  first: AnalysisRun | undefined,
  second: AnalysisRun | undefined,
): CanonicalRunComparisonUiStateV2 {
  const comparisonKey = first && second ? `${first.id}\u0000${second.id}` : null;
  const [snapshot, setSnapshot] = useState<ComparisonSnapshotV2 | null>(null);

  useEffect(() => {
    if (!first || !second || !comparisonKey) return;
    let current = true;
    void canonicalRunComparisonFromAnalysisRunsV2(first, second).then((result) => {
      if (current) setSnapshot({ key: comparisonKey, result });
    });
    return () => {
      current = false;
    };
  }, [comparisonKey, first, second]);

  if (!comparisonKey) return { status: "missing" };
  if (snapshot?.key !== comparisonKey) return { status: "loading" };
  return snapshot.result;
}

export interface CanonicalComparisonAvailabilityCopyV2 {
  available: boolean;
  description: string;
  actionTitle: string;
}

/** Shared Results/Report wording derived from the exact comparison state. */
export function canonicalComparisonAvailabilityCopyV2(
  state: CanonicalRunComparisonUiStateV2,
): CanonicalComparisonAvailabilityCopyV2 {
  if (state.status === "ready") {
    const { row_count: rows, changed_cell_count: changes } = state.comparison.summary;
    return {
      available: true,
      description: `${rows} typed result row(s) are comparable in Results; ${changes} value(s) differ.`,
      actionTitle: "Open the exact side-by-side comparison in Results",
    };
  }
  if (state.status === "loading") {
    return {
      available: false,
      description: "Checking whether the selected runs can be compared.",
      actionTitle: "Comparison check in progress",
    };
  }
  if (state.status === "blocked") {
    const first = state.issues[0];
    return {
      available: false,
      description: first ? `${first.title}: ${first.message}` : "These runs cannot be compared.",
      actionTitle: first?.message ?? "Choose two compatible completed runs",
    };
  }
  if (state.status === "unavailable") {
    const message = state.messages[0] ?? "QuickPLS could not prepare this comparison.";
    return { available: false, description: message, actionTitle: message };
  }
  return {
    available: false,
    description: "Select two completed runs to check their compatibility.",
    actionTitle: "Select two completed runs before comparing",
  };
}
