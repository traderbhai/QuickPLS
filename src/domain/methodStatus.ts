import type { AnalysisMethodId, AnalysisUiSettings, MethodDefinition, MethodStatus } from "../types";

export const selectableAnalysisMethodIds = new Set<AnalysisMethodId>([
  "pls_pm",
  "bootstrap",
  "plsc",
  "wpls",
  "cca",
  "cta_pls",
  "endogeneity",
  "nonlinear_effects",
  "moderated_mediation",
  "predict",
  "mga",
  "ipma",
  "cbsem",
  "pca",
  "gsca",
  "regression",
  "nca",
]);

export const isSelectableAnalysisMethod = (method: MethodDefinition): method is MethodDefinition & { id: AnalysisMethodId } =>
  method.status !== "unsupported" && selectableAnalysisMethodIds.has(method.id as AnalysisMethodId);

export const methodStatusLabel = (status: MethodStatus) => {
  if (status === "validated") return "Validated";
  if (status === "experimental") return "Experimental";
  return "Unsupported";
};

export const effectiveMethodStatus = (method: MethodDefinition | undefined, settings?: AnalysisUiSettings): MethodStatus => {
  if (!method || !selectableAnalysisMethodIds.has(method.id as AnalysisMethodId)) return "unsupported";
  if (method.id === "mga") {
    const groupMethods = (settings?.groupMethods ?? "micom,mga_permutation")
      .split(",")
      .map((item) => item.trim())
      .filter(Boolean);
    return groupMethods.length > 0 && groupMethods.every((item) => item === "micom" || item === "mga_permutation") ? "validated" : "experimental";
  }
  if (method.id === "regression") {
    const regressionType = settings?.regressionType ?? "ols";
    if (regressionType === "ols" || regressionType === "logistic") return "validated";
    if (regressionType === "process" && (settings?.processModel ?? "mediation") !== "moderated_mediation") return "validated";
    return "experimental";
  }
  return method.status;
};

export const effectiveMethodStatusLabel = (method: MethodDefinition | undefined, settings?: AnalysisUiSettings) =>
  methodStatusLabel(effectiveMethodStatus(method, settings));

export const methodStatusDescription = (method: MethodDefinition, settings?: AnalysisUiSettings) => {
  if (!selectableAnalysisMethodIds.has(method.id as AnalysisMethodId)) return "Configured through another supported method setting, not as a standalone run method.";
  const status = effectiveMethodStatus(method, settings);
  if (method.id === "mga" && status === "validated") return "MICOM and permutation MGA are validated for the documented QuickPLS v1.2.2 two-group scope.";
  if (method.id === "mga") return "Unsupported group workflows remain experimental or blocked.";
  if (method.id === "regression" && status === "validated") return "OLS, binary logistic, and bounded PROCESS mediation/moderation are validated for documented QuickPLS scopes; moderated mediation remains experimental.";
  if (method.id === "regression") return "PROCESS moderated mediation and broader regression workflows remain experimental.";
  if (["cca", "cta_pls", "endogeneity", "nonlinear_effects", "moderated_mediation"].includes(method.id)) return "Validated for the documented QuickPLS v1.2.3 bounded diagnostic scope; broader variants remain unsupported.";
  if (method.id === "cbsem") return "Validated for raw-data single-group reflective CFA/SEM ML; bootstrap, unrestricted multigroup/invariance, robust, ordinal, and FIML estimators remain experimental or unsupported.";
  if (method.id === "gsca") return "Validated for the documented QuickPLS v1.2.4 bounded deterministic component-model scope; unrestricted GSCA variants remain unsupported.";
  if (status === "validated") return "Validated for the documented QuickPLS supported scope.";
  if (status === "experimental") return "Available with explicit method-status warnings and watermarked exports where applicable.";
  return "Not available in this build.";
};
