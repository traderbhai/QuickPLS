const STANDALONE_NATIVE_METHODS = new Set<string>([
  "nca",
  "pca",
  "regression",
]);

/**
 * Native standalone analyses consume observed dataset variables and do not
 * create or select an editable SEM model. Keep this predicate centralized so
 * future standalone regression surfaces can opt in without duplicating
 * project-persistence rules.
 */
export function isStandaloneNativeAnalysis(methodOrKind: string | null | undefined): boolean {
  return typeof methodOrKind === "string" && STANDALONE_NATIVE_METHODS.has(methodOrKind);
}
