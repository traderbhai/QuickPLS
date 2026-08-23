#!/usr/bin/env python3
"""Manifest-derived packaged Windows acceptance contract.

The ordered required-check IDs are authoritative. Counts are derived only for
display and receipt compatibility; a matching count cannot substitute for an
exact ID-set comparison.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path
from typing import Any, Mapping


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CONTRACT_PATH = (
    ROOT / "validation/capabilities/packaged_windows_acceptance_v2.manifest.json"
)
BUNDLED_SAMPLE_CATALOG_PATH = ROOT / "src/data/bundledSampleProjects.v1.json"
BUNDLED_SAMPLE_CATALOG_REPOSITORY_PATH = "src/data/bundledSampleProjects.v1.json"
CHECK_ID = re.compile(r"^[A-Za-z][A-Za-z0-9_]*$")
TOKEN = re.compile(r"^[a-z][a-z0-9_]*$")


class PackagedAcceptanceContractError(ValueError):
    pass


def _reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise PackagedAcceptanceContractError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_packaged_acceptance_contract(
    path: Path | str = DEFAULT_CONTRACT_PATH,
) -> dict[str, Any]:
    source = Path(path)
    try:
        value = json.loads(
            source.read_text(encoding="utf-8"),
            object_pairs_hook=_reject_duplicate_keys,
            parse_constant=lambda token: (_ for _ in ()).throw(
                PackagedAcceptanceContractError(f"non-finite JSON value: {token}")
            ),
        )
    except (OSError, json.JSONDecodeError) as exc:
        raise PackagedAcceptanceContractError(str(exc)) from exc
    if not isinstance(value, dict):
        raise PackagedAcceptanceContractError("contract must be a JSON object")
    validate_packaged_acceptance_contract(value)
    return value


def validate_packaged_acceptance_contract(contract: Mapping[str, Any]) -> None:
    expected_keys = {
        "schema_version",
        "contract_id",
        "contract_version",
        "final_scope",
        "ordered_check_sets",
        "phase2_release_required_check_ids",
    }
    if set(contract) != expected_keys:
        raise PackagedAcceptanceContractError("contract fields are not exact")
    if contract.get("schema_version") != 2:
        raise PackagedAcceptanceContractError("schema_version must equal 2")
    if contract.get("contract_id") != "quickpls.packaged_windows_acceptance.v2":
        raise PackagedAcceptanceContractError("contract_id is invalid")
    if not isinstance(contract.get("contract_version"), str):
        raise PackagedAcceptanceContractError("contract_version is missing")
    final_scope = contract.get("final_scope")
    if not isinstance(final_scope, str) or not TOKEN.fullmatch(final_scope):
        raise PackagedAcceptanceContractError("final_scope is invalid")

    check_sets = contract.get("ordered_check_sets")
    if not isinstance(check_sets, list) or not check_sets:
        raise PackagedAcceptanceContractError("ordered_check_sets must be non-empty")
    set_ids: list[str] = []
    scopes: list[str] = []
    checks: list[str] = []
    for index, check_set in enumerate(check_sets):
        if not isinstance(check_set, Mapping) or set(check_set) != {
            "id",
            "scope",
            "required_check_ids",
        }:
            raise PackagedAcceptanceContractError(
                f"ordered_check_sets[{index}] fields are not exact"
            )
        set_id = check_set.get("id")
        scope = check_set.get("scope")
        values = check_set.get("required_check_ids")
        if not isinstance(set_id, str) or not TOKEN.fullmatch(set_id):
            raise PackagedAcceptanceContractError(f"ordered_check_sets[{index}].id is invalid")
        if not isinstance(scope, str) or not TOKEN.fullmatch(scope):
            raise PackagedAcceptanceContractError(
                f"ordered_check_sets[{index}].scope is invalid"
            )
        if not isinstance(values, list) or not values:
            raise PackagedAcceptanceContractError(
                f"ordered_check_sets[{index}].required_check_ids must be non-empty"
            )
        for value in values:
            if not isinstance(value, str) or not CHECK_ID.fullmatch(value):
                raise PackagedAcceptanceContractError(f"invalid required check ID: {value!r}")
        set_ids.append(set_id)
        scopes.append(scope)
        checks.extend(values)
    if len(set_ids) != len(set(set_ids)):
        raise PackagedAcceptanceContractError("check-set IDs must be unique")
    if len(scopes) != len(set(scopes)):
        raise PackagedAcceptanceContractError("check-set scopes must be unique")
    if final_scope != scopes[-1]:
        raise PackagedAcceptanceContractError("final_scope must equal the final check-set scope")
    if len(checks) != len(set(checks)):
        raise PackagedAcceptanceContractError("required check IDs must be globally unique")

    phase2 = contract.get("phase2_release_required_check_ids")
    if (
        not isinstance(phase2, list)
        or not phase2
        or any(not isinstance(value, str) or not CHECK_ID.fullmatch(value) for value in phase2)
        or len(phase2) != len(set(phase2))
    ):
        raise PackagedAcceptanceContractError(
            "phase2_release_required_check_ids must be a unique non-empty check-ID list"
        )
    unknown = sorted(set(phase2) - set(checks))
    if unknown:
        raise PackagedAcceptanceContractError(
            f"Phase-2 checks are not in the full contract: {', '.join(unknown)}"
        )


def required_check_ids(contract: Mapping[str, Any]) -> tuple[str, ...]:
    validate_packaged_acceptance_contract(contract)
    return tuple(
        check_id
        for check_set in contract["ordered_check_sets"]
        for check_id in check_set["required_check_ids"]
    )


def phase2_release_required_check_ids(contract: Mapping[str, Any]) -> tuple[str, ...]:
    validate_packaged_acceptance_contract(contract)
    return tuple(contract["phase2_release_required_check_ids"])


def validate_required_report_checks(
    contract: Mapping[str, Any], checks: Any
) -> dict[str, Any]:
    expected = required_check_ids(contract)
    actual = tuple(checks) if isinstance(checks, Mapping) else ()
    missing = tuple(check_id for check_id in expected if check_id not in actual)
    unexpected = tuple(check_id for check_id in actual if check_id not in expected)
    order_matches = actual == expected
    return {
        "passed": isinstance(checks, Mapping) and not missing and not unexpected,
        "expected_count": len(expected),
        "actual_count": len(actual),
        "missing": missing,
        "unexpected": unexpected,
        "order_matches": order_matches,
    }


CONTRACT = load_packaged_acceptance_contract()
EXPECTED_CHECK_IDS = required_check_ids(CONTRACT)
EXPECTED_CHECK_COUNT = len(EXPECTED_CHECK_IDS)
PHASE2_RELEASE_CHECK_IDS = phase2_release_required_check_ids(CONTRACT)
CONTRACT_REPOSITORY_PATH = "validation/capabilities/packaged_windows_acceptance_v2.manifest.json"
CONTRACT_FILE_SHA256 = hashlib.sha256(DEFAULT_CONTRACT_PATH.read_bytes()).hexdigest()
CONTRACT_FILE_SIZE = DEFAULT_CONTRACT_PATH.stat().st_size
BUNDLED_SAMPLE_CATALOG_SHA256 = hashlib.sha256(BUNDLED_SAMPLE_CATALOG_PATH.read_bytes()).hexdigest()
BUNDLED_SAMPLE_CATALOG_SIZE = BUNDLED_SAMPLE_CATALOG_PATH.stat().st_size


def bundled_sample_catalog_descriptor() -> dict[str, Any]:
    return {
        "path": BUNDLED_SAMPLE_CATALOG_REPOSITORY_PATH,
        "size": BUNDLED_SAMPLE_CATALOG_SIZE,
        "sha256": BUNDLED_SAMPLE_CATALOG_SHA256,
    }


def packaged_acceptance_contract_descriptor() -> dict[str, Any]:
    return {
        "path": CONTRACT_REPOSITORY_PATH,
        "size": CONTRACT_FILE_SIZE,
        "sha256": CONTRACT_FILE_SHA256,
        "bundled_sample_catalog": bundled_sample_catalog_descriptor(),
    }


def receipt_binds_packaged_acceptance_contract(receipt: Any) -> bool:
    if not isinstance(receipt, Mapping):
        return False
    descriptor = receipt.get("acceptance_contract")
    return (
        receipt.get("schema_version") == 2
        and receipt.get("checks") == EXPECTED_CHECK_COUNT
        and receipt.get("unique_checks") == EXPECTED_CHECK_COUNT
        and receipt.get("final_scope") == CONTRACT["final_scope"]
        and isinstance(descriptor, Mapping)
        and descriptor.get("path") == CONTRACT_REPOSITORY_PATH
        and descriptor.get("contract_id") == CONTRACT["contract_id"]
        and descriptor.get("contract_version") == CONTRACT["contract_version"]
        and descriptor.get("required_check_count") == EXPECTED_CHECK_COUNT
        and descriptor.get("sha256") == CONTRACT_FILE_SHA256
        and descriptor.get("bundled_sample_catalog") == bundled_sample_catalog_descriptor()
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--contract", type=Path, default=DEFAULT_CONTRACT_PATH)
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    try:
        contract = load_packaged_acceptance_contract(args.contract)
        report = {
            "passed": True,
            "contract_id": contract["contract_id"],
            "contract_version": contract["contract_version"],
            "required_check_count": len(required_check_ids(contract)),
            "phase2_required_check_count": len(
                phase2_release_required_check_ids(contract)
            ),
        }
        if args.report is not None:
            value = json.loads(args.report.read_text(encoding="utf-8"))
            check_report = validate_required_report_checks(contract, value.get("checks"))
            report["report_check_contract"] = check_report
            report["passed"] = check_report["passed"]
    except (OSError, json.JSONDecodeError, PackagedAcceptanceContractError) as exc:
        report = {"passed": False, "errors": [str(exc)]}
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
