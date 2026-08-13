#!/usr/bin/env python3
"""Fail-closed validator for the QuickPLS 3 competitor catalogue.

The catalogue is a planning crosswalk, not promotion evidence.  This validator
therefore exits successfully when the document is internally valid even while
``competitor_ready`` is false.  It fails when catalogue coverage, dependencies,
priorities, release targets, repository evidence, or the accepted parity ledger
contradict the declared plan.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from collections import Counter
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from validation import method_promotion_manifest, parity_ledger, quickpls_3_release_readiness  # noqa: E402


DEFAULT_MANIFEST = ROOT / "validation" / "quickpls_3_competitor_catalogue.json"
DEFAULT_LEDGER = ROOT / "validation" / "quickpls_3_parity_ledger.json"
DEFAULT_READINESS_CONTRACT = ROOT / "validation" / "quickpls_3_release_readiness.json"
DEFAULT_APPROVAL_ENVELOPE = ROOT / "validation" / "results" / "quickpls_3_competitor_approval.json"

CATALOGUE_DATE = "2026-08-12"
CATALOGUE_URL = "https://smartpls.com/documentation/algorithms-and-techniques/"
CATALOGUE_RELATIVE_PATH = "validation/quickpls_3_competitor_catalogue.json"
LEDGER_RELATIVE_PATH = "validation/quickpls_3_parity_ledger.json"
READINESS_RELATIVE_PATH = "validation/quickpls_3_release_readiness.json"
APPROVAL_RELATIVE_PATH = "validation/results/quickpls_3_competitor_approval.json"
MANIFEST_DIRECTORY = "validation/methods"
PARITY_VALIDATOR_PATH = "validation/parity_ledger.py"
COMMERCIAL_VALIDATOR_PATH = "validation/quickpls_3_release_readiness.py"
METHOD_MANIFEST_VALIDATOR_PATH = "validation/method_promotion_manifest.py"
METHOD_MANIFEST_SCHEMA_PATH = "validation/methods/method_promotion_manifest.schema.json"
APPROVAL_BINDING_IDS = (
    "competitor_catalogue",
    "parity_ledger",
    "parity_report",
    "commercial_readiness_contract",
    "commercial_readiness_report",
    "method_manifest_set",
    "method_manifest_report",
)
MAX_APPROVAL_FUTURE_SKEW = timedelta(minutes=5)
ALLOWED_STATUSES = (
    "release-qualified",
    "native-qualified",
    "engine-preview",
    "absent",
    "deferred",
)
ALLOWED_PRIORITIES = ("P0", "P1", "P2", "P3")
TARGET_RELEASES = (
    "current",
    "2.47.0",
    "2.48.0",
    "2.49.0",
    "2.50.0",
    "2.51.0",
    "2.52.0",
    "3.0.0-beta.1",
    "3.0.0",
    "post-3.0",
)
LEDGER_TO_PROGRAM_STATUS = {
    "absent": "absent",
    "engine_only": "engine-preview",
    "archive_qualified": "engine-preview",
    "native_qualified": "native-qualified",
    "release_qualified": "release-qualified",
}
STATUS_RANK = {
    "absent": 0,
    "engine-preview": 1,
    "native-qualified": 2,
    "release-qualified": 3,
}
METHOD_ID = re.compile(r"^smartpls\.[a-z0-9][a-z0-9_.-]*$")

# This is intentionally closed.  Changing the vendor snapshot requires an
# explicit validator update, not only editing the planning JSON.
EXPECTED_CATALOGUE = (
    ("smartpls.pls_algorithm", "Estimation & Core Algorithm", "PLS-SEM Algorithm"),
    ("smartpls.pls_power_analysis", "Estimation & Core Algorithm", "PLS-SEM Sample Size and Power Analysis"),
    ("smartpls.wpls", "Estimation & Core Algorithm", "Weighted PLS Algorithm (WPLS)"),
    ("smartpls.plsc", "Estimation & Core Algorithm", "Consistent PLS-SEM (PLSc-SEM)"),
    ("smartpls.pca_core", "Estimation & Core Algorithm", "Principal Component Analysis (PCA)"),
    ("smartpls.pls_bootstrapping", "Resampling & Inference", "Bootstrapping"),
    ("smartpls.consistent_bootstrapping", "Resampling & Inference", "Consistent Bootstrapping"),
    ("smartpls.blindfolding", "Resampling & Inference", "Blindfolding"),
    ("smartpls.permutation", "Resampling & Inference", "Permutation"),
    ("smartpls.consistent_permutation", "Resampling & Inference", "Consistent Permutation"),
    ("smartpls.cvpat", "Resampling & Inference", "Cross-validated Predictive Ability Test (CVPAT)"),
    ("smartpls.cca", "Validity, Reliability & Model Fit", "Confirmatory Composite Analysis (CCA)"),
    ("smartpls.cta_pls", "Validity, Reliability & Model Fit", "Confirmatory Tetrad Analysis in PLS (CTA-PLS)"),
    ("smartpls.htmt", "Validity, Reliability & Model Fit", "Discriminant Validity Assessment and HTMT"),
    ("smartpls.gof", "Validity, Reliability & Model Fit", "Goodness of Fit (GoF)"),
    ("smartpls.model_fit", "Validity, Reliability & Model Fit", "Model Fit"),
    ("smartpls.pls_model_comparison", "Validity, Reliability & Model Fit", "Model Comparison"),
    ("smartpls.prediction_oriented_model_selection", "Validity, Reliability & Model Fit", "Prediction-oriented Model Selection"),
    ("smartpls.micom", "Heterogeneity & Multigroup Analysis", "Measurement Invariance Assessment (MICOM)"),
    ("smartpls.mga", "Heterogeneity & Multigroup Analysis", "Multigroup Analysis (MGA)"),
    ("smartpls.consistent_mga", "Heterogeneity & Multigroup Analysis", "Consistent Multigroup Analysis (MGA)"),
    ("smartpls.plspredict", "Prediction & Segmentation", "PLSpredict"),
    ("smartpls.pls_pos", "Prediction & Segmentation", "PLS Prediction-oriented Segmentation (PLS-POS)"),
    ("smartpls.fimix_pls", "Prediction & Segmentation", "Finite Mixture Partial Least Squares (FIMIX-PLS)"),
    ("smartpls.ipma", "Prediction & Segmentation", "Importance-performance Map Analysis (IPMA)"),
    ("smartpls.moderation", "Extended Relationships", "Moderation"),
    ("smartpls.mediation", "Extended Relationships", "Mediation"),
    ("smartpls.nonlinear_relationships", "Extended Relationships", "Nonlinear Relationships"),
    ("smartpls.higher_order_models", "Extended Relationships", "Higher-order Models"),
    ("smartpls.endogeneity_gaussian_copulas", "Extended Relationships", "Endogeneity and Gaussian Copulas"),
    ("smartpls.gsca", "Generalized Structured Component Analysis (GSCA)", "GSCA algorithm and bootstrapping"),
    ("smartpls.logistic_regression", "Regression, Path Analysis and PROCESS", "Logistic Regression"),
    ("smartpls.nca", "Regression, Path Analysis and PROCESS", "Necessary Condition Analysis (NCA)"),
    ("smartpls.process", "Regression, Path Analysis and PROCESS", "Path Analysis and PROCESS"),
    ("smartpls.process_bootstrapping", "Regression, Path Analysis and PROCESS", "Path Analysis and PROCESS Bootstrapping"),
    ("smartpls.regression", "Regression, Path Analysis and PROCESS", "Regression"),
    ("smartpls.regression_bootstrapping", "Regression, Path Analysis and PROCESS", "Regression Bootstrapping"),
    ("smartpls.cbsem", "CB-SEM and CFA", "CB-SEM - Covariance-based Structural Equation Modeling (CB-SEM)"),
    ("smartpls.cbsem_bootstrapping", "CB-SEM and CFA", "CB-SEM Bootstrapping"),
    ("smartpls.cbsem_model_comparison", "CB-SEM and CFA", "CB-SEM Model Comparison"),
    ("smartpls.cbsem_mga", "CB-SEM and CFA", "CB-SEM Multigroup Analysis (MGA)"),
    ("smartpls.cbsem_measurement_invariance", "CB-SEM and CFA", "CB-SEM Measurement Invariance Assessment"),
    ("smartpls.cbsem_moderator", "CB-SEM and CFA", "CB-SEM Moderator Analysis"),
    ("smartpls.cfa", "CB-SEM and CFA", "Confirmatory Factor Analysis (CFA)"),
    ("smartpls.pca_cbsem", "CB-SEM and CFA", "Principal component analysis (PCA)"),
)

# Every crosswalk edge is frozen.  Shared mappings are deliberate: for example,
# the same bounded PCA capability appears in two official catalogue contexts,
# while the official permutation row maps to two separately evidenced QuickPLS
# capabilities.  Adding or borrowing a capability requires a reviewed code and
# manifest change.
EXPECTED_CAPABILITY_MAPPING = {
    "smartpls.pls_algorithm": frozenset({"qpls3.pls.algorithm"}),
    "smartpls.pls_power_analysis": frozenset(),
    "smartpls.wpls": frozenset({"qpls3.pls.weighted"}),
    "smartpls.plsc": frozenset({"qpls3.pls.consistent"}),
    "smartpls.pca_core": frozenset({"qpls3.standalone.pca"}),
    "smartpls.pls_bootstrapping": frozenset({"qpls3.inference.bootstrap"}),
    "smartpls.consistent_bootstrapping": frozenset(),
    "smartpls.blindfolding": frozenset(),
    "smartpls.permutation": frozenset(
        {
            "qpls3.groups.micom_permutation_mga",
            "qpls3.inference.structural_path_randomization",
        }
    ),
    "smartpls.consistent_permutation": frozenset(),
    "smartpls.cvpat": frozenset({"qpls3.prediction.plspredict_cvpat"}),
    "smartpls.cca": frozenset({"qpls3.assessment.cca_residuals"}),
    "smartpls.cta_pls": frozenset(),
    "smartpls.htmt": frozenset(),
    "smartpls.gof": frozenset(),
    "smartpls.model_fit": frozenset(),
    "smartpls.pls_model_comparison": frozenset(),
    "smartpls.prediction_oriented_model_selection": frozenset(),
    "smartpls.micom": frozenset({"qpls3.groups.micom_permutation_mga"}),
    "smartpls.mga": frozenset({"qpls3.groups.micom_permutation_mga"}),
    "smartpls.consistent_mga": frozenset(),
    "smartpls.plspredict": frozenset({"qpls3.prediction.plspredict_cvpat"}),
    "smartpls.pls_pos": frozenset(),
    "smartpls.fimix_pls": frozenset(),
    "smartpls.ipma": frozenset({"qpls3.assessment.ipma"}),
    "smartpls.moderation": frozenset(),
    "smartpls.mediation": frozenset(),
    "smartpls.nonlinear_relationships": frozenset(),
    "smartpls.higher_order_models": frozenset(),
    "smartpls.endogeneity_gaussian_copulas": frozenset(),
    "smartpls.gsca": frozenset({"qpls3.gsca.als"}),
    "smartpls.logistic_regression": frozenset({"qpls3.standalone.logistic"}),
    "smartpls.nca": frozenset({"qpls3.standalone.nca"}),
    "smartpls.process": frozenset({"qpls3.standalone.process"}),
    "smartpls.process_bootstrapping": frozenset({"qpls3.standalone.process"}),
    "smartpls.regression": frozenset({"qpls3.standalone.ols"}),
    "smartpls.regression_bootstrapping": frozenset(
        {"qpls3.standalone.regression_bootstrap"}
    ),
    "smartpls.cbsem": frozenset({"qpls3.cbsem.ml"}),
    "smartpls.cbsem_bootstrapping": frozenset(),
    "smartpls.cbsem_model_comparison": frozenset(),
    "smartpls.cbsem_mga": frozenset(),
    "smartpls.cbsem_measurement_invariance": frozenset(),
    "smartpls.cbsem_moderator": frozenset(),
    "smartpls.cfa": frozenset({"qpls3.cbsem.ml"}),
    "smartpls.pca_cbsem": frozenset({"qpls3.standalone.pca"}),
}


def _reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    document: dict[str, Any] = {}
    for key, value in pairs:
        if key in document:
            raise ValueError(f"duplicate JSON key: {key!r}")
        document[key] = value
    return document


def _reject_nonfinite_constant(value: str) -> Any:
    raise ValueError(f"non-finite JSON constant: {value}")


def load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(
            handle,
            object_pairs_hook=_reject_duplicate_keys,
            parse_constant=_reject_nonfinite_constant,
        )


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _canonical_json_bytes(value: Any) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        allow_nan=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")


def _normalize_report_value(value: Any, repository_root: Path) -> Any:
    """Normalize repository-local absolute paths before hashing a derived report."""

    if isinstance(value, dict):
        return {
            key: _normalize_report_value(item, repository_root)
            for key, item in value.items()
        }
    if isinstance(value, list):
        return [_normalize_report_value(item, repository_root) for item in value]
    if isinstance(value, tuple):
        return [_normalize_report_value(item, repository_root) for item in value]
    if isinstance(value, str):
        candidate = Path(value)
        if candidate.is_absolute():
            try:
                return candidate.resolve().relative_to(repository_root.resolve()).as_posix()
            except ValueError:
                return value
    return value


def _derived_report_sha256(report: Any, repository_root: Path) -> str:
    normalized = _normalize_report_value(report, repository_root)
    return hashlib.sha256(_canonical_json_bytes(normalized)).hexdigest()


def _repository_relative_file(path: Path, repository_root: Path, label: str) -> str:
    resolved = path.resolve()
    try:
        relative = resolved.relative_to(repository_root.resolve()).as_posix()
    except ValueError as exc:
        raise ValueError(f"{label} must be inside the repository") from exc
    if not resolved.is_file():
        raise ValueError(f"{label} is missing: {relative}")
    return relative


def _parse_utc_timestamp(value: Any, label: str) -> datetime:
    if not isinstance(value, str) or not value.endswith("Z"):
        raise ValueError(f"{label} must be an RFC 3339 UTC timestamp ending in Z")
    try:
        parsed = datetime.fromisoformat(value[:-1] + "+00:00")
    except ValueError as exc:
        raise ValueError(f"{label} is not a valid timestamp") from exc
    if parsed.utcoffset() != timedelta(0):
        raise ValueError(f"{label} must use UTC")
    return parsed.astimezone(timezone.utc)


def build_aggregate_approval_bindings(
    *,
    repository_root: Path,
    catalogue_path: Path,
    ledger_path: Path,
    readiness_contract_path: Path,
    parity_report: dict[str, Any],
    commercial_readiness_report: dict[str, Any],
    manifest_factory_report: dict[str, Any],
) -> dict[str, Any]:
    """Build the exact non-circular digest set a final approval must bind."""

    root = repository_root.resolve()
    catalogue_relative = _repository_relative_file(catalogue_path, root, "competitor catalogue")
    ledger_relative = _repository_relative_file(ledger_path, root, "parity ledger")
    readiness_relative = _repository_relative_file(
        readiness_contract_path, root, "commercial-readiness contract"
    )
    if catalogue_relative != CATALOGUE_RELATIVE_PATH:
        raise ValueError(f"competitor catalogue path must be {CATALOGUE_RELATIVE_PATH}")
    if ledger_relative != LEDGER_RELATIVE_PATH:
        raise ValueError(f"parity ledger path must be {LEDGER_RELATIVE_PATH}")
    if readiness_relative != READINESS_RELATIVE_PATH:
        raise ValueError(f"commercial-readiness contract path must be {READINESS_RELATIVE_PATH}")

    generator_paths = {
        "parity": root / PARITY_VALIDATOR_PATH,
        "commercial": root / COMMERCIAL_VALIDATOR_PATH,
        "method_manifest": root / METHOD_MANIFEST_VALIDATOR_PATH,
        "method_manifest_schema": root / METHOD_MANIFEST_SCHEMA_PATH,
    }
    for label, path in generator_paths.items():
        _repository_relative_file(path, root, f"{label} validation source")

    manifest_directory = root / MANIFEST_DIRECTORY
    manifest_paths = sorted(manifest_directory.glob("*.manifest.json"))
    if not manifest_paths:
        raise ValueError("canonical method-manifest set is empty")
    manifest_files = [
        {
            "path": path.resolve().relative_to(root).as_posix(),
            "sha256": _sha256_file(path),
        }
        for path in manifest_paths
    ]

    return {
        "competitor_catalogue": {
            "kind": "file",
            "path": catalogue_relative,
            "sha256": _sha256_file(catalogue_path),
        },
        "parity_ledger": {
            "kind": "file",
            "path": ledger_relative,
            "sha256": _sha256_file(ledger_path),
        },
        "parity_report": {
            "kind": "derived_report",
            "source": LEDGER_RELATIVE_PATH,
            "generator": PARITY_VALIDATOR_PATH,
            "generator_sha256": _sha256_file(generator_paths["parity"]),
            "sha256": _derived_report_sha256(parity_report, root),
        },
        "commercial_readiness_contract": {
            "kind": "file",
            "path": readiness_relative,
            "sha256": _sha256_file(readiness_contract_path),
        },
        "commercial_readiness_report": {
            "kind": "derived_report",
            "source": READINESS_RELATIVE_PATH,
            "generator": COMMERCIAL_VALIDATOR_PATH,
            "generator_sha256": _sha256_file(generator_paths["commercial"]),
            "sha256": _derived_report_sha256(commercial_readiness_report, root),
        },
        "method_manifest_set": {
            "kind": "file_set",
            "directory": MANIFEST_DIRECTORY,
            "files": manifest_files,
            "sha256": hashlib.sha256(_canonical_json_bytes(manifest_files)).hexdigest(),
        },
        "method_manifest_report": {
            "kind": "derived_report",
            "source_directory": MANIFEST_DIRECTORY,
            "generator": METHOD_MANIFEST_VALIDATOR_PATH,
            "generator_sha256": _sha256_file(generator_paths["method_manifest"]),
            "schema": METHOD_MANIFEST_SCHEMA_PATH,
            "schema_sha256": _sha256_file(generator_paths["method_manifest_schema"]),
            "sha256": _derived_report_sha256(manifest_factory_report, root),
        },
    }


def validate_aggregate_approval(
    envelope_path: Path,
    *,
    repository_root: Path,
    catalogue_path: Path,
    ledger_path: Path,
    readiness_contract_path: Path,
    parity_report: dict[str, Any],
    commercial_readiness_report: dict[str, Any],
    manifest_factory_report: dict[str, Any],
    now: datetime | None = None,
) -> dict[str, Any]:
    """Validate a final approval without allowing any input to hash itself."""

    if not envelope_path.is_file():
        return {
            "present": False,
            "passed": False,
            "pending": True,
            "errors": [],
        }

    errors: list[str] = []
    try:
        envelope = load_json(envelope_path)
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as exc:
        return {
            "present": True,
            "passed": False,
            "pending": False,
            "errors": [f"cannot load aggregate approval: {type(exc).__name__}: {exc}"],
        }
    if not isinstance(envelope, dict):
        return {
            "present": True,
            "passed": False,
            "pending": False,
            "errors": ["aggregate approval root must be an object"],
        }

    required_keys = {
        "schema_version",
        "approval_id",
        "target_release",
        "catalogue_snapshot_date",
        "hash_algorithm",
        "approved",
        "approved_by",
        "assembled_at_utc",
        "approved_at_utc",
        "bindings",
    }
    if set(envelope) != required_keys:
        errors.append("aggregate approval keys differ from schema version 1")
    if envelope.get("schema_version") != 1:
        errors.append("aggregate approval schema_version must equal 1")
    if envelope.get("approval_id") != "quickpls_3_competitor_3_0_0_final":
        errors.append("aggregate approval_id is invalid")
    if envelope.get("target_release") != "3.0.0":
        errors.append("aggregate target_release must equal 3.0.0")
    if envelope.get("catalogue_snapshot_date") != CATALOGUE_DATE:
        errors.append("aggregate catalogue_snapshot_date is invalid")
    if envelope.get("hash_algorithm") != "sha256":
        errors.append("aggregate hash_algorithm must equal sha256")
    if envelope.get("approved") is not True:
        errors.append("aggregate approved must be true")
    if not _nonempty_string(envelope.get("approved_by")):
        errors.append("aggregate approved_by must be a non-empty string")

    assembled_at: datetime | None = None
    approved_at: datetime | None = None
    try:
        assembled_at = _parse_utc_timestamp(
            envelope.get("assembled_at_utc"), "aggregate assembled_at_utc"
        )
        approved_at = _parse_utc_timestamp(
            envelope.get("approved_at_utc"), "aggregate approved_at_utc"
        )
        if approved_at <= assembled_at:
            errors.append("aggregate approval must postdate digest assembly")
        validation_time = (now or datetime.now(timezone.utc)).astimezone(timezone.utc)
        if assembled_at > validation_time + MAX_APPROVAL_FUTURE_SKEW:
            errors.append("aggregate assembly timestamp is in the future")
        if approved_at > validation_time + MAX_APPROVAL_FUTURE_SKEW:
            errors.append("aggregate approval timestamp is in the future")
    except ValueError as exc:
        errors.append(str(exc))

    if parity_report.get("passed") is not True:
        errors.append("aggregate approval cannot bind a failed parity report")
    if commercial_readiness_report.get("release_ready") is not True:
        errors.append("aggregate approval cannot bind commercial readiness before release_ready")
    if manifest_factory_report.get("passed") is not True:
        errors.append("aggregate approval cannot bind a failed method-manifest report")

    try:
        catalogue = load_json(catalogue_path)
        reverified_on = catalogue.get("catalogue_snapshot", {}).get("reverified_on")
        if assembled_at is not None:
            try:
                reverified_date = datetime.strptime(reverified_on, "%Y-%m-%d").date()
                if assembled_at.date() < reverified_date:
                    errors.append("aggregate assembly predates catalogue reverification")
            except (TypeError, ValueError):
                errors.append("catalogue reverified_on is invalid for aggregate approval")

        readiness_contract = load_json(readiness_contract_path)
        decision = readiness_contract.get("release_decision", {})
        if decision.get("status") != "approved":
            errors.append("aggregate approval requires an approved commercial release decision")
        else:
            commercial_approved_at = _parse_utc_timestamp(
                decision.get("approved_at"), "commercial release_decision.approved_at"
            )
            if assembled_at is not None and assembled_at < commercial_approved_at:
                errors.append("aggregate assembly must not predate commercial release approval")

        if assembled_at is not None:
            for manifest_path in sorted(
                (repository_root.resolve() / MANIFEST_DIRECTORY).glob("*.manifest.json")
            ):
                method_manifest = load_json(manifest_path)
                frozen_at = _parse_utc_timestamp(
                    method_manifest.get("governance", {}).get("contract_frozen_at_utc"),
                    f"{manifest_path.name} contract_frozen_at_utc",
                )
                if assembled_at < frozen_at:
                    errors.append(
                        f"aggregate assembly predates method manifest {manifest_path.name}"
                    )
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError, AttributeError) as exc:
        errors.append(f"cannot validate aggregate input timestamps: {type(exc).__name__}: {exc}")

    try:
        expected_bindings = build_aggregate_approval_bindings(
            repository_root=repository_root,
            catalogue_path=catalogue_path,
            ledger_path=ledger_path,
            readiness_contract_path=readiness_contract_path,
            parity_report=parity_report,
            commercial_readiness_report=commercial_readiness_report,
            manifest_factory_report=manifest_factory_report,
        )
        bindings = envelope.get("bindings")
        if not isinstance(bindings, dict):
            errors.append("aggregate bindings must be an object")
        else:
            actual_ids = set(bindings)
            required_ids = set(APPROVAL_BINDING_IDS)
            if actual_ids != required_ids:
                errors.append(
                    "aggregate binding IDs differ from the closed set "
                    f"(missing={sorted(required_ids - actual_ids)}, "
                    f"extra={sorted(actual_ids - required_ids)})"
                )
            for binding_id in APPROVAL_BINDING_IDS:
                if binding_id in bindings and bindings[binding_id] != expected_bindings[binding_id]:
                    errors.append(f"aggregate binding mismatch: {binding_id}")
    except (OSError, UnicodeError, json.JSONDecodeError, TypeError, ValueError) as exc:
        errors.append(f"cannot derive aggregate bindings: {type(exc).__name__}: {exc}")

    return {
        "present": True,
        "passed": not errors,
        "pending": False,
        "approval_id": envelope.get("approval_id"),
        "approved_by": envelope.get("approved_by"),
        "approved_at_utc": envelope.get("approved_at_utc"),
        "binding_ids": sorted(envelope.get("bindings", {}))
        if isinstance(envelope.get("bindings"), dict)
        else [],
        "errors": errors,
    }


def _nonempty_string(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _validate_repository_path(value: Any, root: Path, context: str, errors: list[str]) -> None:
    if not _nonempty_string(value):
        errors.append(f"{context}: evidence path must be a non-empty string")
        return
    path = Path(value)
    if path.is_absolute() or ".." in path.parts:
        errors.append(f"{context}: evidence path must be repository-relative: {value!r}")
        return
    if not (root / path).is_file():
        errors.append(f"{context}: implementation evidence is missing: {value}")


def _dependency_errors(methods: list[dict[str, Any]]) -> list[str]:
    errors: list[str] = []
    by_id = {method.get("id"): method for method in methods if _nonempty_string(method.get("id"))}
    release_index = {release: index for index, release in enumerate(TARGET_RELEASES)}

    for method in methods:
        method_id = method.get("id", "<missing>")
        dependencies = method.get("dependencies")
        if not isinstance(dependencies, list):
            errors.append(f"{method_id}: dependencies must be a list")
            continue
        string_dependencies = [dependency for dependency in dependencies if isinstance(dependency, str)]
        if len(string_dependencies) != len(dependencies):
            errors.append(f"{method_id}: dependencies must contain only strings")
        if len(string_dependencies) != len(set(string_dependencies)):
            errors.append(f"{method_id}: dependencies contain duplicates")
        for dependency in string_dependencies:
            if dependency == method_id:
                errors.append(f"{method_id}: method cannot depend on itself")
            elif dependency not in by_id:
                errors.append(f"{method_id}: unknown dependency {dependency!r}")
            elif method.get("target_release") != "current":
                dependency_target = by_id[dependency].get("target_release")
                method_target = method.get("target_release")
                if dependency_target in release_index and method_target in release_index:
                    if release_index[dependency_target] > release_index[method_target]:
                        errors.append(
                            f"{method_id}: dependency {dependency!r} targets later release {dependency_target}"
                        )

    visiting: set[str] = set()
    visited: set[str] = set()

    def visit(method_id: str, trail: list[str]) -> None:
        if method_id in visiting:
            cycle = trail[trail.index(method_id):] + [method_id]
            errors.append(f"dependency cycle detected: {' -> '.join(cycle)}")
            return
        if method_id in visited or method_id not in by_id:
            return
        visiting.add(method_id)
        trail.append(method_id)
        dependencies = by_id[method_id].get("dependencies", [])
        if isinstance(dependencies, list):
            for dependency in dependencies:
                if isinstance(dependency, str):
                    visit(dependency, trail)
        trail.pop()
        visiting.remove(method_id)
        visited.add(method_id)

    for method_id in by_id:
        visit(method_id, [])
    return errors


def validate_program_document(
    document: Any,
    parity_report: Any,
    repository_root: Path,
    *,
    commercial_readiness_report: Any = None,
    manifest_factory_report: Any = None,
    aggregate_approval_report: Any = None,
) -> dict[str, Any]:
    errors: list[str] = []
    if not isinstance(document, dict):
        return {"passed": False, "competitor_ready": False, "errors": ["manifest must be a JSON object"]}
    if not isinstance(parity_report, dict):
        return {"passed": False, "competitor_ready": False, "errors": ["evidence-backed parity report is required"]}

    if document.get("schema_version") != 1:
        errors.append("schema_version must equal 1")
    if document.get("program_id") != "quickpls_3_competitor_program":
        errors.append("program_id must equal quickpls_3_competitor_program")

    snapshot = document.get("catalogue_snapshot")
    if not isinstance(snapshot, dict):
        errors.append("catalogue_snapshot must be an object")
        snapshot = {}
    if snapshot.get("url") != CATALOGUE_URL:
        errors.append("catalogue_snapshot.url does not match the frozen official catalogue URL")
    if snapshot.get("entry_count") != len(EXPECTED_CATALOGUE):
        errors.append(f"catalogue_snapshot.entry_count must equal {len(EXPECTED_CATALOGUE)}")
    if snapshot.get("date") != CATALOGUE_DATE:
        errors.append(f"catalogue_snapshot.date must equal canonical snapshot {CATALOGUE_DATE}")
    if snapshot.get("reverified_on") != "2026-08-13":
        errors.append("catalogue_snapshot.reverified_on must equal 2026-08-13")

    if parity_report.get("passed") is not True:
        errors.append("evidence-backed parity validation did not pass")
        for error in parity_report.get("errors", []):
            if isinstance(error, str):
                errors.append(f"parity: {error}")
    parity_snapshot = parity_report.get("catalogue_snapshot")
    if not isinstance(parity_snapshot, dict):
        errors.append("parity report catalogue_snapshot must be an object")
        parity_snapshot = {}
    if parity_snapshot.get("date") != CATALOGUE_DATE:
        errors.append("parity report catalogue snapshot date differs from the competitor catalogue")
    if parity_snapshot.get("url") != CATALOGUE_URL:
        errors.append("parity report catalogue snapshot URL differs from the competitor catalogue")

    if document.get("allowed_statuses") != list(ALLOWED_STATUSES):
        errors.append("allowed_statuses differs from the closed status vocabulary")
    if document.get("allowed_priorities") != list(ALLOWED_PRIORITIES):
        errors.append("allowed_priorities differs from the closed priority vocabulary")

    target_releases = document.get("target_releases")
    if not isinstance(target_releases, list):
        errors.append("target_releases must be a list")
        target_releases = []
    release_ids = [item.get("id") for item in target_releases if isinstance(item, dict)]
    if release_ids != list(TARGET_RELEASES) or len(target_releases) != len(TARGET_RELEASES):
        errors.append("target_releases differs from the closed ordered release train")
    for index, release in enumerate(target_releases):
        if not isinstance(release, dict) or not _nonempty_string(release.get("objective")):
            errors.append(f"target_releases[{index}] requires a non-empty objective")

    methods = document.get("methods")
    if not isinstance(methods, list):
        errors.append("methods must be a list")
        methods = []
    if len(methods) != len(EXPECTED_CATALOGUE):
        errors.append(f"methods must contain exactly {len(EXPECTED_CATALOGUE)} entries")

    actual_catalogue = []
    method_ids: list[str] = []
    linked_capabilities = set().union(*EXPECTED_CAPABILITY_MAPPING.values())
    parity_features = parity_report.get("features")
    if not isinstance(parity_features, list):
        errors.append("evidence-backed parity report features must be a list")
        parity_features = []
    parity_by_id = {
        feature.get("id"): feature
        for feature in parity_features
        if isinstance(feature, dict) and _nonempty_string(feature.get("id"))
    }

    for index, method in enumerate(methods):
        context = f"methods[{index}]"
        if not isinstance(method, dict):
            errors.append(f"{context}: method must be an object")
            continue
        method_id = method.get("id")
        if isinstance(method_id, str):
            method_ids.append(method_id)
        actual_catalogue.append((method_id, method.get("official_family"), method.get("official_method")))
        if not isinstance(method_id, str) or not METHOD_ID.fullmatch(method_id):
            errors.append(f"{context}: invalid method id {method_id!r}")
        if method.get("catalogue_position") != index + 1:
            errors.append(f"{method_id}: catalogue_position must equal {index + 1}")

        status = method.get("status")
        priority = method.get("priority")
        target = method.get("target_release")
        if status not in ALLOWED_STATUSES:
            errors.append(f"{method_id}: unknown status {status!r}")
        if priority not in ALLOWED_PRIORITIES:
            errors.append(f"{method_id}: unknown priority {priority!r}")
        if target not in TARGET_RELEASES:
            errors.append(f"{method_id}: unknown target release {target!r}")
        if status == "release-qualified" and target != "current":
            errors.append(f"{method_id}: release-qualified methods must target current")
        if status != "release-qualified" and target == "current":
            errors.append(f"{method_id}: only release-qualified methods may target current")
        if status == "deferred" and (target != "post-3.0" or method.get("competitor_scope") is not False):
            errors.append(f"{method_id}: deferred methods must target post-3.0 and leave competitor_scope")
        elif status != "deferred" and method.get("competitor_scope") is not True:
            errors.append(f"{method_id}: non-deferred methods must remain in competitor_scope")

        for field in ("official_family", "official_method", "quickpls_scope", "remaining_gap"):
            if not _nonempty_string(method.get(field)):
                errors.append(f"{method_id}: {field} must be a non-empty string")

        capabilities = method.get("quickpls_capability_ids")
        evidence = method.get("implementation_evidence")
        if not isinstance(capabilities, list) or any(not _nonempty_string(item) for item in capabilities):
            errors.append(f"{method_id}: quickpls_capability_ids must be a string list")
            capabilities = []
        if len(capabilities) != len(set(capabilities)):
            errors.append(f"{method_id}: quickpls_capability_ids contains duplicates")
        if not isinstance(evidence, list):
            errors.append(f"{method_id}: implementation_evidence must be a list")
            evidence = []

        expected_capabilities = EXPECTED_CAPABILITY_MAPPING.get(method_id, frozenset())
        actual_capabilities = frozenset(capabilities)
        if actual_capabilities != expected_capabilities:
            errors.append(
                f"{method_id}: capability mapping differs from frozen crosswalk "
                f"(expected={sorted(expected_capabilities)}, actual={sorted(actual_capabilities)})"
            )

        if expected_capabilities:
            if evidence:
                errors.append(f"{method_id}: mapped statuses must rely on parity evidence, not preview evidence")
            mapped_statuses: list[str] = []
            for capability in sorted(expected_capabilities):
                feature = parity_by_id.get(capability)
                if feature is None:
                    errors.append(f"{method_id}: capability is missing from evidence-backed parity report: {capability}")
                    continue
                # Never consult declared_state here.  The parity evaluator derives
                # this value from current engine/archive/native/release evidence.
                mapped = LEDGER_TO_PROGRAM_STATUS.get(feature.get("derived_state"))
                if mapped is None:
                    errors.append(
                        f"{method_id}: capability has invalid evidence-derived state: "
                        f"{capability}={feature.get('derived_state')!r}"
                    )
                else:
                    mapped_statuses.append(mapped)
            if mapped_statuses:
                derived = min(mapped_statuses, key=STATUS_RANK.__getitem__)
                if status != derived:
                    errors.append(
                        f"{method_id}: declared {status} contradicts evidence-derived {derived}"
                    )
        elif status == "engine-preview":
            if not evidence:
                errors.append(f"{method_id}: engine-preview requires implementation evidence")
            for evidence_index, value in enumerate(evidence):
                _validate_repository_path(value, repository_root, f"{method_id}.implementation_evidence[{evidence_index}]", errors)
        elif status in {"absent", "deferred"}:
            if evidence:
                errors.append(f"{method_id}: {status} must not declare implementation evidence")
        elif status in {"native-qualified", "release-qualified"} and not expected_capabilities:
            errors.append(f"{method_id}: {status} requires a frozen parity-capability mapping")

    if len(method_ids) != len(set(method_ids)):
        errors.append("method IDs must be unique")
    if tuple(actual_catalogue) != EXPECTED_CATALOGUE:
        errors.append("method order, identifiers, families, or official names differ from the frozen catalogue")

    parity_ids = set(parity_by_id)
    missing_ledger_links = sorted(parity_ids - linked_capabilities)
    unknown_ledger_links = sorted(linked_capabilities - parity_ids)
    if missing_ledger_links:
        errors.append(f"parity-ledger capabilities missing from crosswalk: {missing_ledger_links}")
    if unknown_ledger_links:
        errors.append(f"crosswalk capabilities missing from parity ledger: {unknown_ledger_links}")

    errors.extend(_dependency_errors([method for method in methods if isinstance(method, dict)]))

    claim_gate = document.get("competitor_claim_gate")
    if not isinstance(claim_gate, dict):
        errors.append("competitor_claim_gate must be an object")
    else:
        expected_gate_keys = {
            "required_method_status",
            "applies_to",
            "commercial_readiness_contract",
            "method_manifest_directory",
            "aggregate_approval_envelope",
            "aggregate_hash_algorithm",
            "aggregate_required_bindings",
            "readiness_rule",
        }
        if set(claim_gate) != expected_gate_keys:
            errors.append("competitor_claim_gate keys differ from the fail-closed gate schema")
        if claim_gate.get("required_method_status") != "release-qualified":
            errors.append("competitor_claim_gate.required_method_status must equal release-qualified")
        if claim_gate.get("commercial_readiness_contract") != "validation/quickpls_3_release_readiness.json":
            errors.append("competitor_claim_gate must reference the canonical commercial-readiness contract")
        if claim_gate.get("method_manifest_directory") != "validation/methods":
            errors.append("competitor_claim_gate must reference the canonical method-manifest directory")
        if claim_gate.get("aggregate_approval_envelope") != APPROVAL_RELATIVE_PATH:
            errors.append("competitor_claim_gate must reference the canonical aggregate approval envelope")
        if claim_gate.get("aggregate_hash_algorithm") != "sha256":
            errors.append("competitor_claim_gate aggregate_hash_algorithm must equal sha256")
        if claim_gate.get("aggregate_required_bindings") != list(APPROVAL_BINDING_IDS):
            errors.append("competitor_claim_gate aggregate binding list differs from the closed set")
        if not _nonempty_string(claim_gate.get("applies_to")) or not _nonempty_string(claim_gate.get("readiness_rule")):
            errors.append("competitor_claim_gate descriptions must be non-empty")

    commercial_valid = isinstance(commercial_readiness_report, dict)
    commercial_release_ready = False
    commercial_pending: list[str] = []
    commercial_failed: list[str] = []
    if not commercial_valid:
        errors.append("validated commercial-readiness report is required")
    else:
        if commercial_readiness_report.get("structurally_valid") is not True:
            errors.append("commercial-readiness validation did not pass")
        if commercial_readiness_report.get("target_release") != "3.0.0":
            errors.append("commercial-readiness target_release must equal 3.0.0")
        commercial_release_ready = commercial_readiness_report.get("release_ready") is True
        commercial_pending = [
            value for value in commercial_readiness_report.get("pending", [])
            if isinstance(value, str)
        ]
        commercial_failed = [
            value for value in commercial_readiness_report.get("failed", [])
            if isinstance(value, str)
        ]

    factory_valid = isinstance(manifest_factory_report, dict)
    factory_results: list[dict[str, Any]] = []
    if not factory_valid:
        errors.append("validated method-manifest factory report is required")
    else:
        if manifest_factory_report.get("passed") is not True:
            errors.append("method-manifest factory validation did not pass")
            for error in manifest_factory_report.get("errors", []):
                if isinstance(error, str):
                    errors.append(f"method manifest: {error}")
        candidate_results = manifest_factory_report.get("manifests")
        if not isinstance(candidate_results, list) or not candidate_results:
            errors.append("method-manifest factory report must contain validated manifests")
        else:
            factory_results = [result for result in candidate_results if isinstance(result, dict)]
            if len(factory_results) != len(candidate_results):
                errors.append("method-manifest factory results must be objects")
            for result in factory_results:
                if result.get("catalogue_snapshot_date") != CATALOGUE_DATE:
                    errors.append(
                        f"method manifest {result.get('feature_id', result.get('path'))!r} "
                        "catalogue snapshot date differs from the competitor catalogue"
                    )

    factory_by_feature = {
        result.get("feature_id"): result
        for result in factory_results
        if _nonempty_string(result.get("feature_id"))
    }
    missing_method_manifests = sorted(linked_capabilities - set(factory_by_feature))
    non_release_method_manifests = sorted(
        feature_id
        for feature_id in linked_capabilities & set(factory_by_feature)
        if factory_by_feature[feature_id].get("derived_state") != "release_qualified"
    )

    aggregate_present = (
        isinstance(aggregate_approval_report, dict)
        and aggregate_approval_report.get("present") is True
    )
    aggregate_passed = (
        aggregate_present and aggregate_approval_report.get("passed") is True
    )
    aggregate_errors: list[str] = []
    if aggregate_present and not aggregate_passed:
        aggregate_errors = [
            error for error in aggregate_approval_report.get("errors", [])
            if isinstance(error, str)
        ]
        errors.append("aggregate competitor approval validation did not pass")
        errors.extend(f"aggregate approval: {error}" for error in aggregate_errors)

    status_counts = Counter(
        method.get("status") if isinstance(method.get("status"), str) else "<invalid>"
        for method in methods if isinstance(method, dict)
    )
    priority_counts = Counter(
        method.get("priority") if isinstance(method.get("priority"), str) else "<invalid>"
        for method in methods if isinstance(method, dict)
    )
    release_counts = Counter(
        method.get("target_release") if isinstance(method.get("target_release"), str) else "<invalid>"
        for method in methods if isinstance(method, dict)
    )
    competitor_methods = [
        method for method in methods
        if isinstance(method, dict) and method.get("competitor_scope") is True
    ]
    competitor_ready = (
        not errors
        and bool(competitor_methods)
        and all(method.get("status") == "release-qualified" for method in competitor_methods)
        and commercial_release_ready
        and not missing_method_manifests
        and not non_release_method_manifests
        and aggregate_passed
    )
    return {
        "passed": not errors,
        "competitor_ready": competitor_ready,
        "catalogue_snapshot_date": snapshot.get("date"),
        "method_count": len(methods),
        "competitor_scope_count": len(competitor_methods),
        "status_counts": dict(sorted(status_counts.items())),
        "priority_counts": dict(sorted(priority_counts.items())),
        "target_release_counts": {
            release: release_counts.get(release, 0) for release in TARGET_RELEASES
        },
        "parity_evidence_passed": parity_report.get("passed") is True,
        "commercial_release_ready": commercial_release_ready,
        "pending_non_method_gates": commercial_pending,
        "failed_non_method_gates": commercial_failed,
        "method_manifest_factory_passed": (
            isinstance(manifest_factory_report, dict)
            and manifest_factory_report.get("passed") is True
        ),
        "method_manifest_count": len(factory_results),
        "missing_method_manifests": missing_method_manifests,
        "non_release_method_manifests": non_release_method_manifests,
        "aggregate_approval_present": aggregate_present,
        "aggregate_approval_passed": aggregate_passed,
        "aggregate_approval_errors": aggregate_errors,
        "errors": errors,
    }


def validate_program(
    manifest_path: Path,
    ledger_path: Path,
    repository_root: Path,
    *,
    readiness_contract_path: Path = DEFAULT_READINESS_CONTRACT,
    method_manifest_paths: list[Path] | None = None,
    approval_envelope_path: Path = DEFAULT_APPROVAL_ENVELOPE,
) -> dict[str, Any]:
    try:
        document = load_json(manifest_path)
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as exc:
        return {"passed": False, "competitor_ready": False, "errors": [f"manifest load failed: {type(exc).__name__}: {exc}"]}
    try:
        # Strict parsing is a preflight only.  Capability state is derived below
        # by parity_ledger.validate_ledger from this actual path.
        load_json(ledger_path)
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as exc:
        return {"passed": False, "competitor_ready": False, "errors": [f"parity ledger load failed: {type(exc).__name__}: {exc}"]}

    parity_report = parity_ledger.validate_ledger(ledger_path, repository_root)
    try:
        commercial_report = quickpls_3_release_readiness.load_and_validate(
            readiness_contract_path,
            repository_root=repository_root,
        )
    except (OSError, quickpls_3_release_readiness.ContractError) as exc:
        commercial_report = {
            "structurally_valid": False,
            "release_ready": False,
            "target_release": None,
            "pending": [],
            "failed": [],
            "errors": [f"{type(exc).__name__}: {exc}"],
        }
    factory_report = method_promotion_manifest.validate_all(
        method_manifest_paths,
        repository_root,
    )
    aggregate_report = validate_aggregate_approval(
        approval_envelope_path,
        repository_root=repository_root,
        catalogue_path=manifest_path,
        ledger_path=ledger_path,
        readiness_contract_path=readiness_contract_path,
        parity_report=parity_report,
        commercial_readiness_report=commercial_report,
        manifest_factory_report=factory_report,
    )
    return validate_program_document(
        document,
        parity_report,
        repository_root,
        commercial_readiness_report=commercial_report,
        manifest_factory_report=factory_report,
        aggregate_approval_report=aggregate_report,
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--ledger", type=Path, default=DEFAULT_LEDGER)
    parser.add_argument("--readiness-contract", type=Path, default=DEFAULT_READINESS_CONTRACT)
    parser.add_argument("--approval-envelope", type=Path, default=DEFAULT_APPROVAL_ENVELOPE)
    parser.add_argument("--repository-root", type=Path, default=ROOT)
    args = parser.parse_args(argv)
    report = validate_program(
        args.manifest,
        args.ledger,
        args.repository_root,
        readiness_contract_path=args.readiness_contract,
        approval_envelope_path=args.approval_envelope,
    )
    print(json.dumps(report, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
