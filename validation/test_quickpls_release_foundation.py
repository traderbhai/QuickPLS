from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from validation.quickpls_release_foundation import validate_release_foundation
from validation.test_package_release_artifacts import VERSION, write_release_contract
from validation.webview2_offline_containment import EXPECTED_BROWSER_ARGUMENTS


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]


def write_foundation_fixture(root: Path) -> None:
    write_release_contract(root)
    tauri_path = root / "src-tauri" / "tauri.conf.json"
    tauri_path.write_text(
        json.dumps(
            {
                "productName": "QuickPLS",
                "version": VERSION,
                "app": {
                    "windows": [{
                        "title": "QuickPLS",
                        "additionalBrowserArgs": " ".join(EXPECTED_BROWSER_ARGUMENTS),
                    }],
                    "security": {
                        "csp": "default-src 'self'; connect-src ipc: http://ipc.localhost",
                    },
                },
                "bundle": {
                    "active": True,
                    "targets": "nsis",
                    "windows": {
                        "webviewInstallMode": {"type": "offlineInstaller", "silent": True},
                        "allowDowngrades": False,
                    },
                },
            }
        ),
        encoding="utf-8",
    )
    readiness_path = root / "validation" / "quickpls_3_release_readiness.json"
    readiness_path.write_text(
        json.dumps(
            {
                "target_channel": "stable",
                "release_channels": {
                    "internal": "maintainer_only",
                    "beta": "signed_prerelease",
                    "stable": "all_mandatory_gates_passed",
                },
                "requirements": [
                    {"id": "signing.identity", "required": True},
                    {"id": "signing.artifacts", "required": True},
                    {"id": "governance.claims_channels", "required": True},
                ],
            }
        ),
        encoding="utf-8",
    )


def mutate_json(path: Path, keys: tuple[str, ...], value: object) -> None:
    document = json.loads(path.read_text(encoding="utf-8"))
    target = document
    for key in keys[:-1]:
        target = target[key]
    target[keys[-1]] = value
    path.write_text(json.dumps(document), encoding="utf-8")


class ReleaseFoundationTests(unittest.TestCase):
    def test_repository_release_foundation_passes(self) -> None:
        report = validate_release_foundation(REPOSITORY_ROOT)

        self.assertTrue(report["passed"])
        self.assertEqual(report["default_artifact_channel"], "unsigned-preview")
        self.assertEqual(report["unsigned_artifact_channels"], ["internal", "unsigned-preview"])
        self.assertEqual(report["signed_candidate_channels"], ["beta", "stable"])
        self.assertFalse(report["offline_installer"]["downgrades_allowed"])
        self.assertTrue(report["webview2_offline_containment"]["passed"])

    def test_rejects_online_webview_install_and_installer_downgrades(self) -> None:
        mutations = {
            "online WebView bootstrapper": (
                ("bundle", "windows", "webviewInstallMode"),
                {"type": "downloadBootstrapper", "silent": True},
            ),
            "installer downgrade": (("bundle", "windows", "allowDowngrades"), True),
        }
        for name, (keys, value) in mutations.items():
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                write_foundation_fixture(root)
                mutate_json(root / "src-tauri" / "tauri.conf.json", keys, value)
                with self.assertRaises(SystemExit):
                    validate_release_foundation(root)

    def test_rejects_weakening_signed_commercial_channels(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            write_foundation_fixture(root)
            mutate_json(
                root / "validation" / "quickpls_3_release_readiness.json",
                ("release_channels", "beta"),
                "unsigned_prerelease",
            )
            with self.assertRaises(SystemExit):
                validate_release_foundation(root)

    def test_rejects_optional_signing_requirements(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            write_foundation_fixture(root)
            mutate_json(
                root / "validation" / "quickpls_3_release_readiness.json",
                ("requirements", 1, "required"),
                False,
            )
            with self.assertRaises(SystemExit):
                validate_release_foundation(root)


if __name__ == "__main__":
    unittest.main()
