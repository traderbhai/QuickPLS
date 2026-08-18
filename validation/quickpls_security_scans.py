#!/usr/bin/env python3
"""Run and bind QuickPLS dependency, secret, and license security checks.

The command consumes machine-readable output from pinned external scanners,
applies the checked-in disposition policy, and writes one reviewable report.
It is fail-closed for known npm high/critical advisories, Python and Rust
vulnerabilities, undisposed Rust informational warnings, secret findings, and
incomplete license inventories.  Passing this command is necessary but never
sufficient for a commercial release.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import shutil
import subprocess
import sys
from datetime import date, datetime, timezone
from pathlib import Path, PurePosixPath
from typing import Any, Iterable

try:
    from validation.quickpls_supply_chain_evidence import cargo_components, npm_components
except ModuleNotFoundError:
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    from validation.quickpls_supply_chain_evidence import cargo_components, npm_components  # type: ignore[no-redef]


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_DISPOSITIONS = Path(__file__).with_name("security_scan_dispositions.json")
DEFAULT_REPORT = ROOT / "validation" / "results" / "quickpls_security_scan_report.json"
SHA256 = re.compile(r"^[0-9a-f]{64}$")
ADVISORY = re.compile(r"^RUSTSEC-[0-9]{4}-[0-9]{4}$")
ALLOWED_DISPOSITION_STATUS = {"target_excluded", "tracked_nonblocking"}
ALLOWED_RUST_KINDS = {"unmaintained", "unsound", "notice", "yanked"}
ALLOWED_NPM_SEVERITIES = {"info", "low", "moderate", "high", "critical"}
PINNED_PIP_AUDIT_VERSION = "pip-audit 2.10.1"
PINNED_DETECT_SECRETS_VERSION = "1.5.0"
PINNED_CARGO_AUDIT_VERSION = "cargo-audit 0.22.2"
APPROVED_LICENSE_TOKENS = {
    "0BSD",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "BSL-1.0",
    "CC-BY-4.0",
    "CC0-1.0",
    "ISC",
    "LicenseRef-Proprietary",
    "MIT",
    "MIT-0",
    "MPL-2.0",
    "Unicode-3.0",
    "Unlicense",
    "Zlib",
}


class ScanError(ValueError):
    """Raised when scan evidence is malformed, incomplete, or blocking."""


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise ScanError(message)


def _strict_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        _require(key not in value, f"duplicate JSON key is not allowed: {key}")
        value[key] = item
    return value


def _strict_float(token: str) -> float:
    value = float(token)
    _require(math.isfinite(value), f"non-finite JSON number is not allowed: {token}")
    return value


def load_json_bytes(payload: bytes, label: str) -> dict[str, Any]:
    try:
        value = json.loads(
            payload.decode("utf-8-sig"),
            object_pairs_hook=_strict_object,
            parse_float=_strict_float,
            parse_constant=lambda token: (_ for _ in ()).throw(
                ScanError(f"non-finite JSON number is not allowed: {token}")
            ),
        )
    except (UnicodeError, json.JSONDecodeError) as error:
        raise ScanError(f"{label} is not strict JSON: {error}") from error
    _require(isinstance(value, dict), f"{label} must be a JSON object")
    return value


def load_json(path: Path, label: str) -> dict[str, Any]:
    try:
        return load_json_bytes(path.read_bytes(), label)
    except OSError as error:
        raise ScanError(f"cannot read {label}: {error}") from error


def _descriptor(path: Path, root: Path = ROOT) -> dict[str, Any]:
    resolved = path.resolve()
    _require(resolved.is_relative_to(root.resolve()), f"scan input escapes repository: {path}")
    _require(path.is_file() and not path.is_symlink(), f"scan input is missing or linked: {path}")
    payload = path.read_bytes()
    return {
        "path": resolved.relative_to(root.resolve()).as_posix(),
        "size": len(payload),
        "sha256": hashlib.sha256(payload).hexdigest(),
    }


def validate_dispositions(document: dict[str, Any], *, today: date | None = None) -> dict[str, Any]:
    _require(
        set(document) == {"schema_version", "document_type", "policy", "rust_advisory_dispositions", "release_claim"},
        "disposition document keys differ from the frozen schema",
    )
    _require(document["schema_version"] == 1, "disposition schema_version must be 1")
    _require(document["document_type"] == "quickpls_security_scan_dispositions", "disposition document_type is invalid")
    policy = document["policy"]
    _require(
        policy
        == {
            "npm_block_severities": ["high", "critical"],
            "python_block_known_vulnerabilities": True,
            "rust_block_known_vulnerabilities": True,
            "rust_block_unsound_warnings": True,
            "secret_findings_allowed": False,
            "license_inventory_review_required": True,
        },
        "scan policy differs from the frozen fail-closed policy",
    )
    dispositions = document["rust_advisory_dispositions"]
    _require(isinstance(dispositions, list), "rust_advisory_dispositions must be an array")
    normalized: dict[str, dict[str, Any]] = {}
    review_date = today or date.today()
    for index, item in enumerate(dispositions):
        label = f"rust_advisory_dispositions[{index}]"
        _require(isinstance(item, dict), f"{label} must be an object")
        _require(set(item) == {"id", "kind", "status", "scope", "owner", "reviewed_at", "expires_at"}, f"{label} keys are invalid")
        advisory_id = item["id"]
        _require(isinstance(advisory_id, str) and bool(ADVISORY.fullmatch(advisory_id)), f"{label}.id is invalid")
        _require(advisory_id not in normalized, f"duplicate Rust advisory disposition: {advisory_id}")
        _require(item["kind"] in ALLOWED_RUST_KINDS, f"{label}.kind is invalid")
        _require(item["status"] in ALLOWED_DISPOSITION_STATUS, f"{label}.status is invalid")
        for field in ("scope", "owner"):
            _require(isinstance(item[field], str) and bool(item[field].strip()), f"{label}.{field} must be non-empty")
        try:
            reviewed = date.fromisoformat(item["reviewed_at"])
            expires = date.fromisoformat(item["expires_at"])
        except (TypeError, ValueError) as error:
            raise ScanError(f"{label} dates must be ISO-8601 calendar dates") from error
        _require(reviewed <= review_date, f"{label}.reviewed_at cannot be in the future")
        _require(expires >= review_date, f"Rust advisory disposition expired: {advisory_id}")
        _require(expires > reviewed, f"{label}.expires_at must follow reviewed_at")
        normalized[advisory_id] = item
    claim = document["release_claim"]
    _require(isinstance(claim, dict) and set(claim) == {"commercial_ready", "reason"}, "release_claim is invalid")
    _require(claim["commercial_ready"] is False, "security scan dispositions must not claim commercial readiness")
    _require(isinstance(claim["reason"], str) and bool(claim["reason"].strip()), "release_claim.reason is required")
    return {"policy": policy, "dispositions": normalized}


def evaluate_npm(report: dict[str, Any], block_severities: list[str]) -> dict[str, Any]:
    _require(report.get("auditReportVersion") == 2, "npm audit report version must be 2")
    vulnerabilities = report.get("vulnerabilities")
    metadata = report.get("metadata")
    _require(isinstance(vulnerabilities, dict) and isinstance(metadata, dict), "npm audit report is incomplete")
    summary = metadata.get("vulnerabilities")
    _require(isinstance(summary, dict), "npm audit vulnerability summary is missing")
    counts: dict[str, int] = {}
    for severity in ALLOWED_NPM_SEVERITIES:
        count = summary.get(severity)
        _require(isinstance(count, int) and not isinstance(count, bool) and count >= 0, f"npm {severity} count is invalid")
        counts[severity] = count
    _require(summary.get("total") == sum(counts.values()), "npm vulnerability summary total is inconsistent")
    normalized: list[dict[str, str]] = []
    for name, item in sorted(vulnerabilities.items()):
        _require(isinstance(name, str) and isinstance(item, dict), "npm vulnerability row is invalid")
        severity = item.get("severity")
        _require(severity in ALLOWED_NPM_SEVERITIES, f"npm vulnerability severity is invalid: {name}")
        normalized.append({"package": name, "severity": severity})
    for severity in ALLOWED_NPM_SEVERITIES:
        _require(
            counts[severity] == sum(item["severity"] == severity for item in normalized),
            f"npm {severity} summary does not match the vulnerability rows",
        )
    blocked = [item for item in normalized if item["severity"] in set(block_severities)]
    return {"passed": not blocked, "counts": counts, "blocking": blocked, "findings": normalized}


def evaluate_python(report: dict[str, Any]) -> dict[str, Any]:
    dependencies = report.get("dependencies")
    _require(isinstance(dependencies, list), "pip-audit dependencies must be an array")
    findings: list[dict[str, str]] = []
    for index, dependency in enumerate(dependencies):
        _require(isinstance(dependency, dict), f"pip-audit dependency {index} is invalid")
        name = dependency.get("name")
        version = dependency.get("version")
        vulns = dependency.get("vulns")
        _require(isinstance(name, str) and name and isinstance(version, str) and version, f"pip-audit dependency {index} identity is invalid")
        _require(isinstance(vulns, list), f"pip-audit dependency {name} vulnerabilities are invalid")
        for vulnerability in vulns:
            _require(isinstance(vulnerability, dict) and isinstance(vulnerability.get("id"), str), f"pip-audit vulnerability for {name} is invalid")
            findings.append({"package": name, "version": version, "id": vulnerability["id"]})
    return {"passed": not findings, "dependency_count": len(dependencies), "findings": findings}


def evaluate_rust(report: dict[str, Any], dispositions: dict[str, dict[str, Any]]) -> dict[str, Any]:
    vulnerabilities = report.get("vulnerabilities")
    warnings = report.get("warnings")
    database = report.get("database")
    lockfile = report.get("lockfile")
    _require(isinstance(vulnerabilities, dict) and isinstance(warnings, dict), "cargo-audit report is incomplete")
    _require(isinstance(database, dict) and isinstance(lockfile, dict), "cargo-audit metadata is incomplete")
    vulnerability_rows = vulnerabilities.get("list")
    _require(isinstance(vulnerability_rows, list), "cargo-audit vulnerability list is invalid")
    _require(vulnerabilities.get("count") == len(vulnerability_rows), "cargo-audit vulnerability count is inconsistent")
    _require(vulnerabilities.get("found") is bool(vulnerability_rows), "cargo-audit vulnerability flag is inconsistent")
    finding_ids: list[str] = []
    for item in vulnerability_rows:
        _require(isinstance(item, dict) and isinstance(item.get("advisory"), dict), "cargo-audit vulnerability row is invalid")
        advisory_id = item["advisory"].get("id")
        _require(isinstance(advisory_id, str), "cargo-audit vulnerability ID is missing")
        finding_ids.append(advisory_id)
    warning_rows: list[dict[str, str]] = []
    for kind, items in sorted(warnings.items()):
        _require(kind in ALLOWED_RUST_KINDS and isinstance(items, list), f"cargo-audit warning kind is invalid: {kind}")
        for item in items:
            _require(isinstance(item, dict) and isinstance(item.get("advisory"), dict), "cargo-audit warning row is invalid")
            advisory_id = item["advisory"].get("id")
            _require(isinstance(advisory_id, str) and bool(ADVISORY.fullmatch(advisory_id)), "cargo-audit warning ID is invalid")
            warning_rows.append({"id": advisory_id, "kind": kind})
    observed = {item["id"] for item in warning_rows}
    undisposed = sorted(
        item["id"]
        for item in warning_rows
        if item["id"] not in dispositions or dispositions[item["id"]]["kind"] != item["kind"]
    )
    undisposed.extend(
        sorted(
            item["id"]
            for item in warning_rows
            if item["kind"] == "unsound"
            and item["id"] in dispositions
            and dispositions[item["id"]]["status"] != "target_excluded"
        )
    )
    undisposed = sorted(set(undisposed))
    stale = sorted(set(dispositions) - observed)
    _require(not stale, f"Rust advisory dispositions are stale or not present in the scan: {', '.join(stale)}")
    return {
        "passed": not finding_ids and not undisposed,
        "database": {
            "advisory_count": database.get("advisory-count"),
            "last_commit": database.get("last-commit"),
            "last_updated": database.get("last-updated"),
        },
        "dependency_count": lockfile.get("dependency-count"),
        "vulnerabilities": sorted(finding_ids),
        "warnings": warning_rows,
        "undisposed_warnings": undisposed,
        "disposition_count": len(dispositions),
    }


def evaluate_secrets(report: dict[str, Any]) -> dict[str, Any]:
    _require(report.get("version") == "1.5.0", "detect-secrets report must come from pinned version 1.5.0")
    results = report.get("results")
    plugins = report.get("plugins_used")
    _require(isinstance(results, dict) and isinstance(plugins, list) and plugins, "detect-secrets report is incomplete")
    findings = sum(len(items) for items in results.values() if isinstance(items, list))
    _require(all(isinstance(items, list) for items in results.values()), "detect-secrets result rows are invalid")
    return {"passed": findings == 0, "finding_count": findings, "files": sorted(results)}


def _license_tokens(expression: str) -> set[str]:
    # The existing lockfiles contain valid SPDX-like expressions plus legacy
    # slash separators.  Token validation is deliberately conservative and the
    # resulting inventory remains subject to qualified legal review.
    return {
        token
        for token in re.findall(r"[A-Za-z0-9][A-Za-z0-9.+-]*", expression)
        if token not in {"AND", "OR", "WITH"}
    }


def evaluate_licenses(package_lock: dict[str, Any], cargo_metadata: dict[str, Any]) -> dict[str, Any]:
    npm, _ = npm_components(package_lock)
    cargo, _ = cargo_components(cargo_metadata)
    rows = sorted(
        [
        {
            "ecosystem": next(item["value"] for item in component["properties"] if item["name"] == "quickpls:ecosystem"),
            "name": component["name"],
            "version": component["version"],
            "license_expression": component["licenses"][0]["expression"],
        }
        for component in [*npm, *cargo]
        ],
        key=lambda row: (row["ecosystem"], row["name"].casefold(), row["version"], row["license_expression"]),
    )
    unknown = sorted(
        {
            token
            for row in rows
            for token in _license_tokens(row["license_expression"])
            if token not in APPROVED_LICENSE_TOKENS
        }
    )
    return {
        "passed": not unknown and bool(rows),
        "component_count": len(rows),
        "npm_count": sum(row["ecosystem"] == "npm" for row in rows),
        "cargo_count": sum(row["ecosystem"] == "cargo" for row in rows),
        "license_expressions": sorted({row["license_expression"] for row in rows}),
        "unknown_license_tokens": unknown,
        "legal_review_complete": False,
        "components": rows,
    }


def _run_json(command: list[str], *, root: Path, label: str) -> dict[str, Any]:
    completed = subprocess.run(
        command,
        cwd=root,
        capture_output=True,
        text=False,
        check=False,
    )
    # npm audit and pip-audit use a nonzero exit when findings exist; their JSON
    # must still be evaluated. cargo-audit likewise exposes findings in JSON.
    _require(completed.stdout.strip(), f"{label} produced no JSON (exit={completed.returncode})")
    return load_json_bytes(completed.stdout, label)


def _require_tool_version(command: list[str], expected: str, *, root: Path, label: str) -> None:
    completed = subprocess.run(
        command,
        cwd=root,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
    )
    actual = completed.stdout.strip()
    _require(
        completed.returncode == 0 and actual == expected,
        f"{label} version differs from the pinned toolchain: {actual or '<missing>'}",
    )


def _tracked_files(root: Path) -> bytes:
    completed = subprocess.run(
        ["git", "ls-files", "-z"], cwd=root, capture_output=True, check=False
    )
    _require(completed.returncode == 0, "git ls-files failed for secret scan")
    return completed.stdout


def _run_detect_secrets(root: Path) -> dict[str, Any]:
    command = [
        sys.executable,
        "-m",
        "detect_secrets",
        "scan",
        "--all-files",
        "--force-use-all-plugins",
        "--exclude-files",
        r"(?i)([.](png|jpe?g|gif|pdf|qpls|xlsx|exe|zip)$|^docs/design/|^docs/screenshots/|^validation/fixtures/)",
        "--exclude-lines",
        r"(?i)sha256|integrity|fingerprint|secret-pattern|bearer",
        "-",
    ]
    completed = subprocess.run(command, cwd=root, input=_tracked_files(root), capture_output=True, check=False)
    _require(completed.stdout.strip(), f"detect-secrets produced no JSON (exit={completed.returncode})")
    return load_json_bytes(completed.stdout, "detect-secrets")


def collect_scans(root: Path = ROOT) -> dict[str, dict[str, Any]]:
    root = root.resolve()
    npm_command = shutil.which("npm.cmd") or shutil.which("npm")
    cargo_command = shutil.which("cargo.exe") or shutil.which("cargo")
    cargo_audit_command = shutil.which("cargo-audit.exe") or shutil.which("cargo-audit")
    _require(npm_command is not None, "npm is required for the security scan")
    _require(cargo_command is not None, "Cargo is required for the security scan")
    _require(cargo_audit_command is not None, "cargo-audit is required for the security scan")
    _require_tool_version(
        [sys.executable, "-m", "pip_audit", "--version"],
        PINNED_PIP_AUDIT_VERSION,
        root=root,
        label="pip-audit",
    )
    _require_tool_version(
        [sys.executable, "-m", "detect_secrets", "--version"],
        PINNED_DETECT_SECRETS_VERSION,
        root=root,
        label="detect-secrets",
    )
    _require_tool_version(
        [cargo_audit_command, "--version"],
        PINNED_CARGO_AUDIT_VERSION,
        root=root,
        label="cargo-audit",
    )
    npm = _run_json([npm_command, "audit", "--audit-level=high", "--json"], root=root, label="npm audit")
    python = _run_json(
        [sys.executable, "-m", "pip_audit", "-r", "validation/requirements.txt", "--require-hashes", "--format=json"],
        root=root,
        label="pip-audit",
    )
    rust = _run_json(
        [cargo_command, "audit", "--target-os", "windows", "--target-arch", "x86_64", "--json"],
        root=root,
        label="cargo-audit",
    )
    secrets = _run_detect_secrets(root)
    cargo_metadata = _run_json(
        [cargo_command, "metadata", "--locked", "--filter-platform", "x86_64-pc-windows-msvc", "--format-version", "1"],
        root=root,
        label="cargo metadata",
    )
    return {"npm": npm, "python": python, "rust": rust, "secrets": secrets, "cargo_metadata": cargo_metadata}


def evaluate_scans(
    scans: dict[str, dict[str, Any]],
    dispositions_document: dict[str, Any],
    *,
    package_lock: dict[str, Any],
    today: date | None = None,
) -> dict[str, Any]:
    contract = validate_dispositions(dispositions_document, today=today)
    npm = evaluate_npm(scans["npm"], contract["policy"]["npm_block_severities"])
    python = evaluate_python(scans["python"])
    rust = evaluate_rust(scans["rust"], contract["dispositions"])
    secrets = evaluate_secrets(scans["secrets"])
    licenses = evaluate_licenses(package_lock, scans["cargo_metadata"])
    checks = {"npm": npm, "python": python, "rust": rust, "secrets": secrets, "licenses": licenses}
    passed = all(item["passed"] for item in checks.values())
    return {
        "schema_version": 1,
        "document_type": "quickpls_security_scan_report",
        "passed": passed,
        "checks": checks,
        "trust": {
            "exact_candidate_scan": False,
            "external_security_review_complete": False,
            "legal_review_complete": False,
            "commercial_gate_satisfied": False,
            "competitor_claims_authorized": False,
        },
    }


def write_report(path: Path, report: dict[str, Any], *, root: Path = ROOT) -> None:
    resolved = path.resolve()
    _require(resolved.is_relative_to(root.resolve()), "security scan report must remain inside the repository")
    _require(not path.exists() and not path.is_symlink(), f"refusing to overwrite security scan report: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = (json.dumps(report, indent=2, sort_keys=True) + "\n").encode("utf-8")
    try:
        with path.open("xb") as handle:
            handle.write(payload)
    except BaseException:
        path.unlink(missing_ok=True)
        raise


def main(argv: Iterable[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dispositions", type=Path, default=DEFAULT_DISPOSITIONS)
    parser.add_argument("--report", type=Path, default=DEFAULT_REPORT)
    args = parser.parse_args(list(argv) if argv is not None else None)
    try:
        scans = collect_scans(ROOT)
        dispositions = load_json(args.dispositions, "security scan dispositions")
        package_lock = load_json(ROOT / "package-lock.json", "package-lock.json")
        report = evaluate_scans(scans, dispositions, package_lock=package_lock)
        report.update(
            {
                "generated_at_utc": datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z"),
                "inputs": {
                    "package_lock": _descriptor(ROOT / "package-lock.json"),
                    "cargo_lock": _descriptor(ROOT / "Cargo.lock"),
                    "python_requirements": _descriptor(ROOT / "validation" / "requirements.txt"),
                    "security_tool_requirements": _descriptor(
                        ROOT / "validation" / "security-tools-requirements.txt"
                    ),
                    "dispositions": _descriptor(args.dispositions),
                },
            }
        )
        write_report(args.report, report)
    except (OSError, ScanError, subprocess.SubprocessError) as error:
        print(json.dumps({"passed": False, "commercial_gate_satisfied": False, "errors": [str(error)]}, indent=2))
        return 1
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0 if report["passed"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
