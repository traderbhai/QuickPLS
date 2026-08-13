#!/usr/bin/env python3
"""Validate the pre-certificate QuickPLS 3 operational policy foundation."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path, PurePosixPath
from typing import Any
from urllib.parse import urlparse


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CONTRACT = Path(__file__).with_name("quickpls_3_operational_foundation.json")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
SEMVER = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+$")

EXPECTED_DOCUMENTS = {
    "support_policy",
    "version_support_policy",
    "rollback_recovery",
    "known_issues",
    "security_model",
    "privacy_notice",
    "diagnostics_logging",
    "supply_chain_policy",
}
EXPECTED_ROUTES = {
    "public_support": "public_no_confidential_data",
    "private_security": "private_security_reports",
}
EXPECTED_CONTROLS = {
    "support.channel_policy": ({"support.operations"}, "policy_defined"),
    "support.version_policy": ({"support.operations", "support.docs_diagnostics"}, "policy_defined"),
    "support.rollback_known_issues": ({"support.operations"}, "policy_defined"),
    "diagnostics.bundle_contract": ({"support.docs_diagnostics"}, "implementation_required"),
    "docs.versioned_release_set": ({"support.docs_diagnostics"}, "implementation_required"),
    "privacy.offline_network": ({"trust.privacy_telemetry"}, "implementation_required"),
    "security.threat_model": ({"trust.security"}, "external_review_required"),
    "legal.release_review": ({"trust.legal"}, "external_review_required"),
    "supply_chain.sbom_licenses": ({"supply_chain.sbom_licenses"}, "implementation_required"),
    "supply_chain.provenance": ({"supply_chain.provenance"}, "implementation_required"),
}
EXPECTED_PRIVACY_DEFAULTS = {
    "offline_processing": True,
    "telemetry_enabled": False,
    "launch_network_request": False,
    "diagnostics_user_initiated": True,
    "diagnostics_auto_upload": False,
}
ALLOWED_STATES = {"policy_defined", "implementation_required", "external_review_required"}


class ContractError(ValueError):
    """Raised when the operational foundation contract is invalid."""


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise ContractError(message)


def _nonempty_string(value: object, label: str) -> str:
    _require(isinstance(value, str) and bool(value.strip()), f"{label} must be a non-empty string")
    return value.strip()


def _reject_constant(value: str) -> None:
    raise ContractError(f"non-finite JSON constant is not allowed: {value}")


def _unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in pairs:
        if key in result:
            raise ContractError(f"duplicate JSON key is not allowed: {key}")
        result[key] = value
    return result


def load_contract(path: Path) -> dict[str, Any]:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as error:
        raise ContractError(f"cannot read operational contract: {error}") from error
    try:
        value = json.loads(text, object_pairs_hook=_unique_object, parse_constant=_reject_constant)
    except ContractError:
        raise
    except json.JSONDecodeError as error:
        raise ContractError(f"operational contract is malformed JSON: {error}") from error
    _require(isinstance(value, dict), "operational contract must be a JSON object")
    return value


def _safe_path(value: object, label: str) -> PurePosixPath:
    text = _nonempty_string(value, label).replace("\\", "/")
    path = PurePosixPath(text)
    _require(not path.is_absolute(), f"{label} must be repository-relative")
    _require(".." not in path.parts, f"{label} must not traverse outside the repository")
    _require(path.parts and path.parts[0] == "docs", f"{label} must identify a durable docs artifact")
    return path


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _string_list(value: object, label: str) -> list[str]:
    _require(isinstance(value, list) and bool(value), f"{label} must be a non-empty array")
    result = [_nonempty_string(item, f"{label} item") for item in value]
    _require(len(result) == len(set(result)), f"{label} entries must be unique")
    return result


def validate_contract(contract: dict[str, Any], *, repository_root: Path = ROOT) -> dict[str, Any]:
    expected_keys = {
        "schema_version",
        "program",
        "scope",
        "target_release",
        "support_routes",
        "privacy_defaults",
        "documents",
        "controls",
        "release_claim",
    }
    _require(set(contract) == expected_keys, "operational contract top-level keys differ from the frozen schema")
    _require(contract["schema_version"] == 1, "schema_version must be 1")
    _require(contract["program"] == "quickpls_3_operational_foundation", "program is invalid")
    _require(contract["scope"] == "pre_certificate_operational_foundation", "scope is invalid")
    target_release = _nonempty_string(contract["target_release"], "target_release")
    _require(bool(SEMVER.fullmatch(target_release)), "target_release must be stable semantic version")

    routes = contract["support_routes"]
    _require(isinstance(routes, list) and len(routes) == len(EXPECTED_ROUTES), "support_routes are incomplete")
    route_ids: list[str] = []
    for index, route in enumerate(routes):
        label = f"support_routes[{index}]"
        _require(isinstance(route, dict), f"{label} must be an object")
        _require(set(route) == {"id", "url", "privacy"}, f"{label} keys are invalid")
        route_id = _nonempty_string(route["id"], f"{label}.id")
        route_ids.append(route_id)
        _require(route_id in EXPECTED_ROUTES, f"{label}.id is unknown")
        _require(route["privacy"] == EXPECTED_ROUTES[route_id], f"{label}.privacy is invalid")
        parsed = urlparse(_nonempty_string(route["url"], f"{label}.url"))
        _require(parsed.scheme == "https" and parsed.netloc == "github.com", f"{label}.url must be an HTTPS GitHub route")
        _require(parsed.path.startswith("/traderbhai/QuickPLS/"), f"{label}.url must target the official QuickPLS repository")
    _require(len(route_ids) == len(set(route_ids)) and set(route_ids) == set(EXPECTED_ROUTES), "support route IDs differ from the frozen set")

    _require(contract["privacy_defaults"] == EXPECTED_PRIVACY_DEFAULTS, "privacy defaults differ from the offline product contract")

    documents = contract["documents"]
    _require(isinstance(documents, list) and bool(documents), "documents must be a non-empty array")
    document_ids: list[str] = []
    document_paths: list[str] = []
    for index, document in enumerate(documents):
        label = f"documents[{index}]"
        _require(isinstance(document, dict), f"{label} must be an object")
        _require(set(document) == {"id", "path", "size", "sha256", "required_headings"}, f"{label} keys are invalid")
        document_id = _nonempty_string(document["id"], f"{label}.id")
        relative = _safe_path(document["path"], f"{label}.path")
        size = document["size"]
        digest = _nonempty_string(document["sha256"], f"{label}.sha256")
        _require(isinstance(size, int) and not isinstance(size, bool) and size > 0, f"{label}.size must be a positive integer")
        _require(bool(SHA256.fullmatch(digest)), f"{label}.sha256 must be lowercase 64-hex")
        headings = _string_list(document["required_headings"], f"{label}.required_headings")
        path = (repository_root / Path(*relative.parts)).resolve()
        root = repository_root.resolve()
        _require(path.is_relative_to(root), f"{label}.path resolves outside the repository")
        _require(path.is_file(), f"{label}.path does not exist: {relative.as_posix()}")
        _require(path.stat().st_size == size, f"{label}.size does not match the document bytes")
        _require(_sha256(path) == digest, f"{label}.sha256 does not match the document bytes")
        lines = {line.rstrip() for line in path.read_text(encoding="utf-8").splitlines()}
        for heading in headings:
            _require(heading in lines, f"{label} is missing required heading: {heading}")
        document_ids.append(document_id)
        document_paths.append(relative.as_posix())
    _require(len(document_ids) == len(set(document_ids)), "document IDs must be unique")
    _require(len(document_paths) == len(set(document_paths)), "document paths must be unique")
    _require(set(document_ids) == EXPECTED_DOCUMENTS, "document IDs differ from the frozen operational set")

    controls = contract["controls"]
    _require(isinstance(controls, list) and bool(controls), "controls must be a non-empty array")
    control_ids: list[str] = []
    state_counts = {state: 0 for state in sorted(ALLOWED_STATES)}
    pending_requirements: set[str] = set()
    for index, control in enumerate(controls):
        label = f"controls[{index}]"
        _require(isinstance(control, dict), f"{label} must be an object")
        _require(
            set(control) == {"id", "state", "release_requirement_ids", "policy_document_ids", "remaining_release_evidence"},
            f"{label} keys are invalid",
        )
        control_id = _nonempty_string(control["id"], f"{label}.id")
        control_ids.append(control_id)
        _require(control_id in EXPECTED_CONTROLS, f"{label}.id is unknown")
        requirement_ids = set(_string_list(control["release_requirement_ids"], f"{label}.release_requirement_ids"))
        expected_requirements, expected_state = EXPECTED_CONTROLS[control_id]
        _require(requirement_ids == expected_requirements, f"{label}.release_requirement_ids differ from the frozen mapping")
        state = _nonempty_string(control["state"], f"{label}.state")
        _require(state in ALLOWED_STATES and state == expected_state, f"{label}.state is invalid for this pre-certificate control")
        policy_ids = _string_list(control["policy_document_ids"], f"{label}.policy_document_ids")
        _require(set(policy_ids).issubset(EXPECTED_DOCUMENTS), f"{label}.policy_document_ids contains an unknown document")
        _string_list(control["remaining_release_evidence"], f"{label}.remaining_release_evidence")
        state_counts[state] += 1
        pending_requirements.update(requirement_ids)
    _require(len(control_ids) == len(set(control_ids)), "control IDs must be unique")
    _require(set(control_ids) == set(EXPECTED_CONTROLS), "control IDs differ from the frozen operational set")

    claim = contract["release_claim"]
    _require(isinstance(claim, dict), "release_claim must be an object")
    _require(set(claim) == {"commercial_ready", "reason", "authoritative_gate"}, "release_claim keys are invalid")
    _require(claim["commercial_ready"] is False, "the operational foundation must not claim commercial readiness")
    _nonempty_string(claim["reason"], "release_claim.reason")
    _require(claim["authoritative_gate"] == "validation/quickpls_3_release_readiness.json", "release_claim.authoritative_gate is invalid")

    return {
        "structurally_valid": True,
        "foundation_ready": True,
        "commercial_ready": False,
        "target_release": target_release,
        "document_count": len(documents),
        "control_count": len(controls),
        "state_counts": state_counts,
        "pending_release_requirements": sorted(pending_requirements),
        "authoritative_commercial_gate": claim["authoritative_gate"],
        "errors": [],
    }


def load_and_validate(path: Path = DEFAULT_CONTRACT, *, repository_root: Path = ROOT) -> dict[str, Any]:
    return validate_contract(load_contract(path), repository_root=repository_root)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--contract", type=Path, default=DEFAULT_CONTRACT)
    parser.add_argument("--require-foundation", action="store_true")
    args = parser.parse_args()
    try:
        report = load_and_validate(args.contract, repository_root=ROOT)
    except ContractError as error:
        report = {
            "structurally_valid": False,
            "foundation_ready": False,
            "commercial_ready": False,
            "errors": [str(error)],
        }
        print(json.dumps(report, indent=2, sort_keys=True))
        return 1
    print(json.dumps(report, indent=2, sort_keys=True))
    if args.require_foundation and not report["foundation_ready"]:
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
