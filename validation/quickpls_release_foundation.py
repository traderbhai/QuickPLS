"""Validate the unsigned-preview release foundation without weakening launch gates."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

try:
    from validation.package_release_artifacts import (
        EXPECTED_CHANNEL_POLICY,
        read_release_channel_contract,
        read_version_contract,
    )
    from validation.webview2_offline_containment import validate_webview2_offline_containment
except ModuleNotFoundError:  # Direct execution from the validation directory.
    from package_release_artifacts import (  # type: ignore[no-redef]
        EXPECTED_CHANNEL_POLICY,
        read_release_channel_contract,
        read_version_contract,
    )
    from webview2_offline_containment import validate_webview2_offline_containment  # type: ignore[no-redef]


ROOT = Path(__file__).resolve().parents[1]
COMMERCIAL_CHANNELS = {
    "internal": "maintainer_only",
    "beta": "signed_prerelease",
    "stable": "all_mandatory_gates_passed",
}


def _read_json(path: Path, label: str) -> dict[str, Any]:
    def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f"duplicate key {key!r}")
            result[key] = value
        return result

    def reject_non_finite(value: str) -> None:
        raise ValueError(f"non-finite numeric constant {value!r}")

    try:
        document = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=reject_duplicate_keys,
            parse_constant=reject_non_finite,
        )
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as error:
        raise SystemExit(f"Cannot read {label} at {path}: {error}") from error
    if not isinstance(document, dict):
        raise SystemExit(f"{label} must contain a JSON object: {path}")
    return document


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def validate_release_foundation(root: Path = ROOT) -> dict[str, Any]:
    """Prove channel separation, offline NSIS configuration, and stable-gate binding."""

    root = root.resolve()
    version, version_evidence = read_version_contract(root)
    channel_contract = read_release_channel_contract(root, expected_version=version)

    unsigned_channels = sorted(
        name
        for name, policy in EXPECTED_CHANNEL_POLICY.items()
        if policy["artifact_factory"] == "unsigned_preview"
    )
    signed_channels = sorted(
        name
        for name, policy in EXPECTED_CHANNEL_POLICY.items()
        if policy["artifact_factory"] == "signed_candidate"
    )
    _require(unsigned_channels == ["internal", "unsigned-preview"], "Unsigned channel set changed")
    _require(signed_channels == ["beta", "stable"], "Signed channel set changed")
    for name in unsigned_channels:
        policy = EXPECTED_CHANNEL_POLICY[name]
        _require(not policy["authenticode_required"], f"{name} has contradictory signing policy")
        _require(policy["competitor_claims_policy"] == "prohibited", f"{name} cannot authorize competitor claims")
        _require(policy["commercial_channel"] is None, f"{name} cannot be a commercial channel")
    for name in signed_channels:
        _require(
            bool(EXPECTED_CHANNEL_POLICY[name]["authenticode_required"]),
            f"{name} must require Authenticode",
        )
    _require(
        EXPECTED_CHANNEL_POLICY["beta"]["competitor_claims_policy"] == "prohibited",
        "Beta cannot authorize competitor claims",
    )
    _require(
        EXPECTED_CHANNEL_POLICY["stable"]["competitor_claims_policy"] == "commercial_gate_required",
        "Stable competitor claims must remain conditional on the commercial gate",
    )

    tauri = _read_json(root / "src-tauri" / "tauri.conf.json", "Tauri configuration")
    bundle = tauri.get("bundle")
    _require(isinstance(bundle, dict), "Tauri configuration must define bundle")
    _require(bundle.get("active") is True, "Tauri bundling must remain active")
    targets = bundle.get("targets")
    _require(targets == "nsis" or targets == ["nsis"], "Windows release bundle target must be NSIS")
    windows = bundle.get("windows")
    _require(isinstance(windows, dict), "Tauri bundle.windows configuration is required")
    _require(
        windows.get("webviewInstallMode") == {"type": "offlineInstaller", "silent": True},
        "Tauri must embed the silent WebView2 offline installer",
    )
    _require(windows.get("allowDowngrades") is False, "Tauri installer downgrades must be blocked")
    webview2_containment = validate_webview2_offline_containment(root)

    readiness_path = root / channel_contract["commercial_readiness_contract"]
    readiness = _read_json(readiness_path, "QuickPLS 3 commercial readiness contract")
    _require(readiness.get("target_channel") == "stable", "Commercial target channel must remain stable")
    _require(
        readiness.get("release_channels") == COMMERCIAL_CHANNELS,
        "Commercial release channels must retain the frozen signed-beta/stable policy",
    )
    requirements = readiness.get("requirements")
    _require(isinstance(requirements, list), "Commercial readiness requirements must be a list")
    requirement_map = {
        item.get("id"): item
        for item in requirements
        if isinstance(item, dict) and isinstance(item.get("id"), str)
    }
    for requirement_id in ("signing.identity", "signing.artifacts", "governance.claims_channels"):
        requirement = requirement_map.get(requirement_id)
        _require(isinstance(requirement, dict), f"Missing commercial requirement {requirement_id}")
        _require(requirement.get("required") is True, f"Commercial requirement {requirement_id} must remain mandatory")

    return {
        "schema_version": 1,
        "target": "QuickPLS release engineering foundation",
        "passed": True,
        "version": version,
        "version_contract": version_evidence,
        "default_artifact_channel": channel_contract["default_artifact_channel"],
        "unsigned_artifact_channels": unsigned_channels,
        "signed_candidate_channels": signed_channels,
        "offline_installer": {
            "bundle_target": "nsis",
            "webview2_install_mode": "offlineInstaller",
            "silent": True,
            "downgrades_allowed": False,
        },
        "webview2_offline_containment": webview2_containment,
        "commercial_gate": {
            "target_channel": "stable",
            "beta_requires_authenticode": True,
            "stable_requires_authenticode": True,
            "mandatory_signing_requirements": ["signing.identity", "signing.artifacts"],
        },
    }


def main() -> None:
    print(json.dumps(validate_release_foundation(), indent=2))


if __name__ == "__main__":
    main()
