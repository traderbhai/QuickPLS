"""Validate QuickPLS functional-offline claims and the WebView2 boundary.

This governance check intentionally separates application behavior from the
complete packaged process tree. QuickPLS analytical workflows and app/page code
do not require remote services, while a Microsoft-managed WebView2 process may
still make background connections. The stronger process-tree claim therefore
remains blocked until a separate OS-enforced fixed-runtime gate passes.
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
READINESS = Path("validation/quickpls_3_release_readiness.json")
EXTERNAL_BETA = Path("validation/quickpls_external_beta.json")
RELEASE_CHANNELS = Path("validation/quickpls_release_channels.json")
GOVERNED_DOCS = (
    Path("README.md"),
    Path("docs/INSTALLATION.md"),
    Path("docs/WEBVIEW2_OFFLINE_BOUNDARY.md"),
    Path("docs/QUICKPLS_3_COMMERCIAL_READINESS.md"),
    Path("docs/QUICKPLS_3_COMPETITOR_PROGRAM.md"),
)

EXPECTED_PROHIBITED_CLAIMS = [
    "complete_smartpls_parity",
    "identical_undocumented_behavior",
    "smartpls_project_compatibility",
    "smartpls_affiliation",
    "fully_offline_without_os_enforced_fixed_webview2_containment",
    "no_telemetry_without_os_enforced_fixed_webview2_containment",
    "zero_egress_without_os_enforced_fixed_webview2_containment",
]
EXPECTED_STRICT_GATE = {
    "status": "pending",
    "required_control": "os_enforced_fixed_webview2_runtime_containment",
    "application_level_containment_sufficient": False,
    "evidence": [],
}
EXPECTED_BETA_CLAIMS = {
    "allowed_positioning": "closed_signed_beta_testing_only",
    "functional_offline_statement": "analytical_workflows_require_no_internet_account_or_cloud",
    "application_network_statement": "quickpls_application_and_page_make_no_external_requests",
    "strict_process_tree_claim": "prohibited_pending_os_enforced_fixed_webview2_containment",
}
EXPECTED_CHANNEL_CLAIMS = {
    "internal": "prohibited",
    "unsigned-preview": "prohibited",
    "beta": "prohibited",
    "stable": "commercial_gate_required",
}
REQUIRED_DOC_FRAGMENTS = {
    Path("README.md"): (
        "analytical workflows require no internet connection, account, or cloud service",
        "QuickPLS application and page make no external requests",
        "Microsoft-managed WebView2 runtime can still make its own background service connections",
        "not a claim that the complete WebView2 process tree has zero egress",
    ),
    Path("docs/INSTALLATION.md"): (
        "analytical workflows require no internet connection, account, or cloud service",
        "QuickPLS application and page make no external requests",
        "not a literal fully-offline, no-telemetry, or zero-egress process-tree claim",
        "OS-enforced fixed-WebView2 network boundary",
    ),
    Path("docs/WEBVIEW2_OFFLINE_BOUNDARY.md"): (
        "QuickPLS application and page make no external requests",
        "strict zero-egress acceptance red unless an OS-enforced boundary proves it",
        "It must not make a literal \"fully offline\", \"no telemetry\", or \"zero egress\" process-tree claim",
        "stricter architecture and gate remain pending",
    ),
    Path("docs/QUICKPLS_3_COMMERCIAL_READINESS.md"): (
        "workflows require no internet connection, account, or cloud service",
        "QuickPLS application/page makes no external requests",
        "literal fully-offline, no-telemetry, or zero-egress process-tree claim",
        "current candidate has not satisfied that stronger condition",
    ),
    Path("docs/QUICKPLS_3_COMPETITOR_PROGRAM.md"): (
        "analytical workflows require no internet connection, account, or cloud service",
        "application/page makes no external requests",
        "literal fully-offline, no-telemetry, or zero-process-egress claim",
        "OS-enforced fixed-WebView2 containment gate passes",
    ),
}
UNQUALIFIED_STRICT_CLAIMS = (
    re.compile(r"\bQuickPLS\s+(?:is|is marketed as|is positioned as)\s+(?:completely\s+)?fully[- ]offline\b", re.I),
    re.compile(r"\bQuickPLS\s+(?:has|provides|guarantees)\s+zero[- ]egress\b", re.I),
    re.compile(r"\bQuickPLS\s+(?:has|sends|collects)\s+no\s+telemetry\b", re.I),
)


class OfflineClaimError(ValueError):
    """Raised when a release or documentation claim crosses the frozen boundary."""


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise OfflineClaimError(message)


def _strict_json(path: Path) -> dict[str, Any]:
    def pairs(rows: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in rows:
            if key in result:
                raise OfflineClaimError(f"duplicate JSON key in {path}: {key}")
            result[key] = value
        return result

    try:
        value = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=pairs,
            parse_constant=lambda token: (_ for _ in ()).throw(
                OfflineClaimError(f"non-finite JSON token in {path}: {token}")
            ),
        )
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise OfflineClaimError(f"cannot read claim contract {path}: {error}") from error
    _require(isinstance(value, dict), f"claim contract must be a JSON object: {path}")
    return value


def _normalized_text(path: Path) -> str:
    try:
        return " ".join(path.read_text(encoding="utf-8").split())
    except (OSError, UnicodeError) as error:
        raise OfflineClaimError(f"cannot read governed claim document {path}: {error}") from error


def validate_offline_claims(root: Path = ROOT) -> dict[str, Any]:
    root = root.resolve()
    readiness = _strict_json(root / READINESS)
    beta = _strict_json(root / EXTERNAL_BETA)
    channels = _strict_json(root / RELEASE_CHANNELS)

    product = readiness.get("product_policy")
    _require(isinstance(product, dict), "commercial readiness product_policy is missing")
    _require(
        product.get("functional_offline_scope")
        == "analytical_workflows_require_no_internet_account_or_cloud",
        "commercial readiness lost the bounded functional-offline fact",
    )
    _require(
        product.get("application_network_behavior")
        == "quickpls_application_and_page_make_no_external_requests",
        "commercial readiness lost the QuickPLS application/page boundary",
    )
    _require(
        product.get("webview2_runtime_boundary")
        == "microsoft_managed_background_service_connections_may_occur",
        "commercial readiness must disclose possible Microsoft-managed WebView2 connections",
    )
    _require(
        product.get("strict_zero_egress_claim_gate") == EXPECTED_STRICT_GATE,
        "strict zero-egress claim gate must remain pending and OS-enforced",
    )
    _require(
        product.get("prohibited_claims") == EXPECTED_PROHIBITED_CLAIMS,
        "commercial readiness prohibited-claims list drifted",
    )

    privacy_requirement = next(
        (
            row
            for row in readiness.get("requirements", [])
            if isinstance(row, dict) and row.get("id") == "trust.privacy_telemetry"
        ),
        None,
    )
    _require(isinstance(privacy_requirement, dict), "trust.privacy_telemetry requirement is missing")
    _require(
        "trust.privacy_telemetry.webview2_claim_boundary"
        in privacy_requirement.get("acceptance_check_ids", []),
        "commercial readiness must retain the release-blocking WebView2 claim-boundary check",
    )

    _require(beta.get("claim_policy") == EXPECTED_BETA_CLAIMS, "external beta claim policy drifted")
    channel_rows = channels.get("channels")
    _require(isinstance(channel_rows, dict), "release channel rows are missing")
    actual_channel_claims = {
        name: row.get("competitor_claims_policy") if isinstance(row, dict) else None
        for name, row in channel_rows.items()
    }
    _require(
        actual_channel_claims == EXPECTED_CHANNEL_CLAIMS,
        "release channels must keep competitor claims prohibited outside the commercial stable gate",
    )

    for relative in GOVERNED_DOCS:
        text = _normalized_text(root / relative)
        for fragment in REQUIRED_DOC_FRAGMENTS[relative]:
            _require(fragment in text, f"{relative.as_posix()} lost required bounded-claim text: {fragment}")
        if relative != Path("docs/WEBVIEW2_OFFLINE_BOUNDARY.md"):
            for pattern in UNQUALIFIED_STRICT_CLAIMS:
                _require(
                    pattern.search(text) is None,
                    f"{relative.as_posix()} contains an unqualified strict offline claim: {pattern.pattern}",
                )

    return {
        "schema_version": 1,
        "passed": True,
        "functional_offline_claim": "authorized",
        "quickpls_app_page_external_requests": False,
        "strict_process_tree_claim": "prohibited",
        "strict_gate_status": "pending",
        "required_control": EXPECTED_STRICT_GATE["required_control"],
        "governed_documents": [path.as_posix() for path in GOVERNED_DOCS],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=ROOT)
    args = parser.parse_args()
    try:
        report = validate_offline_claims(args.repo_root)
    except OfflineClaimError as error:
        print(json.dumps({"passed": False, "errors": [str(error)]}, indent=2))
        return 1
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
