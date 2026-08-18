import type { AnalysisMethodId, AnalysisUiSettings, MethodDefinition, MethodStatus } from "../types";

export const selectableAnalysisMethodIds = new Set<AnalysisMethodId>([
  "pls_pm",
  "bootstrap",
  "permutation",
  "pls_sample_size_power",
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
  if (status === "validated") return "Supported";
  if (status === "experimental") return "Experimental";
  return "Not available";
};

export const effectiveMethodStatus = (method: MethodDefinition | undefined, settings?: AnalysisUiSettings): MethodStatus => {
  if (!method || !selectableAnalysisMethodIds.has(method.id as AnalysisMethodId)) return "unsupported";
  if (method.id === "pls_sample_size_power") return "validated";
  if (method.id === "cbsem") return "validated";
  if (method.id === "mga") return "experimental";
  if (method.id === "regression") {
    const regressionType = settings?.regressionType ?? "ols";
    if (regressionType === "ols" || regressionType === "logistic") return "validated";
    if (regressionType === "process") return "validated";
    return "experimental";
  }
  return method.status;
};

export const effectiveMethodStatusLabel = (method: MethodDefinition | undefined, settings?: AnalysisUiSettings) =>
  methodStatusLabel(effectiveMethodStatus(method, settings));

export const methodStatusDescription = (method: MethodDefinition, settings?: AnalysisUiSettings) => {
  if (!selectableAnalysisMethodIds.has(method.id as AnalysisMethodId)) return "Configured through another supported method setting, not as a standalone run method.";
  const status = effectiveMethodStatus(method, settings);
  if (method.id === "mga") {
    const groupMethods = (settings?.groupMethods ?? "micom")
      .split(",")
      .map((item) => item.trim())
      .filter(Boolean);
    return groupMethods.length === 1 && groupMethods[0] === "micom"
      ? "Experimental MICOM v3.1 requires researcher confirmation of Step 1 and calculates only Steps 2 and 3. Structural-path permutation MGA is a separate workflow."
      : "This combined or non-MICOM group selection is not available for a new calculation. Choose MICOM v3.1 and review its requirements.";
  }
  if (method.id === "regression" && (settings?.regressionType ?? "ols") === "process") return "Supports the graph-defined continuous-outcome PROCESS v2 workflow for the model and data requirements listed in Method Details; historical PROCESS v1 results remain readable in saved projects.";
  if (method.id === "regression" && status === "validated") return "Supports OLS and binary logistic regression with the listed model and data requirements.";
  if (method.id === "regression") return "This regression variant is not available for a new calculation.";
  if (method.id === "permutation") return "Supports fixed-score path inference under exchangeable reduced-model residuals with raw, unadjusted, pathwise plus-one p values; current calibration covers homoscedastic Gaussian errors.";
  if (method.id === "pls_sample_size_power") return "Supports prospective Monte Carlo power v2 for exactly one two-construct reflective Gaussian path, an explicit sample-size grid, and the null-centered two-sided case-bootstrap plus-one test. It is not retrospective observed power or a general sample-size guarantee.";
  if (["cca", "cta_pls", "endogeneity", "nonlinear_effects", "moderated_mediation"].includes(method.id)) return "Supports the requirements listed in Method Details; broader variants are not available.";
  if (method.id === "cbsem") return "Supports point-only raw-data, single-group, reflective CFA or recursive SEM using maximum likelihood. The separate Exact CB-SEM workspace supports the registered exact-CFA case-bootstrap family; historical bootstrap identities remain read-only.";
  if (method.id === "gsca") return "Supports deterministic component models with the listed reflective, formative, and recursive-path requirements; unrestricted GSCA variants are not available.";
  if (status === "validated") return "Supported for the model and data requirements listed in Method Details.";
  if (status === "experimental") return "Experimental. Review Method Details and independently check the result before final reporting.";
  return "Not available in this build.";
};
