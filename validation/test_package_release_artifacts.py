from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from validation import package_release_artifacts as release


VERSION = "3.0.0"
REPOSITORY_RELEASE_VERSION = "2.53.0"
REPOSITORY_ARTIFACT_LABEL = "v2_53_0_native_mediation_moderation"
REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
REPOSITORY_CARGO_PACKAGES = {
    "qpls-assessment",
    "qpls-cli",
    "qpls-core",
    "qpls-data",
    "qpls-estimation",
    "qpls-project",
    "qpls-resampling",
    "qpls-runner",
    "quickpls-desktop",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest().upper()


def write_release_contract(root: Path, version: str = VERSION) -> None:
    (root / "package.json").write_text(
        json.dumps({"name": "quickpls", "version": version}), encoding="utf-8"
    )
    (root / "package-lock.json").write_text(
        json.dumps(
            {
                "name": "quickpls",
                "version": version,
                "lockfileVersion": 3,
                "packages": {"": {"name": "quickpls", "version": version}},
            }
        ),
        encoding="utf-8",
    )
    (root / "Cargo.toml").write_text(
        """[workspace]
members = ["crates/qpls-cli", "src-tauri"]

[workspace.package]
version = "{version}"
edition = "2024"
""".format(version=version),
        encoding="utf-8",
    )
    (root / "crates" / "qpls-cli").mkdir(parents=True)
    (root / "crates" / "qpls-cli" / "Cargo.toml").write_text(
        '[package]\nname = "qpls-cli"\nversion.workspace = true\n', encoding="utf-8"
    )
    (root / "src-tauri").mkdir()
    (root / "src-tauri" / "Cargo.toml").write_text(
        '[package]\nname = "quickpls-desktop"\nversion.workspace = true\n', encoding="utf-8"
    )
    (root / "src-tauri" / "tauri.conf.json").write_text(
        json.dumps({"productName": "QuickPLS", "version": version}), encoding="utf-8"
    )
    (root / "validation").mkdir()
    (root / "validation" / "quickpls_release_channels.json").write_text(
        json.dumps(
            {
                "schema_version": 1,
                "product_version": version,
                "default_artifact_channel": "unsigned-preview",
                "commercial_readiness_contract": "validation/quickpls_3_release_readiness.json",
                "channels": release.EXPECTED_CHANNEL_POLICY,
            }
        ),
        encoding="utf-8",
    )
    (root / "Cargo.lock").write_text(
        """version = 4

[[package]]
name = "qpls-cli"
version = "{version}"

[[package]]
name = "quickpls-desktop"
version = "{version}"
""".format(version=version),
        encoding="utf-8",
    )


def write_release_inputs(root: Path, version: str = VERSION) -> Path:
    release_dir = root / "target" / "release"
    nsis = release_dir / "bundle" / "nsis"
    nsis.mkdir(parents=True)
    (release_dir / "quickpls-desktop.exe").write_bytes(b"portable-desktop")
    (release_dir / "qpls.exe").write_bytes(b"quickpls-cli")
    (nsis / f"QuickPLS_{version}_x64-setup.exe").write_bytes(b"nsis-setup")
    return release_dir


class VersionContractTests(unittest.TestCase):
    def test_accepts_one_exact_version_across_manifests_members_and_lock(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            write_release_contract(root)

            version, evidence = release.read_version_contract(root)

            self.assertEqual(version, VERSION)
            self.assertEqual(evidence["package_lock"], {"document": VERSION, "root_package": VERSION})
            self.assertEqual(
                evidence["cargo_lock_quickpls_packages"],
                {"qpls-cli": VERSION, "quickpls-desktop": VERSION},
            )

    def test_rejects_every_release_version_drift_and_lock_ambiguity(self) -> None:
        mutations = {
            "package lock document": lambda root: _mutate_json(root / "package-lock.json", ("version",), "2.9.9"),
            "package lock root": lambda root: _mutate_json(
                root / "package-lock.json", ("packages", "", "version"), "2.9.9"
            ),
            "cargo workspace": lambda root: _replace(root / "Cargo.toml", VERSION, "2.9.9"),
            "tauri": lambda root: _mutate_json(root / "src-tauri" / "tauri.conf.json", ("version",), "2.9.9"),
            "cargo lock package": lambda root: _replace_first(
                root / "Cargo.lock", f'version = "{VERSION}"', 'version = "2.9.9"'
            ),
            "cargo lock duplicate": lambda root: _append(
                root / "Cargo.lock", f'\n[[package]]\nname = "qpls-cli"\nversion = "{VERSION}"\n'
            ),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                write_release_contract(root)
                mutate(root)
                with self.assertRaises(SystemExit):
                    release.read_version_contract(root)


class RepositoryReleaseMetadataTests(unittest.TestCase):
    def test_scoped_methods_release_metadata_and_current_facing_copy_are_coordinated(self) -> None:
        version, evidence = release.read_version_contract(REPOSITORY_ROOT)
        package = json.loads((REPOSITORY_ROOT / "package.json").read_text(encoding="utf-8"))
        prototype = (REPOSITORY_ROOT / "src" / "v2" / "NativePrototypeApp.tsx").read_text(encoding="utf-8")
        readme = (REPOSITORY_ROOT / "README.md").read_text(encoding="utf-8")
        installation = (REPOSITORY_ROOT / "docs" / "INSTALLATION.md").read_text(encoding="utf-8")

        self.assertEqual(version, REPOSITORY_RELEASE_VERSION)
        self.assertEqual(set(evidence["cargo_members"]), REPOSITORY_CARGO_PACKAGES)
        self.assertEqual(set(evidence["cargo_lock_quickpls_packages"]), REPOSITORY_CARGO_PACKAGES)
        self.assertEqual(
            package["scripts"]["qpls:release:artifacts"],
            "python validation/package_release_artifacts.py --channel unsigned-preview "
            f"--label {REPOSITORY_ARTIFACT_LABEL}",
        )
        self.assertEqual(prototype.count('const releaseVersion = "2.53.0";'), 1)
        self.assertNotIn('const releaseVersion = "2.45.0";', prototype)

        self.assertIn(
            "Current public pre-release: [`v2.53.0`](https://github.com/traderbhai/QuickPLS/releases/tag/v2.53.0).",
            readme,
        )
        self.assertIn("_x64_cli.exe` - command-line executable for batch recipes.", readme)
        self.assertIn(
            "Standard shows only an exact option cell that has passed the full release evidence ladder and "
            "has either complete coverage or a nonempty, explicitly documented bounded scope.",
            " ".join(readme.replace("**", "").split()),
        )
        self.assertIn(
            "The Registry contains 41 scoped-Standard exact cells; its conservative compatibility projection "
            "is 27 Standard rows, 16 Labs rows, and two Legacy rows.",
            " ".join(readme.split()),
        )
        self.assertNotIn("The coordinated public 2.46.0 Wave 1 release packages this qualified capability", readme)
        self.assertIn("Previous Milestone Notes v2.45.0", readme)
        self.assertNotIn("coordinated public 2.46.0 release transition is still pending", readme)

        self.assertIn(
            "Current public pre-release: [`v2.53.0`](https://github.com/traderbhai/QuickPLS/releases/tag/v2.53.0).",
            installation,
        )
        self.assertIn("_x64_cli.exe` for offline command-line and batch recipe execution.", installation)
        normalized_installation = " ".join(installation.split())
        self.assertIn(
            "The desktop, CLI, and analytical workflows require no internet connection, "
            "account, or cloud service after download.",
            normalized_installation,
        )
        self.assertIn(
            "This is a functional-offline claim, not a literal fully-offline, no-telemetry, "
            "or zero-egress process-tree claim",
            normalized_installation,
        )
        self.assertNotIn("All three executables run fully offline after download.", installation)
        self.assertIn("_x64_cli.exe -Algorithm SHA256", installation)


class ArtifactPackagingTests(unittest.TestCase):
    def test_installer_selection_is_exact_version_and_unambiguous(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            nsis = root / "nsis"
            nsis.mkdir()
            exact = nsis / f"QuickPLS_{VERSION}_x64-setup.exe"
            exact.write_bytes(b"exact")
            (nsis / "QuickPLS_2.9.9_x64-setup.exe").write_bytes(b"stale")
            self.assertEqual(release.select_exact_installer(nsis, VERSION), exact)

            alternate = nsis / f"QuickPLS_{VERSION}_debug_setup.exe"
            alternate.write_bytes(b"ambiguous")
            with self.assertRaises(SystemExit):
                release.select_exact_installer(nsis, VERSION)
            alternate.unlink()
            exact.unlink()
            (nsis / f"QuickPLS_{VERSION}_custom_setup.exe").write_bytes(b"noncanonical")
            with self.assertRaises(SystemExit):
                release.select_exact_installer(nsis, VERSION)

    def test_packages_verified_portable_cli_setup_and_checksum_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            write_release_contract(root)
            release_dir = write_release_inputs(root)
            artifact_dir = release_dir / "artifacts"
            report_path = root / "validation" / "results" / "release_artifacts.json"

            report = release.package_release_artifacts(
                root=root,
                release_dir=release_dir,
                artifact_dir=artifact_dir,
                report_path=report_path,
                channel="unsigned-preview",
                label="wave 8 candidate",
                timestamp="20260813-120102",
            )

            stem = "QuickPLS_3.0.0_unsigned-preview_wave_8_candidate_20260813-120102_x64"
            expected_names = [
                f"{stem}_portable.exe",
                f"{stem}_cli.exe",
                f"{stem}_setup.exe",
                f"{stem}_checksums.txt",
            ]
            self.assertEqual([Path(item["path"]).name for item in report["artifacts"]], expected_names)
            self.assertEqual([item["role"] for item in report["artifacts"]], ["portable", "cli", "setup", "checksums"])
            self.assertEqual(report["schema_version"], 2)
            self.assertEqual(report["release_channel"], "unsigned-preview")
            self.assertEqual(report["trust"]["status"], "not_verified")
            self.assertFalse(report["trust"]["stable_eligible"])
            self.assertFalse(report["trust"]["competitor_claims_authorized"])
            for item in report["artifacts"][:3]:
                destination = root / item["path"]
                source = root / item["source"]
                self.assertTrue(item["copy_verified"])
                self.assertEqual(item["bytes"], item["source_bytes"])
                self.assertEqual(item["sha256"], item["source_sha256"])
                self.assertEqual(item["sha256"], sha256(destination))
                self.assertEqual(source.read_bytes(), destination.read_bytes())
            checksum = artifact_dir / expected_names[-1]
            self.assertEqual(
                checksum.read_text(encoding="utf-8").splitlines(),
                [f"{item['sha256']}  {Path(item['path']).name}" for item in report["artifacts"][:3]],
            )
            self.assertEqual(json.loads(report_path.read_text(encoding="utf-8")), report)
            with self.assertRaises(SystemExit):
                release.package_release_artifacts(
                    root=root,
                    release_dir=release_dir,
                    artifact_dir=artifact_dir,
                    report_path=report_path,
                    channel="unsigned-preview",
                    label="wave 8 candidate",
                    timestamp="20260813-120102",
                )

    def test_unsigned_factory_rejects_beta_and_stable_before_copying(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            write_release_contract(root)
            release_dir = write_release_inputs(root)
            artifact_dir = release_dir / "artifacts"
            report_path = root / "validation" / "results" / "release_artifacts.json"

            for channel in ("beta", "stable"):
                with self.subTest(channel=channel), self.assertRaises(SystemExit):
                    release.package_release_artifacts(
                        root=root,
                        release_dir=release_dir,
                        artifact_dir=artifact_dir,
                        report_path=report_path,
                        channel=channel,
                        label="must fail",
                        timestamp="20260813-120103",
                    )
            self.assertFalse(artifact_dir.exists())

    def test_channel_contract_rejects_version_drift_and_policy_weakening(self) -> None:
        mutations = {
            "version drift": lambda root: _mutate_json(
                root / "validation" / "quickpls_release_channels.json",
                ("product_version",),
                "2.9.9",
            ),
            "unsigned beta": lambda root: _mutate_json(
                root / "validation" / "quickpls_release_channels.json",
                ("channels", "beta", "authenticode_required"),
                False,
            ),
            "stable claims bypass gate": lambda root: _mutate_json(
                root / "validation" / "quickpls_release_channels.json",
                ("channels", "stable", "competitor_claims_policy"),
                "authorized",
            ),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                write_release_contract(root)
                mutate(root)
                with self.assertRaises(SystemExit):
                    release.read_version_contract(root)

    def test_copy_fails_closed_when_destination_differs_from_source(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / "source.exe"
            destination = root / "destination.exe"
            source.write_bytes(b"trusted")

            def corrupt_copy(_source: Path, output: Path) -> None:
                Path(output).write_bytes(b"corrupt")

            with mock.patch.object(release.shutil, "copy2", side_effect=corrupt_copy):
                with self.assertRaises(SystemExit):
                    release.copy_artifact("portable", source, destination, root)
            self.assertFalse(destination.exists())

    def test_copy_fails_closed_when_source_changes_during_copy(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / "source.exe"
            destination = root / "destination.exe"
            source.write_bytes(b"original")

            def mutate_source(input_path: Path, output: Path) -> None:
                Path(output).write_bytes(Path(input_path).read_bytes())
                Path(input_path).write_bytes(b"changed-after-copy")

            with mock.patch.object(release.shutil, "copy2", side_effect=mutate_source):
                with self.assertRaises(SystemExit):
                    release.copy_artifact("portable", source, destination, root)
            self.assertFalse(destination.exists())


def _mutate_json(path: Path, keys: tuple[str, ...], value: object) -> None:
    document = json.loads(path.read_text(encoding="utf-8"))
    target = document
    for key in keys[:-1]:
        target = target[key]
    target[keys[-1]] = value
    path.write_text(json.dumps(document), encoding="utf-8")


def _replace(path: Path, old: str, new: str) -> None:
    path.write_text(path.read_text(encoding="utf-8").replace(old, new), encoding="utf-8")


def _replace_first(path: Path, old: str, new: str) -> None:
    path.write_text(path.read_text(encoding="utf-8").replace(old, new, 1), encoding="utf-8")


def _append(path: Path, text: str) -> None:
    path.write_text(path.read_text(encoding="utf-8") + text, encoding="utf-8")


if __name__ == "__main__":
    unittest.main()
