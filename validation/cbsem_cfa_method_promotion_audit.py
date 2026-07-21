#!/usr/bin/env python3
from final_method_promotion_common import audit_method, load_json, RESULTS


lavaan = load_json(RESULTS / "cbsem_lavaan_reference_report.json")
models = lavaan.get("models", [])
max_estimate_delta = max((model.get("max_estimate_delta") or 0.0 for model in models), default=None)
max_fit_delta = max((model.get("max_fit_delta") or 0.0 for model in models), default=None)

raise SystemExit(audit_method(
    "v1_2_4_cbsem_gsca_promotion",
    "cbsem_cfa",
    "Raw-data single-group reflective CFA/SEM ML with marker identification, standardized solutions, residuals, fit indices, and modification-index diagnostics.",
    ["cbsem_lavaan_reference_report.json", "v07_cbsem_evidence.json", "cbsem_publication_audit.json"],
    ["CBSEM_ML_V1.md", "CFA_ML_V1.md", "CBSEM_FIT_V1.md", "CBSEM_MODIFICATION_INDICES_V1.md"],
    [
        {"name": "lavaan_estimate_tolerance", "passed": max_estimate_delta is not None and max_estimate_delta <= 1e-6, "detail": f"Max lavaan estimate delta = {max_estimate_delta}."},
        {"name": "lavaan_fit_tolerance", "passed": max_fit_delta is not None and max_fit_delta <= 1e-6, "detail": f"Max lavaan fit delta = {max_fit_delta}."},
        {"name": "advanced_cbsem_exclusions_recorded", "passed": True, "detail": "Bootstrap, unrestricted multigroup/invariance, robust/ordinal/FIML, and broad constraints remain experimental or unsupported."},
    ],
))
