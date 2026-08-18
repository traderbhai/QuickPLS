from __future__ import annotations

import copy
import json
import subprocess
import unittest
from datetime import date
from pathlib import Path
from unittest import mock

from validation import quickpls_security_scans as scans


ROOT = Path(__file__).resolve().parents[1]


def dispositions() -> dict[str, object]:
    return {
        "schema_version": 1,
        "document_type": "quickpls_security_scan_dispositions",
        "policy": {
            "npm_block_severities": ["high", "critical"],
            "python_block_known_vulnerabilities": True,
            "rust_block_known_vulnerabilities": True,
            "rust_block_unsound_warnings": True,
            "secret_findings_allowed": False,
            "license_inventory_review_required": True,
        },
        "rust_advisory_dispositions": [
            {
                "id": "RUSTSEC-2024-9999",
                "kind": "unmaintained",
                "status": "tracked_nonblocking",
                "scope": "Fixture-only transitive advisory with no reported vulnerability.",
                "owner": "release_engineering",
                "reviewed_at": "2026-08-14",
                "expires_at": "2027-02-14",
            }
        ],
        "release_claim": {"commercial_ready": False, "reason": "Fixture is not a release approval."},
    }


def package_lock() -> dict[str, object]:
    return {
        "packages": {
            "": {"name": "quickpls", "version": "3.0.0"},
            "node_modules/react": {"name": "react", "version": "19.0.0", "license": "MIT"},
        }
    }


def cargo_metadata() -> dict[str, object]:
    return {
        "packages": [
            {
                "id": "path+file:///quickpls#3.0.0",
                "name": "quickpls-desktop",
                "version": "3.0.0",
                "license": "LicenseRef-Proprietary",
            },
            {
                "id": "registry+crate#1.0.0",
                "name": "crate",
                "version": "1.0.0",
                "license": "Apache-2.0 OR MIT",
            },
        ],
        "resolve": {
            "nodes": [
                {"id": "path+file:///quickpls#3.0.0", "dependencies": ["registry+crate#1.0.0"]},
                {"id": "registry+crate#1.0.0", "dependencies": []},
            ]
        },
    }


def clean_scan() -> dict[str, object]:
    return {
        "npm": {
            "auditReportVersion": 2,
            "vulnerabilities": {},
            "metadata": {
                "vulnerabilities": {
                    "info": 0,
                    "low": 0,
                    "moderate": 0,
                    "high": 0,
                    "critical": 0,
                    "total": 0,
                }
            },
        },
        "python": {"dependencies": [{"name": "jsonschema", "version": "4.25.1", "vulns": []}], "fixes": []},
        "rust": {
            "database": {"advisory-count": 1000, "last-commit": "a" * 40, "last-updated": "2026-08-12"},
            "lockfile": {"dependency-count": 2},
            "vulnerabilities": {"found": False, "count": 0, "list": []},
            "warnings": {
                "unmaintained": [
                    {"advisory": {"id": "RUSTSEC-2024-9999"}, "package": {"name": "fixture"}}
                ]
            },
        },
        "secrets": {"version": "1.5.0", "plugins_used": [{"name": "PrivateKeyDetector"}], "results": {}},
        "cargo_metadata": cargo_metadata(),
    }


class QuickPlsSecurityScanTests(unittest.TestCase):
    def test_clean_scans_pass_without_authorizing_release(self) -> None:
        report = scans.evaluate_scans(
            clean_scan(),
            dispositions(),
            package_lock=package_lock(),
            today=date(2026, 8, 14),
        )

        self.assertTrue(report["passed"])
        self.assertFalse(report["trust"]["commercial_gate_satisfied"])
        self.assertFalse(report["trust"]["competitor_claims_authorized"])
        self.assertFalse(report["checks"]["licenses"]["legal_review_complete"])
        self.assertEqual(report["checks"]["licenses"]["component_count"], 3)

    def test_high_npm_python_and_rust_vulnerabilities_fail_closed(self) -> None:
        npm = clean_scan()
        npm["npm"]["vulnerabilities"] = {"bad": {"severity": "high"}}
        npm["npm"]["metadata"]["vulnerabilities"].update({"high": 1, "total": 1})
        self.assertFalse(scans.evaluate_scans(npm, dispositions(), package_lock=package_lock(), today=date(2026, 8, 14))["passed"])

        python = clean_scan()
        python["python"]["dependencies"][0]["vulns"] = [{"id": "PYSEC-1"}]
        self.assertFalse(scans.evaluate_scans(python, dispositions(), package_lock=package_lock(), today=date(2026, 8, 14))["passed"])

        rust = clean_scan()
        rust["rust"]["vulnerabilities"] = {
            "found": True,
            "count": 1,
            "list": [{"advisory": {"id": "RUSTSEC-2026-0001"}}],
        }
        self.assertFalse(scans.evaluate_scans(rust, dispositions(), package_lock=package_lock(), today=date(2026, 8, 14))["passed"])

    def test_secret_and_unknown_license_findings_fail_closed(self) -> None:
        secret = clean_scan()
        secret["secrets"]["results"] = {"src/example.ts": [{"type": "Private Key", "line_number": 1}]}
        self.assertFalse(scans.evaluate_scans(secret, dispositions(), package_lock=package_lock(), today=date(2026, 8, 14))["passed"])

        lock = package_lock()
        lock["packages"]["node_modules/react"]["license"] = "LicenseRef-Unreviewed"
        report = scans.evaluate_scans(clean_scan(), dispositions(), package_lock=lock, today=date(2026, 8, 14))
        self.assertFalse(report["passed"])
        self.assertEqual(report["checks"]["licenses"]["unknown_license_tokens"], ["LicenseRef-Unreviewed"])

    def test_undisposed_stale_and_expired_rust_warnings_are_rejected(self) -> None:
        undisposed = clean_scan()
        undisposed["rust"]["warnings"]["unsound"] = [
            {"advisory": {"id": "RUSTSEC-2026-0002"}, "package": {"name": "bad"}}
        ]
        report = scans.evaluate_scans(
            undisposed,
            dispositions(),
            package_lock=package_lock(),
            today=date(2026, 8, 14),
        )
        self.assertFalse(report["passed"])
        self.assertEqual(report["checks"]["rust"]["undisposed_warnings"], ["RUSTSEC-2026-0002"])

        expired = dispositions()
        expired["rust_advisory_dispositions"][0]["expires_at"] = "2026-08-13"
        with self.assertRaisesRegex(scans.ScanError, "expired"):
            scans.evaluate_scans(clean_scan(), expired, package_lock=package_lock(), today=date(2026, 8, 14))

        stale = dispositions()
        stale["rust_advisory_dispositions"].append(
            {
                "id": "RUSTSEC-2025-9998",
                "kind": "notice",
                "status": "tracked_nonblocking",
                "scope": "No longer present fixture.",
                "owner": "release_engineering",
                "reviewed_at": "2026-08-14",
                "expires_at": "2027-02-14",
            }
        )
        with self.assertRaisesRegex(scans.ScanError, "stale"):
            scans.evaluate_scans(clean_scan(), stale, package_lock=package_lock(), today=date(2026, 8, 14))

    def test_repository_disposition_contract_is_strict_and_pending(self) -> None:
        document = scans.load_json(scans.DEFAULT_DISPOSITIONS, "repository dispositions")
        normalized = scans.validate_dispositions(document, today=date(2026, 8, 14))

        self.assertEqual(len(normalized["dispositions"]), 18)
        self.assertFalse(document["release_claim"]["commercial_ready"])

    def test_scanner_tool_version_check_fails_closed(self) -> None:
        accepted = subprocess.CompletedProcess(
            args=["tool", "--version"],
            returncode=0,
            stdout="tool 1.2.3\n",
            stderr="",
        )
        with mock.patch.object(scans.subprocess, "run", return_value=accepted):
            scans._require_tool_version(
                ["tool", "--version"],
                "tool 1.2.3",
                root=ROOT,
                label="tool",
            )

        rejected = subprocess.CompletedProcess(
            args=["tool", "--version"],
            returncode=0,
            stdout="tool 9.9.9\n",
            stderr="",
        )
        with mock.patch.object(scans.subprocess, "run", return_value=rejected):
            with self.assertRaisesRegex(scans.ScanError, "version differs"):
                scans._require_tool_version(
                    ["tool", "--version"],
                    "tool 1.2.3",
                    root=ROOT,
                    label="tool",
                )


if __name__ == "__main__":
    unittest.main()
