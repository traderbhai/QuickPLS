"""Copy verified release outputs to unique versioned artifact names.

Tauri writes predictable bundle names under target/release and may overwrite
the previous installer for the same app version. This script preserves a fresh
copy for user testing by adding the app version, milestone label, and UTC build
timestamp to each artifact name. It is deliberately packaging-only: builds and
signing must complete before this script is invoked.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import tomllib
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
RELEASE_DIR = Path(os.environ.get("QPLS_RELEASE_DIR", ROOT / "target" / "release"))
ARTIFACT_DIR = ROOT / "target" / "release" / "artifacts"
REPORT = ROOT / "validation" / "results" / "release_artifacts.json"

TIMESTAMP_PATTERN = re.compile(r"^[0-9]{8}-[0-9]{6}$")
VERSION_FILENAME_PATTERN = re.compile(r"^[0-9A-Za-z][0-9A-Za-z.+-]*$")

EXPECTED_CHANNEL_POLICY: dict[str, dict[str, object]] = {
    "internal": {
        "audience": "maintainers",
        "artifact_token": "internal",
        "authenticode_required": False,
        "distribution": "maintainer_only",
        "commercial_channel": None,
        "competitor_claims_policy": "prohibited",
        "artifact_factory": "unsigned_preview",
    },
    "unsigned-preview": {
        "audience": "named_technical_preview_testers",
        "artifact_token": "unsigned-preview",
        "authenticode_required": False,
        "distribution": "private_named_testers_only",
        "commercial_channel": None,
        "competitor_claims_policy": "prohibited",
        "artifact_factory": "unsigned_preview",
    },
    "beta": {
        "audience": "named_external_beta_testers",
        "artifact_token": "beta",
        "authenticode_required": True,
        "distribution": "signed_prerelease_only",
        "commercial_channel": "beta",
        "competitor_claims_policy": "prohibited",
        "artifact_factory": "signed_candidate",
    },
    "stable": {
        "audience": "public_users",
        "artifact_token": "stable",
        "authenticode_required": True,
        "distribution": "commercial_gate_required",
        "commercial_channel": "stable",
        "competitor_claims_policy": "commercial_gate_required",
        "artifact_factory": "signed_candidate",
    },
}


def slug(value: str) -> str:
    cleaned = re.sub(r"[^A-Za-z0-9_.-]+", "_", value.strip())
    return cleaned.strip("._-") or "build"


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
        value = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=reject_duplicate_keys,
            parse_constant=reject_non_finite,
        )
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as error:
        raise SystemExit(f"Cannot read {label} at {path}: {error}") from error
    if not isinstance(value, dict):
        raise SystemExit(f"{label} must contain a JSON object: {path}")
    return value


def _read_toml(path: Path, label: str) -> dict[str, Any]:
    try:
        value = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, tomllib.TOMLDecodeError) as error:
        raise SystemExit(f"Cannot read {label} at {path}: {error}") from error
    if not isinstance(value, dict):
        raise SystemExit(f"{label} must contain a TOML table: {path}")
    return value


def _required_version(value: object, label: str) -> str:
    if not isinstance(value, str) or not value.strip() or value != value.strip():
        raise SystemExit(f"{label} must be a non-empty version string")
    return value


def read_release_channel_contract(
    root: Path = ROOT, *, expected_version: str | None = None
) -> dict[str, Any]:
    """Load the frozen channel policy used by the unsigned artifact factory.

    Beta and stable remain in the policy so attempts to route them through the
    unsigned factory fail explicitly. The exact expected policy is enforced in
    code so editing a JSON flag cannot silently authorize unsigned distribution.
    """

    root = root.resolve()
    contract = _read_json(
        root / "validation" / "quickpls_release_channels.json",
        "QuickPLS release channel contract",
    )
    required_keys = {
        "schema_version",
        "product_version",
        "default_artifact_channel",
        "commercial_readiness_contract",
        "channels",
    }
    if set(contract) != required_keys:
        raise SystemExit(
            "Release channel contract keys do not match the frozen schema: "
            f"expected={sorted(required_keys)}, actual={sorted(contract)}"
        )
    if contract["schema_version"] != 1:
        raise SystemExit("Release channel contract schema_version must be 1")
    version = _required_version(contract["product_version"], "release channel product_version")
    if expected_version is not None and version != expected_version:
        raise SystemExit(
            f"Release channel product_version mismatch: {version}, expected {expected_version}"
        )
    if contract["default_artifact_channel"] != "unsigned-preview":
        raise SystemExit("Default artifact channel must remain unsigned-preview")
    if contract["commercial_readiness_contract"] != "validation/quickpls_3_release_readiness.json":
        raise SystemExit("Release channel contract must bind the QuickPLS 3 commercial readiness contract")
    channels = contract["channels"]
    if channels != EXPECTED_CHANNEL_POLICY:
        raise SystemExit("Release channels do not match the frozen fail-closed policy")
    return contract


def _workspace_member_manifests(root: Path, workspace: dict[str, Any]) -> tuple[Path, ...]:
    workspace_table = workspace.get("workspace")
    members = workspace_table.get("members") if isinstance(workspace_table, dict) else None
    if not isinstance(members, list) or not members or any(not isinstance(item, str) for item in members):
        raise SystemExit("Cargo.toml [workspace].members must be a non-empty string list")

    manifests: list[Path] = []
    for pattern in members:
        matches = sorted(root.glob(pattern), key=lambda path: path.as_posix())
        if not matches:
            raise SystemExit(f"Cargo workspace member pattern did not resolve: {pattern}")
        for match in matches:
            manifest = match / "Cargo.toml" if match.is_dir() else match
            if not manifest.is_file():
                raise SystemExit(f"Cargo workspace member manifest is missing: {manifest}")
            manifests.append(manifest.resolve())
    unique = tuple(dict.fromkeys(manifests))
    if len(unique) != len(manifests):
        raise SystemExit("Cargo workspace member patterns resolve to duplicate manifests")
    return unique


def read_version_contract(root: Path = ROOT) -> tuple[str, dict[str, Any]]:
    """Require one exact version across every release-facing manifest and lock."""

    root = root.resolve()
    package = _read_json(root / "package.json", "package.json")
    package_lock = _read_json(root / "package-lock.json", "package-lock.json")
    tauri = _read_json(root / "src-tauri" / "tauri.conf.json", "Tauri configuration")
    cargo = _read_toml(root / "Cargo.toml", "workspace Cargo.toml")
    cargo_lock = _read_toml(root / "Cargo.lock", "Cargo.lock")

    package_version = _required_version(package.get("version"), "package.json version")
    if not VERSION_FILENAME_PATTERN.fullmatch(package_version):
        raise SystemExit(f"Version is not safe for release filenames: {package_version!r}")

    package_lock_packages = package_lock.get("packages")
    if not isinstance(package_lock_packages, dict):
        raise SystemExit("package-lock.json is missing packages root metadata")
    package_lock_root = package_lock_packages.get("")
    if not isinstance(package_lock_root, dict):
        raise SystemExit("package-lock.json is missing packages[''] root metadata")

    workspace_table = cargo.get("workspace")
    workspace_package = workspace_table.get("package") if isinstance(workspace_table, dict) else None
    # tomllib represents [workspace.package] as workspace['package'].
    if not isinstance(workspace_package, dict):
        raise SystemExit("Cargo.toml is missing [workspace.package]")
    cargo_version = _required_version(workspace_package.get("version"), "Cargo workspace version")

    versions = {
        "package.json": package_version,
        "package-lock.json": _required_version(package_lock.get("version"), "package-lock.json version"),
        "package-lock.json root package": _required_version(
            package_lock_root.get("version"), "package-lock.json root package version"
        ),
        "Cargo.toml workspace": cargo_version,
        "src-tauri/tauri.conf.json": _required_version(tauri.get("version"), "Tauri version"),
        "validation/quickpls_release_channels.json": _required_version(
            read_release_channel_contract(root, expected_version=package_version).get("product_version"),
            "release channel product_version",
        ),
    }
    mismatches = {name: value for name, value in versions.items() if value != package_version}
    if mismatches:
        details = ", ".join(f"{name}={value}" for name, value in versions.items())
        raise SystemExit(f"Release version mismatch: {details}")

    member_versions: dict[str, str] = {}
    member_names: list[str] = []
    for manifest in _workspace_member_manifests(root, cargo):
        member = _read_toml(manifest, f"Cargo workspace member {manifest}")
        package_table = member.get("package")
        if not isinstance(package_table, dict):
            raise SystemExit(f"Cargo workspace member is missing [package]: {manifest}")
        name = package_table.get("name")
        if not isinstance(name, str) or not name:
            raise SystemExit(f"Cargo workspace member has no package name: {manifest}")
        if name in member_names:
            raise SystemExit(f"Duplicate Cargo workspace package name: {name}")
        declared = package_table.get("version")
        if isinstance(declared, dict):
            if declared != {"workspace": True}:
                raise SystemExit(f"Cargo package {name} has an invalid workspace version declaration")
            member_version = cargo_version
        else:
            member_version = _required_version(declared, f"Cargo package {name} version")
        if member_version != package_version:
            raise SystemExit(
                f"Cargo workspace package version mismatch: {name}={member_version}, expected {package_version}"
            )
        member_names.append(name)
        member_versions[name] = member_version

    lock_packages = cargo_lock.get("package")
    if not isinstance(lock_packages, list):
        raise SystemExit("Cargo.lock is missing [[package]] entries")
    quickpls_lock_entries = [
        item
        for item in lock_packages
        if isinstance(item, dict)
        and isinstance(item.get("name"), str)
        and (item["name"].startswith("qpls-") or item["name"] in {"quickpls", "quickpls-desktop"})
    ]
    lock_versions: dict[str, str] = {}
    for name in member_names:
        matches = [item for item in quickpls_lock_entries if item.get("name") == name]
        if len(matches) != 1:
            raise SystemExit(f"Cargo.lock must contain exactly one entry for workspace package {name}; found {len(matches)}")
        locked_version = _required_version(matches[0].get("version"), f"Cargo.lock package {name} version")
        if locked_version != package_version:
            raise SystemExit(
                f"Cargo.lock QuickPLS package version mismatch: {name}={locked_version}, expected {package_version}"
            )
        lock_versions[name] = locked_version
    unexpected = sorted({item["name"] for item in quickpls_lock_entries} - set(member_names))
    if unexpected:
        raise SystemExit(f"Cargo.lock contains QuickPLS packages outside the workspace: {unexpected}")

    return package_version, {
        "package_json": package_version,
        "package_lock": {
            "document": versions["package-lock.json"],
            "root_package": versions["package-lock.json root package"],
        },
        "cargo_workspace": cargo_version,
        "cargo_members": dict(sorted(member_versions.items())),
        "cargo_lock_quickpls_packages": dict(sorted(lock_versions.items())),
        "tauri": versions["src-tauri/tauri.conf.json"],
        "release_channels": versions["validation/quickpls_release_channels.json"],
    }


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest().upper()


def _display_path(path: Path, root: Path) -> str:
    resolved = path.resolve()
    try:
        return resolved.relative_to(root.resolve()).as_posix()
    except ValueError:
        return str(resolved)


def _file_identity(path: Path) -> tuple[int, str]:
    if not path.is_file():
        raise SystemExit(f"Missing release artifact: {path}")
    size = path.stat().st_size
    if size <= 0:
        raise SystemExit(f"Release artifact is empty: {path}")
    return size, sha256(path)


def copy_artifact(role: str, source: Path, destination: Path, root: Path = ROOT) -> dict[str, object]:
    """Copy one new artifact and prove source stability plus byte/hash identity."""

    if destination.exists():
        raise SystemExit(f"Refusing to overwrite release artifact: {destination}")
    source_before = _file_identity(source)
    destination.parent.mkdir(parents=True, exist_ok=True)
    try:
        shutil.copy2(source, destination)
        source_after = _file_identity(source)
        destination_identity = _file_identity(destination)
        if source_before != source_after:
            raise SystemExit(f"Release source changed while it was being copied: {source}")
        if destination_identity != source_before:
            raise SystemExit(f"Copied release artifact does not match its source: {destination}")
    except BaseException:
        if destination.exists():
            destination.unlink()
        raise
    return {
        "role": role,
        "source": _display_path(source, root),
        "source_bytes": source_before[0],
        "source_sha256": source_before[1],
        "path": _display_path(destination, root),
        "bytes": destination_identity[0],
        "sha256": destination_identity[1],
        "copy_verified": True,
    }


def select_exact_installer(nsis_dir: Path, version: str) -> Path:
    """Select the sole canonical NSIS output for the exact release version."""

    if not nsis_dir.is_dir():
        raise SystemExit(f"NSIS release directory is missing: {nsis_dir}")
    expected = nsis_dir / f"QuickPLS_{version}_x64-setup.exe"
    version_candidates = sorted(
        (
            path
            for path in nsis_dir.iterdir()
            if path.is_file()
            and path.name.startswith(f"QuickPLS_{version}_")
            and path.name.lower().endswith("setup.exe")
        ),
        key=lambda path: path.name,
    )
    if version_candidates != [expected] or not expected.is_file():
        rendered = [path.name for path in version_candidates]
        raise SystemExit(
            f"Expected exactly one canonical installer {expected.name}; exact-version candidates={rendered}"
        )
    return expected


def _validated_timestamp(value: str | None) -> str:
    timestamp = value or datetime.now(timezone.utc).strftime("%Y%m%d-%H%M%S")
    if not TIMESTAMP_PATTERN.fullmatch(timestamp):
        raise SystemExit(f"Release timestamp must use UTC YYYYMMDD-HHMMSS: {timestamp!r}")
    try:
        datetime.strptime(timestamp, "%Y%m%d-%H%M%S")
    except ValueError as error:
        raise SystemExit(f"Release timestamp is invalid: {timestamp!r}") from error
    return timestamp


def package_release_artifacts(
    *,
    root: Path = ROOT,
    release_dir: Path = RELEASE_DIR,
    artifact_dir: Path = ARTIFACT_DIR,
    report_path: Path = REPORT,
    channel: str = "unsigned-preview",
    label: str = "manual_release",
    timestamp: str | None = None,
) -> dict[str, Any]:
    root = root.resolve()
    release_dir = release_dir.resolve()
    artifact_dir = artifact_dir.resolve()
    report_path = report_path.resolve()
    version, version_contract = read_version_contract(root)
    channel_contract = read_release_channel_contract(root, expected_version=version)
    channel_policy = channel_contract["channels"].get(channel)
    if channel_policy is None:
        raise SystemExit(
            f"Unknown release channel {channel!r}; expected one of {sorted(channel_contract['channels'])}"
        )
    if channel_policy["artifact_factory"] != "unsigned_preview":
        raise SystemExit(
            f"Channel {channel!r} requires the signed-candidate factory and cannot be packaged by "
            "the unsigned-preview artifact command"
        )
    release_label = slug(label)
    release_timestamp = _validated_timestamp(timestamp)
    stem = (
        f"QuickPLS_{version}_{channel_policy['artifact_token']}_"
        f"{release_label}_{release_timestamp}_x64"
    )

    sources = (
        ("portable", release_dir / "quickpls-desktop.exe", artifact_dir / f"{stem}_portable.exe"),
        ("cli", release_dir / "qpls.exe", artifact_dir / f"{stem}_cli.exe"),
        (
            "setup",
            select_exact_installer(release_dir / "bundle" / "nsis", version),
            artifact_dir / f"{stem}_setup.exe",
        ),
    )
    checksum_path = artifact_dir / f"{stem}_checksums.txt"
    collisions = [str(destination) for _, _, destination in sources if destination.exists()]
    if checksum_path.exists():
        collisions.append(str(checksum_path))
    if collisions:
        raise SystemExit(f"Refusing to overwrite existing timestamped release artifacts: {collisions}")

    artifacts: list[dict[str, object]] = []
    created: list[Path] = []
    try:
        for role, source, destination in sources:
            artifacts.append(copy_artifact(role, source, destination, root))
            created.append(destination)
        checksum_text = "".join(
            f"{item['sha256']}  {Path(str(item['path'])).name}\n" for item in artifacts
        )
        checksum_path.parent.mkdir(parents=True, exist_ok=True)
        with checksum_path.open("x", encoding="utf-8", newline="\n") as handle:
            handle.write(checksum_text)
        created.append(checksum_path)
        if checksum_path.read_text(encoding="utf-8") != checksum_text:
            raise SystemExit(f"Checksum manifest read-back mismatch: {checksum_path}")
        checksum_size, checksum_sha = _file_identity(checksum_path)
        artifacts.append(
            {
                "role": "checksums",
                "source": None,
                "source_bytes": None,
                "source_sha256": None,
                "path": _display_path(checksum_path, root),
                "bytes": checksum_size,
                "sha256": checksum_sha,
                "copy_verified": True,
            }
        )
    except BaseException:
        for created_path in reversed(created):
            if created_path.exists():
                created_path.unlink()
        raise

    report = {
        "schema_version": 2,
        "target": "QuickPLS unsigned preview artifact preservation",
        "passed": True,
        "version": version,
        "version_contract": version_contract,
        "release_channel": channel,
        "channel_policy": channel_policy,
        "trust": {
            "authenticode_required": False,
            "authenticode_verification_performed": False,
            "status": "not_verified",
            "stable_eligible": False,
            "competitor_claims_authorized": False,
        },
        "label": release_label,
        "timestamp_utc": release_timestamp,
        "artifact_directory": _display_path(artifact_dir, root),
        "artifacts": artifacts,
        "note": (
            "Files are copied to unique names so repeated desktop builds do not overwrite prior "
            "test artifacts. Every copied binary is size- and SHA-256-identical to its source, but "
            "checksums do not establish publisher authenticity. This unsigned-preview factory never "
            "authorizes beta, stable, or competitor-ready distribution."
        ),
    }
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return report


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--channel",
        default="unsigned-preview",
        choices=sorted(EXPECTED_CHANNEL_POLICY),
        help="Artifact channel. Beta and stable are deliberately rejected by this unsigned factory.",
    )
    parser.add_argument("--label", default="manual_release", help="Milestone/build label to include in artifact names.")
    parser.add_argument("--timestamp", default=None, help="Optional UTC timestamp override, e.g. 20260722-120000.")
    args = parser.parse_args()
    report = package_release_artifacts(channel=args.channel, label=args.label, timestamp=args.timestamp)
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
