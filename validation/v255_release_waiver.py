"""Exact, release-scoped QuickPLS 2.55 evidence-waiver authority.

This module does not turn the waived requirement into a passing assertion.  It
only recognizes the one product-owner-approved exception and is shared by the
collector, verifier, and final product audit so no second case can inherit it.
"""

from __future__ import annotations

from typing import Any


DPI_WAIVER_CASE_ID = (
    "cross_method:accessibility:actual Windows 200 percent scaling"
)
DPI_WAIVER_OPERATION = "exercise_accessibility"
DPI_WAIVER_ASSERTION_ID = f"{DPI_WAIVER_OPERATION}:{DPI_WAIVER_CASE_ID}"
DPI_WAIVER_CROSS_REPORT_SUITE = "quickpls_v255_cross_method_candidate_wrapper_v1"
DPI_WAIVER_METADATA: dict[str, str] = {
    "waiver_authority": "product_owner",
    "waiver_date": "2026-08-22",
    "reason": (
        "product owner explicitly authorized ignoring the 200 percent scaling "
        "requirement"
    ),
}
DPI_WAIVER_EXPECTED: dict[str, Any] = {
    "effective_dpi": 192,
    "device_pixel_ratio": 2,
    "clean_profile": True,
    "forced_scale_argument_present": False,
}
DPI_WAIVER_CONTRACT: dict[str, Any] = {
    "case_id": DPI_WAIVER_CASE_ID,
    "status": "waived",
    **DPI_WAIVER_METADATA,
    "expected": DPI_WAIVER_EXPECTED,
    "all_other_cases_must_pass": True,
}
DPI_WAIVER_MANIFEST_DECLARATION: dict[str, Any] = {
    "case_id": DPI_WAIVER_CASE_ID,
    "status": "waived",
    **DPI_WAIVER_METADATA,
}


def exact_approved_waiver_contract(value: object) -> bool:
    return value == DPI_WAIVER_CONTRACT


def exact_waiver_metadata(value: object) -> bool:
    return value == DPI_WAIVER_METADATA


def exact_waiver_observed(value: object) -> bool:
    return (
        isinstance(value, dict)
        and set(value) == set(DPI_WAIVER_EXPECTED)
        and isinstance(value.get("effective_dpi"), int)
        and not isinstance(value.get("effective_dpi"), bool)
        and value["effective_dpi"] > 0
        and value["effective_dpi"] != 192
        and isinstance(value.get("device_pixel_ratio"), (int, float))
        and not isinstance(value.get("device_pixel_ratio"), bool)
        and value["device_pixel_ratio"] > 0
        and value.get("clean_profile") is True
        and value.get("forced_scale_argument_present") is False
        and value != DPI_WAIVER_EXPECTED
    )


def exact_waived_observation(value: object) -> bool:
    if not isinstance(value, dict):
        return False
    assertion = value.get("assertion")
    observed = assertion.get("observed") if isinstance(assertion, dict) else None
    return (
        value.get("schema_version") == 1
        and value.get("case_id") == DPI_WAIVER_CASE_ID
        and value.get("operation") == DPI_WAIVER_OPERATION
        and value.get("status") == "waived"
        and exact_waiver_metadata(value.get("waiver"))
        and isinstance(assertion, dict)
        and assertion.get("id") == DPI_WAIVER_ASSERTION_ID
        and assertion.get("passed") is False
        and assertion.get("expected") == DPI_WAIVER_EXPECTED
        and exact_waiver_observed(observed)
    )


def exact_release_waiver_receipt(value: object) -> bool:
    if not isinstance(value, dict):
        return False
    return (
        set(value)
        == {
            "case_id",
            "status",
            "assertion_passed",
            "waiver_authority",
            "waiver_date",
            "reason",
            "expected",
            "observed",
        }
        and value.get("case_id") == DPI_WAIVER_CASE_ID
        and value.get("status") == "waived"
        and value.get("assertion_passed") is False
        and all(value.get(key) == expected for key, expected in DPI_WAIVER_METADATA.items())
        and value.get("expected") == DPI_WAIVER_EXPECTED
        and exact_waiver_observed(value.get("observed"))
    )


def exact_release_waiver_matches_observation(
    receipt: object, observation: object
) -> bool:
    if not exact_release_waiver_receipt(receipt) or not exact_waived_observation(
        observation
    ):
        return False
    assert isinstance(receipt, dict)
    assert isinstance(observation, dict)
    assertion = observation["assertion"]
    assert isinstance(assertion, dict)
    return receipt == {
        "case_id": observation["case_id"],
        "status": observation["status"],
        "assertion_passed": assertion["passed"],
        **observation["waiver"],
        "expected": assertion["expected"],
        "observed": assertion["observed"],
    }


def exact_case_waiver_receipt_matches_observation(
    receipt: object, observation: object
) -> bool:
    if not isinstance(receipt, dict) or not exact_waived_observation(observation):
        return False
    assert isinstance(observation, dict)
    assertion = observation["assertion"]
    screenshot = observation.get("screenshot")
    receipt_screenshot = receipt.get("screenshot")
    return (
        receipt.get("status") == "waived"
        and receipt.get("case_id") == observation["case_id"]
        and receipt.get("operation") == observation["operation"]
        and receipt.get("waiver") == observation["waiver"]
        and receipt.get("assertion") == assertion
        and isinstance(screenshot, dict)
        and isinstance(screenshot.get("sha256"), str)
        and isinstance(receipt_screenshot, dict)
        and receipt_screenshot.get("sha256") == screenshot["sha256"]
    )


def exact_cross_report_waiver_binding(
    payload: object, observation: object
) -> bool:
    if not isinstance(payload, dict) or not exact_waived_observation(observation):
        return False
    assert isinstance(observation, dict)
    assertion = observation["assertion"]
    assert isinstance(assertion, dict)
    expected = assertion["expected"]
    observed = assertion["observed"]
    assert isinstance(expected, dict)
    assert isinstance(observed, dict)
    release_waivers = payload.get("release_waivers")
    dpi = payload.get("dpi")
    return (
        payload.get("schema_version") == 1
        and payload.get("suite_id") == DPI_WAIVER_CROSS_REPORT_SUITE
        and payload.get("target_release") == "2.55.0"
        and payload.get("passed") is True
        and payload.get("qualification_status") == "passed_with_waiver"
        and isinstance(release_waivers, list)
        and len(release_waivers) == 1
        and exact_release_waiver_matches_observation(
            release_waivers[0], observation
        )
        and isinstance(dpi, dict)
        and set(dpi)
        == {
            "requirement_status",
            "effective_dpi",
            "required_dpi",
            "device_pixel_ratio",
            "display_settings_changed",
            "forced_scale_argument_present",
            "profile_was_fresh",
        }
        and dpi.get("requirement_status") == "waived"
        and dpi.get("effective_dpi") == observed["effective_dpi"]
        and dpi.get("required_dpi") == expected["effective_dpi"]
        and dpi.get("device_pixel_ratio") == observed["device_pixel_ratio"]
        and dpi.get("display_settings_changed") is False
        and dpi.get("forced_scale_argument_present")
        is observed["forced_scale_argument_present"]
        and dpi.get("profile_was_fresh") is observed["clean_profile"]
    )


def exact_waived_index_entry(value: object, *, require_artifacts: bool) -> bool:
    if not isinstance(value, dict):
        return False
    if (
        value.get("id") != DPI_WAIVER_CASE_ID
        or value.get("status") != "waived"
        or not exact_waiver_metadata(value.get("waiver"))
    ):
        return False
    if not require_artifacts:
        return True
    screenshot = value.get("screenshot")
    receipt = value.get("receipt")
    return (
        isinstance(screenshot, dict)
        and isinstance(screenshot.get("member"), str)
        and isinstance(screenshot.get("sha256"), str)
        and isinstance(receipt, dict)
        and isinstance(receipt.get("member"), str)
        and isinstance(receipt.get("sha256"), str)
        and isinstance(receipt.get("binding"), dict)
        and receipt["binding"].get("json_pointer") == "/case_id"
        and receipt["binding"].get("expected_value") == DPI_WAIVER_CASE_ID
    )


def exact_population_status(entries: object, index_status: object) -> bool:
    if not isinstance(entries, list):
        return False
    waived = [entry for entry in entries if isinstance(entry, dict) and entry.get("status") == "waived"]
    verified = [entry for entry in entries if isinstance(entry, dict) and entry.get("status") == "verified"]
    return (
        index_status == "verified_with_waiver"
        and len(entries) == 55
        and len(verified) == 54
        and len(waived) == 1
        and exact_waived_index_entry(waived[0], require_artifacts=True)
    )
