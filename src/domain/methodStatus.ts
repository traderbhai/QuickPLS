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
  if (method.id === "regression") {
    return (settings?.regressionType ?? "ols") === "ols" ? "validated" : "experimental";
  }
  return method.status;
};

export const effectiveMethodStatusLabel = (method: MethodDefinition | undefined, settings?: AnalysisUiSettings) =>
  methodStatusLabel(effectiveMethodStatus(method, settings));

export const methodStatusDescription = (method: MethodDefinition, settings?: AnalysisUiSettings) => {
  if (!selectableAnalysisMethodIds.has(method.id as AnalysisMethodId)) return "Configured through another supported method setting, not as a standalone run method.";
  const status = effectiveMethodStatus(method, settings);
  if (method.id === "regression" && status === "validated") return "OLS regression is validated for the documented QuickPLS v1.2 OLS scope; logistic and PROCESS variants remain experimental.";
  if (method.id === "regression") return "Logistic regression and PROCESS-style workflows remain experimental and require explicit method-status warnings.";
  if (status === "validated") return "Validated for the documented QuickPLS supported scope.";
  if (status === "experimental") return "Available with explicit method-status warnings and watermarked exports where applicable.";
  return "Not available in this build.";
};
