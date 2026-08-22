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
import subprocess
import tomllib
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
RELEASE_DIR = Path(os.environ.get("QPLS_RELEASE_DIR", ROOT / "target" / "release"))
ARTIFACT_DIR = ROOT / "target" / "release" / "artifacts"
REPORT = ROOT / "validation" / "results" / "release_artifacts.json"

SOURCE_PROVENANCE_SCHEMA = 1
BUILD_SESSION_SCHEMA = 2
BUILD_SESSION_SUITE = "quickpls_unsigned_candidate_build_session_v2"
GIB_BYTES = 1024**3
BUILD_DISK_FLOOR_GIB = 20.0
BUILD_DISK_FLOOR_BYTES = int(BUILD_DISK_FLOOR_GIB * GIB_BYTES)
BUILD_PREFLIGHT_REQUIRED_GIB = {"C": 26.5, "D": 20.5}
BUILD_PREFLIGHT_REQUIRED_BYTES = {
    drive: int(required_gib * GIB_BYTES)
    for drive, required_gib in BUILD_PREFLIGHT_REQUIRED_GIB.items()
}
BUILD_PREFLIGHT_RESERVE_GIB = {"C": 6.5, "D": 0.5}
BUILD_DISK_POLL_INTERVAL_MS = 1000
BUILD_DISK_BREACH_ACTION = "terminate_only_exact_wrapper_owned_process_tree"

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
        "audience": "public_technical_preview_users",
        "artifact_token": "unsigned-preview",
        "authenticode_required": False,
        "distribution": "public_github_prerelease",
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
            path.read_text(encoding="utf-8-sig"),
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


def _git_bytes(root: Path, *arguments: str) -> bytes:
    try:
        completed = subprocess.run(
            ["git", "-C", str(root), *arguments],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    except (OSError, subprocess.CalledProcessError) as error:
        detail = ""
        if isinstance(error, subprocess.CalledProcessError):
            detail = error.stderr.decode("utf-8", errors="replace").strip()
        raise SystemExit(f"Git provenance command failed ({' '.join(arguments)}): {detail}") from error
    return completed.stdout


def read_clean_source_provenance(root: Path = ROOT) -> dict[str, Any]:
    """Bind a candidate to one exact clean Git commit and tracked source tree."""

    root = root.resolve()
    top_level = Path(_git_bytes(root, "rev-parse", "--show-toplevel").decode("utf-8").strip()).resolve()
    if top_level != root:
        raise SystemExit(f"Release root is not the Git top level: root={root}, git={top_level}")
    status = _git_bytes(root, "status", "--porcelain=v1", "--untracked-files=all")
    if status:
        rendered = status.decode("utf-8", errors="replace").strip()
        raise SystemExit(f"Release packaging requires a clean source worktree: {rendered}")
    commit = _git_bytes(root, "rev-parse", "HEAD").decode("ascii").strip().lower()
    tree = _git_bytes(root, "rev-parse", "HEAD^{tree}").decode("ascii").strip().lower()
    if not re.fullmatch(r"[0-9a-f]{40}", commit) or not re.fullmatch(r"[0-9a-f]{40}", tree):
        raise SystemExit("Git source commit/tree identity is malformed")
    tracked_manifest = _git_bytes(root, "ls-tree", "-r", "-z", "--full-tree", "HEAD")
    tracked_files = tuple(item for item in tracked_manifest.split(b"\0") if item)
    if not tracked_files:
        raise SystemExit("Git tracked-source manifest is empty")

    authority_paths = (
        "package.json",
        "package-lock.json",
        "Cargo.toml",
        "Cargo.lock",
        "src-tauri/tauri.conf.json",
        "validation/quickpls_release_channels.json",
    )
    authorities: list[dict[str, object]] = []
    for relative in authority_paths:
        path = root / relative
        size, digest = _file_identity(path)
        authorities.append({"path": relative, "bytes": size, "sha256": digest})

    return {
        "schema_version": SOURCE_PROVENANCE_SCHEMA,
        "repository_root": str(root),
        "commit": commit,
        "tree": tree,
        "worktree_clean": True,
        "tracked_file_count": len(tracked_files),
        "tracked_manifest_sha256": hashlib.sha256(tracked_manifest).hexdigest().upper(),
        "version_authorities": authorities,
    }


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


def _parse_utc_timestamp(value: object, label: str) -> datetime:
    if not isinstance(value, str) or not value.endswith("Z"):
        raise SystemExit(f"{label} must be an ISO-8601 UTC timestamp ending in Z")
    try:
        parsed = datetime.fromisoformat(value[:-1] + "+00:00")
    except ValueError as error:
        raise SystemExit(f"{label} is not a valid ISO-8601 UTC timestamp") from error
    if parsed.tzinfo is None or parsed.utcoffset() != timezone.utc.utcoffset(parsed):
        raise SystemExit(f"{label} must be UTC")
    return parsed


def _validate_log_binding(
    item: object, label: str, *, allow_empty: bool = False
) -> dict[str, object]:
    if not isinstance(item, dict):
        raise SystemExit(f"{label} must be an object")
    if set(item) != {"path", "bytes", "sha256"}:
        raise SystemExit(f"{label} keys do not match the frozen build-log schema")
    path_value = item.get("path")
    if not isinstance(path_value, str) or not path_value:
        raise SystemExit(f"{label}.path must be a non-empty absolute path")
    path = Path(path_value)
    if not path.is_absolute() or not path.is_file():
        raise SystemExit(f"{label}.path is not an existing absolute file: {path}")
    # A successful exact Cargo invocation can legitimately leave redirected
    # stdout empty because normal build progress is written to stderr.  The
    # command exit code and invocation are validated separately; log integrity
    # is the exact byte count and SHA-256, including the empty-file digest.
    size = path.stat().st_size
    if size == 0 and not allow_empty:
        raise SystemExit(f"{label} is unexpectedly empty")
    digest = sha256(path)
    if item.get("bytes") != size or str(item.get("sha256", "")).upper() != digest:
        raise SystemExit(f"{label} does not match the current log bytes")
    return {"path": str(path.resolve()), "bytes": size, "sha256": digest}


def _is_json_integer(value: object) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def _validate_exact_drive_bytes(value: object, label: str) -> dict[str, int]:
    if not isinstance(value, dict) or set(value) != {"C", "D"}:
        raise SystemExit(f"{label} must report exact byte counts for C and D")
    if any(not _is_json_integer(item) or item < 0 for item in value.values()):
        raise SystemExit(f"{label} must contain non-negative integer byte counts")
    return {"C": int(value["C"]), "D": int(value["D"])}


def _validate_build_disk_watcher(
    value: object,
    *,
    started_at: datetime,
    completed_at: datetime,
) -> None:
    if not isinstance(value, dict) or set(value) != {
        "policy",
        "preflight",
        "samples",
        "breach_detected",
        "exact_pid_tree_only",
    }:
        raise SystemExit("Build disk watcher does not match the frozen schema")
    if value.get("breach_detected") is not False or value.get("exact_pid_tree_only") is not True:
        raise SystemExit("Build disk watcher must prove a breach-free exact-PID-tree build")

    policy = value.get("policy")
    expected_policy = {
        "minimum_free_gib_exclusive": BUILD_DISK_FLOOR_GIB,
        "minimum_free_bytes_exclusive": BUILD_DISK_FLOOR_BYTES,
        "preflight_reserve_gib": BUILD_PREFLIGHT_RESERVE_GIB,
        "preflight_required_free_gib_exclusive": BUILD_PREFLIGHT_REQUIRED_GIB,
        "preflight_required_free_bytes_exclusive": BUILD_PREFLIGHT_REQUIRED_BYTES,
        "poll_interval_ms": BUILD_DISK_POLL_INTERVAL_MS,
        "breach_action": BUILD_DISK_BREACH_ACTION,
    }
    if policy != expected_policy:
        raise SystemExit("Build disk-watcher policy does not match the frozen release-safety policy")

    preflight = value.get("preflight")
    if not isinstance(preflight, dict) or set(preflight) != {
        "captured_at",
        "observed_free_bytes",
        "required_free_bytes_exclusive",
        "required_free_gib_exclusive",
        "passed",
    }:
        raise SystemExit("Build disk-watcher preflight does not match the frozen schema")
    if (
        preflight.get("passed") is not True
        or preflight.get("required_free_bytes_exclusive") != BUILD_PREFLIGHT_REQUIRED_BYTES
        or preflight.get("required_free_gib_exclusive") != BUILD_PREFLIGHT_REQUIRED_GIB
    ):
        raise SystemExit("Build disk-watcher preflight does not bind the required reserve thresholds")
    preflight_at = _parse_utc_timestamp(preflight.get("captured_at"), "build disk preflight captured_at")
    if preflight_at > started_at:
        raise SystemExit("Build disk preflight must complete before the build-session start")
    observed = _validate_exact_drive_bytes(
        preflight.get("observed_free_bytes"), "build disk preflight observed_free_bytes"
    )
    if any(observed[drive] <= required for drive, required in BUILD_PREFLIGHT_REQUIRED_BYTES.items()):
        raise SystemExit("Build disk preflight did not retain the required C/D reserve")

    samples = value.get("samples")
    if not isinstance(samples, list) or not samples:
        raise SystemExit("Build disk watcher must contain command-bound samples")
    expected_command_ids = ("tauri_desktop_bundle", "locked_release_cli")
    states_by_command: dict[str, list[str]] = {command_id: [] for command_id in expected_command_ids}
    root_pid_by_command: dict[str, int] = {}
    previous_time = started_at
    previous_command_index = 0
    for index, sample in enumerate(samples):
        label = f"build disk watcher sample {index}"
        if not isinstance(sample, dict) or set(sample) != {
            "captured_at",
            "command_id",
            "root_pid",
            "process_tree_pids",
            "state",
            "free_bytes",
            "floor_breached",
        }:
            raise SystemExit(f"{label} does not match the frozen schema")
        command_id = sample.get("command_id")
        if command_id not in states_by_command:
            raise SystemExit(f"{label} has an unknown command_id")
        command_index = expected_command_ids.index(command_id)
        if command_index < previous_command_index:
            raise SystemExit("Build disk-watcher command samples are not in execution order")
        previous_command_index = command_index
        state = sample.get("state")
        if state not in {"running", "completed"}:
            raise SystemExit(f"{label}.state must be running or completed")
        root_pid = sample.get("root_pid")
        if not _is_json_integer(root_pid) or root_pid <= 0:
            raise SystemExit(f"{label}.root_pid must be a positive integer")
        bound_root_pid = root_pid_by_command.setdefault(command_id, root_pid)
        if root_pid != bound_root_pid:
            raise SystemExit(f"{label}.root_pid changed within one build command")
        process_tree = sample.get("process_tree_pids")
        if (
            not isinstance(process_tree, list)
            or not process_tree
            or any(not _is_json_integer(pid) or pid <= 0 for pid in process_tree)
            or process_tree != sorted(set(process_tree))
            or root_pid not in process_tree
        ):
            raise SystemExit(f"{label}.process_tree_pids is not a sorted exact PID set containing the root")
        free_bytes = _validate_exact_drive_bytes(sample.get("free_bytes"), f"{label}.free_bytes")
        if sample.get("floor_breached") is not False or any(
            free_bytes[drive] <= BUILD_DISK_FLOOR_BYTES for drive in ("C", "D")
        ):
            raise SystemExit(f"{label} reached the strict 20 GiB release floor")
        captured_at = _parse_utc_timestamp(sample.get("captured_at"), f"{label}.captured_at")
        if captured_at < previous_time or captured_at > completed_at:
            raise SystemExit("Build disk-watcher sample timestamps are outside the ordered build interval")
        previous_time = captured_at
        states_by_command[command_id].append(state)

    for command_id, states in states_by_command.items():
        if not states or states[-1] != "completed" or states.count("completed") != 1 or "running" not in states:
            raise SystemExit(
                f"Build disk watcher must contain running samples followed by one completed sample for {command_id}"
            )


def validate_build_session(
    path: Path,
    *,
    root: Path,
    release_dir: Path,
    version: str,
    source: dict[str, Any],
) -> dict[str, Any]:
    """Verify the wrapper's fresh-target build invocation before preserving artifacts."""

    receipt_identity_before = _file_identity(path.resolve())
    session = _read_json(path.resolve(), "unsigned candidate build session")
    required = {
        "schema_version",
        "suite_id",
        "passed",
        "target_release",
        "source",
        "target_directory",
        "target_preexisting",
        "started_at_utc",
        "completed_at_utc",
        "environment",
        "commands",
        "minimum_free_gib",
        "disk_snapshots",
        "disk_watcher",
    }
    if set(session) != required:
        raise SystemExit("Unsigned candidate build-session keys do not match the frozen schema")
    if (
        session.get("schema_version") != BUILD_SESSION_SCHEMA
        or session.get("suite_id") != BUILD_SESSION_SUITE
        or session.get("passed") is not True
        or session.get("target_release") != version
        or session.get("target_preexisting") is not False
        or session.get("environment") != {"CARGO_INCREMENTAL": "0"}
        or session.get("minimum_free_gib") != BUILD_DISK_FLOOR_GIB
    ):
        raise SystemExit("Unsigned candidate build session is not a passing fresh-target session")
    session_source = session.get("source")
    if session_source != source:
        raise SystemExit("Build-session source provenance does not match the current clean source")
    target = Path(str(session.get("target_directory", "")))
    if not target.is_absolute() or target.resolve() != release_dir.parent.resolve():
        raise SystemExit("Build-session target directory does not own the supplied release directory")
    started = session.get("started_at_utc")
    completed = session.get("completed_at_utc")
    started_at = _parse_utc_timestamp(started, "build started_at_utc")
    completed_at = _parse_utc_timestamp(completed, "build completed_at_utc")
    if completed_at <= started_at:
        raise SystemExit("Build-session timestamps are not strictly ordered")
    _validate_build_disk_watcher(
        session.get("disk_watcher"),
        started_at=started_at,
        completed_at=completed_at,
    )

    commands = session.get("commands")
    if not isinstance(commands, list) or len(commands) != 2:
        raise SystemExit("Build session must contain exactly the desktop and CLI build commands")
    disk_snapshots = session.get("disk_snapshots")
    if not isinstance(disk_snapshots, list) or len(disk_snapshots) != 2:
        raise SystemExit("Build session must contain exactly two disk-space snapshots")
    for snapshot in disk_snapshots:
        if not isinstance(snapshot, dict) or set(snapshot) != {"label", "captured_at", "drives"}:
            raise SystemExit("Build disk-space snapshot does not match the frozen schema")
        drives = snapshot.get("drives")
        if not isinstance(drives, dict) or set(drives) != {"C", "D"}:
            raise SystemExit("Build disk-space snapshot must report exactly C and D")
        if any(not isinstance(value, (int, float)) or value <= 20.0 for value in drives.values()):
            raise SystemExit("Build disk-space snapshot does not retain strictly more than 20 GiB")

    expected = (
        (
            "tauri_desktop_bundle",
            {"npm", "npm.cmd", "npm.exe"},
            ["run", "tauri", "--", "build", "--bundles", "nsis", "--ci", "--", "--locked"],
        ),
        ("locked_release_cli", {"cargo", "cargo.exe"}, ["build", "--locked", "--release", "-p", "qpls-cli"]),
    )
    normalized: list[dict[str, Any]] = []
    for index, (command, (expected_id, expected_names, expected_arguments)) in enumerate(zip(commands, expected, strict=True)):
        if not isinstance(command, dict):
            raise SystemExit(f"Build command {index} is not an object")
        if set(command) != {"id", "executable", "arguments", "exit_code", "stdout", "stderr"}:
            raise SystemExit(f"Build command {index} keys do not match the frozen schema")
        executable = Path(str(command.get("executable", "")))
        if (
            command.get("id") != expected_id
            or command.get("arguments") != expected_arguments
            or command.get("exit_code") != 0
            or not executable.is_absolute()
            or not executable.is_file()
            or executable.name.lower() not in expected_names
        ):
            raise SystemExit(f"Build command {expected_id} did not record the exact successful invocation")
        normalized.append(
            {
                "id": expected_id,
                "executable": str(executable.resolve()),
                "arguments": expected_arguments,
                "exit_code": 0,
                "stdout": _validate_log_binding(
                    command.get("stdout"),
                    f"{expected_id}.stdout",
                    allow_empty=expected_id == "locked_release_cli",
                ),
                "stderr": _validate_log_binding(command.get("stderr"), f"{expected_id}.stderr"),
            }
        )
    receipt_identity_after = _file_identity(path.resolve())
    if receipt_identity_after != receipt_identity_before:
        raise SystemExit("Unsigned candidate build-session receipt changed while it was validated")
    return {
        **session,
        "target_directory": str(target.resolve()),
        "commands": normalized,
        "receipt_path": str(path.resolve()),
        "receipt_sha256": receipt_identity_after[1],
    }


def package_release_artifacts(
    *,
    root: Path = ROOT,
    release_dir: Path = RELEASE_DIR,
    artifact_dir: Path = ARTIFACT_DIR,
    report_path: Path = REPORT,
    build_session_path: Path,
    channel: str = "unsigned-preview",
    label: str = "manual_release",
    timestamp: str | None = None,
) -> dict[str, Any]:
    root = root.resolve()
    release_dir = release_dir.resolve()
    artifact_dir = artifact_dir.resolve()
    report_path = report_path.resolve()
    if report_path.exists():
        raise SystemExit(f"Refusing to overwrite release artifact report: {report_path}")
    version, version_contract = read_version_contract(root)
    source_provenance = read_clean_source_provenance(root)
    build_session = validate_build_session(
        build_session_path,
        root=root,
        release_dir=release_dir,
        version=version,
        source=source_provenance,
    )
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
    build_started_epoch = _parse_utc_timestamp(
        build_session["started_at_utc"], "build started_at_utc"
    ).timestamp()
    build_completed_epoch = _parse_utc_timestamp(
        build_session["completed_at_utc"], "build completed_at_utc"
    ).timestamp()
    for role, source, _destination in sources:
        if not source.is_file():
            raise SystemExit(f"Missing {role} build output: {source}")
        modified = source.stat().st_mtime
        if modified < build_started_epoch - 2.0 or modified > build_completed_epoch + 2.0:
            raise SystemExit(
                f"{role} build output timestamp falls outside the recorded fresh build session: {source}"
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
        if read_clean_source_provenance(root) != source_provenance:
            raise SystemExit("Clean source provenance changed while release artifacts were packaged")
    except BaseException:
        for created_path in reversed(created):
            if created_path.exists():
                created_path.unlink()
        raise

    report = {
        "schema_version": 3,
        "target": "QuickPLS unsigned preview artifact preservation",
        "passed": True,
        "version": version,
        "version_contract": version_contract,
        "source": source_provenance,
        "build": build_session,
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
    report_text = json.dumps(report, indent=2) + "\n"
    report_created = False
    try:
        report_path.parent.mkdir(parents=True, exist_ok=True)
        with report_path.open("x", encoding="utf-8", newline="\n") as handle:
            report_created = True
            handle.write(report_text)
        if report_path.read_text(encoding="utf-8") != report_text:
            raise SystemExit(f"Release artifact report read-back mismatch: {report_path}")
    except BaseException:
        if report_created and report_path.exists():
            report_path.unlink()
        for created_path in reversed(created):
            if created_path.exists():
                created_path.unlink()
        raise
    return report


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--source-provenance-only",
        action="store_true",
        help="Print the exact clean Git source provenance and do not package artifacts.",
    )
    parser.add_argument(
        "--channel",
        default="unsigned-preview",
        choices=sorted(EXPECTED_CHANNEL_POLICY),
        help="Artifact channel. Beta and stable are deliberately rejected by this unsigned factory.",
    )
    parser.add_argument("--label", default="manual_release", help="Milestone/build label to include in artifact names.")
    parser.add_argument("--timestamp", default=None, help="Optional UTC timestamp override, e.g. 20260722-120000.")
    parser.add_argument("--release-dir", type=Path, default=RELEASE_DIR)
    parser.add_argument("--artifact-dir", type=Path, default=ARTIFACT_DIR)
    parser.add_argument("--report", type=Path, default=REPORT)
    parser.add_argument(
        "--build-session",
        type=Path,
        help="Mandatory passing build-session receipt created by the isolated candidate-build wrapper.",
    )
    args = parser.parse_args()
    if args.source_provenance_only:
        if args.build_session is not None:
            parser.error("--source-provenance-only cannot be combined with --build-session")
        print(json.dumps(read_clean_source_provenance(ROOT), indent=2))
        return
    if args.build_session is None:
        parser.error("--build-session is required for release artifact packaging")
    report = package_release_artifacts(
        release_dir=args.release_dir,
        artifact_dir=args.artifact_dir,
        report_path=args.report,
        build_session_path=args.build_session,
        channel=args.channel,
        label=args.label,
        timestamp=args.timestamp,
    )
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
