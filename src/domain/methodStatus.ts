import type { AnalysisMethodId, MethodDefinition, MethodStatus } from "../types";

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

export const methodStatusDescription = (method: MethodDefinition) => {
  if (!selectableAnalysisMethodIds.has(method.id as AnalysisMethodId)) return "Configured through another supported method setting, not as a standalone run method.";
  if (method.status === "validated") return "Validated for the documented QuickPLS v1.0 supported scope.";
  if (method.status === "experimental") return "Available with explicit method-status warnings and watermarked exports where applicable.";
  return "Not available in this build.";
};
