from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Iterable

ROOT = Path(__file__).resolve().parents[2]
RESULTS = ROOT / "validation" / "results"

QUICKPLS_PACKAGES = [
    "qpls-assessment",
    "qpls-cli",
    "qpls-core",
    "qpls-data",
    "qpls-estimation",
    "qpls-project",
    "qpls-resampling",
    "qpls-runner",
    "quickpls-desktop",
]

FORBIDDEN_MOJIBAKE = ["RÂ²", "RÃ", "Ã‚", "Ãƒ", "ï¿½", "�"]
FORBIDDEN_SMARTPLS_EQUIVALENCE = [
    "identical to smartpls",
    "smartpls equivalent",
    "equivalent to smartpls",
]


def read_text(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8-sig")


def read_json(relative: str):
    return json.loads(read_text(relative))


def write_result(name: str, payload: dict) -> None:
    RESULTS.mkdir(parents=True, exist_ok=True)
    (RESULTS / name).write_text(json.dumps(payload, indent=2), encoding="utf-8")


def source_bundle(paths: Iterable[str]) -> str:
    return "\n".join(read_text(path) for path in paths if (ROOT / path).exists())


def no_forbidden_tokens(text: str, tokens: Iterable[str]) -> bool:
    return not any(token in text for token in tokens)


def no_smartpls_equivalence(text: str) -> bool:
    lowered = text.lower()
    return not any(phrase in lowered for phrase in FORBIDDEN_SMARTPLS_EQUIVALENCE)


def quickpls_lock_versions_are(cargo_lock: str, version: str) -> bool:
    return all(f'name = "{name}"\nversion = "{version}"' in cargo_lock for name in QUICKPLS_PACKAGES)


def shared_v2_metadata_checks(*, version: str, target_stage: str, expected_label: str, package: dict, package_lock: dict, registry: dict, cargo: str, cargo_lock: str, tauri: dict, roadmap: str, topbar: str) -> dict[str, bool]:
    return {
        f"current stage is {target_stage}": registry.get("current_stage") == target_stage,
        f"registry contains {target_stage} slice": any(s.get("id") == target_stage and s.get("status") == "validated" for s in registry.get("slices", [])),
        f"package version is {version}": package.get("version") == version,
        f"package lock root version is {version}": package_lock.get("version") == version and package_lock.get("packages", {}).get("", {}).get("version") == version,
        f"cargo workspace version is {version}": f'version = "{version}"' in cargo,
        f"quickpls lock versions are {version}": quickpls_lock_versions_are(cargo_lock, version),
        f"tauri version is {version}": tauri.get("version") == version,
        f"roadmap expects {target_stage}": target_stage in roadmap,
        f"topbar shows {version} label": expected_label in topbar,
        f"artifact label is {target_stage}": target_stage in package["scripts"].get("qpls:release:artifacts", ""),
    }


def command_bar_contract_checks(topbar: str) -> dict[str, bool]:
    return {
        "command bar metadata present": all(token in topbar for token in [
            "command-run-cluster",
            "data-command-bar-state",
            "data-run-state",
            "data-run-method",
            "data-run-blocker-id",
            "data-run-blocker-view",
            "data-run-disabled-reason",
            "data-run-blocker-action",
        ]),
        "run disabled reason remains nearby": "aria-describedby={!activeJob && !canRun ? \"run-disabled-reason\" : undefined}" in topbar and "aria-label={runDisabledLabel}" in topbar,
        "blocker navigation keeps destination context": "coachId: \"top-command-bar\"" in topbar and "actionLabel: `Run blocker: ${topBlocker.label}`" in topbar,
    }


def frontend_boundary_check(audit_source: str) -> bool:
    return not re.search(r"crates/qpls-(estimation|assessment|resampling)/src", audit_source)
