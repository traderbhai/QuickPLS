"""Deterministic source discovery and build-receipt binding for the diagnostic gate.

The build subcommand is the only supported way to create the receipt consumed by
packaged diagnostic acceptance. Snapshot/finish-gate are read-only and fail
closed when source, generated frontend assets, Cargo dep-info, or the executable
do not match that receipt.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import stat
import subprocess
import sys
import time
import tomllib
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable, Sequence


ROOT = Path(__file__).resolve().parents[1]
DISCOVERY_CONTRACT = "quickpls_diagnostic_packaged_source_manifest_v1"
RECEIPT_SCHEMA = "quickpls.diagnostic_bundle_build_receipt.v1"
SNAPSHOT_SCHEMA = "quickpls.diagnostic_bundle_gate_source_snapshot.v1"
EVIDENCE_SCHEMA = "quickpls.diagnostic_bundle_source_evidence.v1"
BUILD_COMMAND = ("npm.cmd", "run", "tauri", "--", "build")
DESKTOP_PATH = "target/release/quickpls-desktop.exe"
DEP_INFO_PATH = "target/release/quickpls-desktop.d"
RECEIPT_PATH = "validation/results/diagnostic_bundle_build_receipt.json"
VITE_CONFIG_PRECEDENCE = (
    "vite.config.js",
    "vite.config.mjs",
    "vite.config.ts",
    "vite.config.cjs",
    "vite.config.mts",
    "vite.config.cts",
)
PRODUCTION_ENV_FILES = (
    ".env",
    ".env.local",
    ".env.production",
    ".env.production.local",
)
FIXED_PRODUCT_PATHS = (
    "Cargo.lock",
    "Cargo.toml",
    "index.html",
    "package-lock.json",
    "package.json",
    "tsconfig.app.json",
    "tsconfig.json",
    "tsconfig.node.json",
)
REQUIRED_EXTERNAL_PRODUCT_PATHS = (
    "THIRD_PARTY_NOTICES.md",
    "validation/development_slices.json",
    "validation/fixtures/corporate_reputation.csv",
    "validation/fixtures/mediation_sample.csv",
    "validation/fixtures/simple_reflective.csv",
)
GATE_ONLY_PATHS = (
    "validation/close_tauri_test_window.mjs",
    "validation/diagnostic_bundle_packaged_acceptance.mjs",
    "validation/diagnostic_bundle_packaged_acceptance.py",
    "validation/diagnostic_bundle_packaged_acceptance.schema.json",
    "validation/diagnostic_bundle_source_manifest.py",
    "validation/monitor_quickpls_network.ps1",
    "validation/monitor_quickpls_process_tree.ps1",
    "validation/run_diagnostic_bundle_packaged_acceptance.ps1",
    "validation/test_diagnostic_bundle_packaged_acceptance.py",
    "validation/windows_native_save_diagnostic_bundle.py",
)
REPARSE_POINT_ATTRIBUTE = 0x400


class SourceManifestFailure(RuntimeError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SourceManifestFailure(message)


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def canonical_json_bytes(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"), sort_keys=True).encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _has_reparse_point(path: Path) -> bool:
    info = path.lstat()
    return path.is_symlink() or bool(getattr(info, "st_file_attributes", 0) & REPARSE_POINT_ATTRIBUTE)


def _assert_safe_path(path: Path, root: Path, *, require_file: bool = False) -> Path:
    lexical_root = root.absolute()
    absolute = path.absolute()
    try:
        lexical_relative = absolute.relative_to(lexical_root)
    except ValueError:
        lexical_relative = None
    if lexical_relative is not None:
        cursor = lexical_root
        for part in lexical_relative.parts:
            cursor = cursor / part
            require(cursor.exists(), f"Required path is missing: {lexical_relative.as_posix()}")
            require(not _has_reparse_point(cursor), f"Reparse/symlink paths are forbidden: {lexical_relative.as_posix()}")
    root = root.resolve()
    resolved = absolute.resolve(strict=True)
    try:
        relative = resolved.relative_to(root)
    except ValueError as error:
        raise SourceManifestFailure(f"Resolved path escapes repository: {path}") from error
    cursor = root
    for part in relative.parts:
        cursor = cursor / part
        require(not _has_reparse_point(cursor), f"Reparse/symlink paths are forbidden: {relative.as_posix()}")
    if require_file:
        mode = resolved.stat().st_mode
        require(stat.S_ISREG(mode), f"Source artifact is not a regular file: {relative.as_posix()}")
    return resolved


def normalize_relative(path: Path | str, root: Path = ROOT) -> str:
    root = root.resolve()
    candidate = Path(path)
    if not candidate.is_absolute():
        candidate = root / candidate
    resolved = _assert_safe_path(candidate, root)
    relative = resolved.relative_to(root).as_posix()
    require(relative and not relative.startswith("/") and "\\" not in relative, f"Unsafe relative path: {relative}")
    require(".." not in Path(relative).parts, f"Escaping relative path: {relative}")
    return relative


def _validate_unique_paths(paths: Iterable[str], label: str) -> tuple[str, ...]:
    ordered = tuple(sorted(paths))
    require(len(ordered) == len(set(ordered)), f"Duplicate {label} paths were discovered")
    folded: dict[str, str] = {}
    for relative in ordered:
        previous = folded.setdefault(relative.casefold(), relative)
        require(previous == relative, f"Case-fold duplicate {label} paths: {previous} and {relative}")
    return ordered


def _files_under(directory: Path, root: Path) -> tuple[str, ...]:
    if not directory.exists():
        return ()
    _assert_safe_path(directory, root)
    paths: list[str] = []
    for candidate in directory.rglob("*"):
        if candidate.is_dir() and _has_reparse_point(candidate):
            raise SourceManifestFailure(f"Reparse/symlink directory is forbidden: {candidate}")
        if candidate.is_file():
            _assert_safe_path(candidate, root, require_file=True)
            paths.append(normalize_relative(candidate, root))
    return _validate_unique_paths(paths, f"files under {normalize_relative(directory, root)}")


def _cargo_dependency_tables(manifest: dict[str, Any]) -> Iterable[dict[str, Any]]:
    for key in ("dependencies", "build-dependencies"):
        value = manifest.get(key)
        if isinstance(value, dict):
            yield value
    target = manifest.get("target")
    if isinstance(target, dict):
        for target_value in target.values():
            if not isinstance(target_value, dict):
                continue
            for key in ("dependencies", "build-dependencies"):
                value = target_value.get(key)
                if isinstance(value, dict):
                    yield value


def discover_desktop_cargo_packages(root: Path = ROOT) -> tuple[str, ...]:
    root = root.resolve()
    initial = _assert_safe_path(root / "src-tauri/Cargo.toml", root, require_file=True)
    pending = [initial]
    discovered: set[str] = set()
    while pending:
        manifest_path = pending.pop()
        manifest_relative = normalize_relative(manifest_path, root)
        if manifest_relative in discovered:
            continue
        discovered.add(manifest_relative)
        document = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
        for table in _cargo_dependency_tables(document):
            for specification in table.values():
                if not isinstance(specification, dict) or "path" not in specification:
                    continue
                package_root = (manifest_path.parent / str(specification["path"])).absolute()
                dependency_manifest = _assert_safe_path(package_root / "Cargo.toml", root, require_file=True)
                if normalize_relative(dependency_manifest, root) not in discovered:
                    pending.append(dependency_manifest)
    return _validate_unique_paths(discovered, "Cargo package manifests")


def _tauri_selected_paths(config_path: Path, root: Path) -> tuple[str, ...]:
    config = json.loads(config_path.read_text(encoding="utf-8"))
    bundle = config.get("bundle", {})
    paths: list[str] = []
    for relative in bundle.get("icon", []) or []:
        paths.append(normalize_relative(config_path.parent / str(relative), root))
    resources = bundle.get("resources", {}) or {}
    values = resources.keys() if isinstance(resources, dict) else resources
    for relative in values:
        candidate = (config_path.parent / str(relative)).absolute()
        safe = _assert_safe_path(candidate, root)
        if safe.is_dir():
            paths.extend(_files_under(safe, root))
        else:
            _assert_safe_path(safe, root, require_file=True)
            paths.append(normalize_relative(safe, root))
    return _validate_unique_paths(paths, "Tauri selected resource")


def discover_product_source(root: Path = ROOT) -> dict[str, Any]:
    root = root.resolve()
    paths: set[str] = set()
    for relative in FIXED_PRODUCT_PATHS + REQUIRED_EXTERNAL_PRODUCT_PATHS:
        paths.add(normalize_relative(_assert_safe_path(root / relative, root, require_file=True), root))

    package = json.loads((root / "package.json").read_text(encoding="utf-8"))
    package_scripts = package.get("scripts", {})
    package_build_script = package_scripts.get("build")
    package_tauri_script = package_scripts.get("tauri")
    require(package_build_script == "tsc -b && vite build", "Unsupported package build script")
    require(package_tauri_script == "tauri", "Unsupported package Tauri script")

    present_vite = tuple(name for name in VITE_CONFIG_PRECEDENCE if (root / name).is_file())
    require(present_vite, "No recognized Vite configuration is present")
    for relative in present_vite:
        paths.add(normalize_relative(_assert_safe_path(root / relative, root, require_file=True), root))
    present_env = tuple(name for name in PRODUCTION_ENV_FILES if (root / name).exists())
    for relative in present_env:
        paths.add(normalize_relative(_assert_safe_path(root / relative, root, require_file=True), root))

    paths.update(_files_under(root / "src", root))
    if (root / "public").exists():
        paths.update(_files_under(root / "public", root))

    cargo_manifests = discover_desktop_cargo_packages(root)
    for relative in cargo_manifests:
        manifest_path = root / relative
        package_root = manifest_path.parent
        paths.add(relative)
        build_script = package_root / "build.rs"
        if build_script.exists():
            paths.add(normalize_relative(_assert_safe_path(build_script, root, require_file=True), root))
        source_root = package_root / "src"
        require(source_root.is_dir(), f"Cargo package has no src directory: {relative}")
        paths.update(_files_under(source_root, root))

    tauri_root = root / "src-tauri"
    tauri_configs = _validate_unique_paths(
        (normalize_relative(path, root) for path in tauri_root.glob("tauri*.conf.json") if path.is_file()),
        "Tauri config",
    )
    require("src-tauri/tauri.conf.json" in tauri_configs, "Base Tauri configuration is missing")
    paths.update(tauri_configs)
    paths.update(_files_under(tauri_root / "capabilities", root))
    for relative in tauri_configs:
        paths.update(_tauri_selected_paths(root / relative, root))
    tauri_base = json.loads((root / "src-tauri/tauri.conf.json").read_text(encoding="utf-8"))
    tauri_build = tauri_base.get("build", {})
    tauri_before_build_command = tauri_build.get("beforeBuildCommand")
    tauri_frontend_dist = tauri_build.get("frontendDist")
    require(tauri_before_build_command == "npm run build", "Unsupported Tauri beforeBuildCommand")
    require(tauri_frontend_dist == "../dist", "Unsupported Tauri frontendDist")
    require(
        (tauri_root / tauri_frontend_dist).resolve() == (root / "dist").resolve(),
        "Tauri frontendDist does not resolve to repository dist",
    )

    ordered = _validate_unique_paths(paths, "product source")
    gate_overlap = sorted(set(ordered).intersection(GATE_ONLY_PATHS))
    require(not gate_overlap, f"Product/gate-only path overlap: {gate_overlap}")
    return {
        "paths": list(ordered),
        "vite_config_precedence": list(VITE_CONFIG_PRECEDENCE),
        "present_vite_configs": list(present_vite),
        "active_vite_config": present_vite[0],
        "present_production_env_files": list(present_env),
        "desktop_cargo_manifests": list(cargo_manifests),
        "tauri_configs": list(tauri_configs),
        "package_build_script": package_build_script,
        "package_tauri_script": package_tauri_script,
        "tauri_before_build_command": tauri_before_build_command,
        "tauri_frontend_dist": tauri_frontend_dist,
    }


def discover_gate_only(root: Path = ROOT) -> tuple[str, ...]:
    root = root.resolve()
    for relative in GATE_ONLY_PATHS:
        _assert_safe_path(root / relative, root, require_file=True)
    return _validate_unique_paths(GATE_ONLY_PATHS, "gate-only")


def describe_file(path: Path | str, root: Path = ROOT) -> dict[str, Any]:
    root = root.resolve()
    relative = normalize_relative(path, root)
    candidate = _assert_safe_path(root / relative, root, require_file=True)
    before = candidate.stat()
    digest = sha256_file(candidate)
    after = candidate.stat()
    require(
        before.st_size == after.st_size and before.st_mtime_ns == after.st_mtime_ns,
        f"File changed while hashed: {relative}",
    )
    require(after.st_size > 0, f"Zero-byte source/build artifact is forbidden: {relative}")
    return {
        "path": relative,
        "size": after.st_size,
        "sha256": digest,
        "mtime_ns": after.st_mtime_ns,
    }


def describe_paths(paths: Sequence[str], root: Path = ROOT) -> dict[str, Any]:
    ordered = _validate_unique_paths(paths, "manifest")
    descriptors = [describe_file(relative, root) for relative in ordered]
    return {
        "descriptors": descriptors,
        "manifest_sha256": sha256_bytes(canonical_json_bytes(descriptors)),
    }


def capture_product_source(root: Path = ROOT) -> dict[str, Any]:
    discovery = discover_product_source(root)
    manifest = describe_paths(discovery["paths"], root)
    return {"discovery": discovery, **manifest}


def capture_gate_only(root: Path = ROOT) -> dict[str, Any]:
    return describe_paths(discover_gate_only(root), root)


def capture_dist(root: Path = ROOT) -> dict[str, Any]:
    paths = _files_under(root.resolve() / "dist", root.resolve())
    require(paths, "dist contains no regular files")
    return describe_paths(paths, root)


def _parse_dep_info_paths(dep_info: Path, root: Path) -> tuple[str, ...]:
    text = dep_info.read_text(encoding="utf-8")
    marker = ".exe:"
    marker_index = text.find(marker)
    require(marker_index >= 0, "Cargo dep-info does not identify quickpls-desktop.exe")
    dependencies = text[marker_index + len(marker):].strip().split()
    require(dependencies, "Cargo dep-info has no dependencies")
    paths: list[str] = []
    for token in dependencies:
        candidate = Path(token.replace("\\ ", " "))
        if not candidate.is_absolute():
            candidate = root / candidate
        try:
            safe = _assert_safe_path(candidate, root)
        except SourceManifestFailure:
            continue
        if safe.is_dir():
            paths.extend(_files_under(safe, root))
        elif safe.is_file():
            paths.append(normalize_relative(safe, root))
    return _validate_unique_paths(paths, "Cargo dep-info repository dependency")


def capture_dep_info(root: Path = ROOT) -> dict[str, Any]:
    root = root.resolve()
    dep_info_path = _assert_safe_path(root / DEP_INFO_PATH, root, require_file=True)
    repository_paths = _parse_dep_info_paths(dep_info_path, root)
    dist_paths = tuple(path for path in repository_paths if path == "dist" or path.startswith("dist/"))
    current_dist_paths = tuple(row["path"] for row in capture_dist(root)["descriptors"])
    require(
        dist_paths == current_dist_paths,
        f"Cargo dep-info dist dependency set differs from current dist: dep-info={dist_paths!r}, current={current_dist_paths!r}",
    )
    for required in REQUIRED_EXTERNAL_PRODUCT_PATHS:
        require(required in repository_paths, f"Cargo dep-info omits required embedded/resource input: {required}")
    return {
        "descriptor": describe_file(dep_info_path, root),
        "repository_dependency_paths": list(repository_paths),
        "dist_dependency_paths": list(dist_paths),
        "dist_set_exact": True,
    }


def source_freshness(
    product_source: dict[str, Any],
    dist: dict[str, Any],
    desktop: dict[str, Any],
    *,
    build_started_unix_ns: int,
    build_finished_unix_ns: int,
) -> dict[str, Any]:
    inputs = product_source["descriptors"] + dist["descriptors"]
    require(inputs, "Freshness calculation has no product/build inputs")
    newest_mtime = max(row["mtime_ns"] for row in inputs)
    newest_path = min(row["path"] for row in inputs if row["mtime_ns"] == newest_mtime)
    desktop_mtime = desktop["mtime_ns"]
    passed = (
        desktop_mtime >= newest_mtime
        and desktop_mtime >= build_started_unix_ns
        and desktop_mtime <= build_finished_unix_ns
    )
    return {
        "passed": passed,
        "tested_desktop_path": desktop["path"],
        "tested_desktop_mtime_unix_ns": desktop_mtime,
        "newest_product_input_path": newest_path,
        "newest_product_input_mtime_unix_ns": newest_mtime,
        "build_started_unix_ns": build_started_unix_ns,
        "build_finished_unix_ns": build_finished_unix_ns,
        "desktop_not_older_than_every_product_input": desktop_mtime >= newest_mtime,
        "desktop_created_during_recorded_build": (
            desktop_mtime >= build_started_unix_ns and desktop_mtime <= build_finished_unix_ns
        ),
    }


def _load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    require(isinstance(value, dict), f"Expected JSON object: {path}")
    return value


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    temporary.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8", newline="\n")
    temporary.replace(path)


def validate_build_receipt(receipt: dict[str, Any], root: Path = ROOT) -> None:
    root = root.resolve()
    require(receipt.get("schema_version") == RECEIPT_SCHEMA, "Build receipt schema drifted")
    require(receipt.get("kind") == "quickpls_diagnostic_packaged_frozen_build_v1", "Build receipt kind drifted")
    require(receipt.get("passed") is True, "Build receipt is not passing")
    require(receipt.get("build_command") == list(BUILD_COMMAND), "Build command drifted")
    require(receipt.get("build_exit_code") == 0, "Build exit code is not zero")
    require(receipt.get("source_stable_during_build") is True, "Source was not stable during build")
    require(receipt.get("dist_bound_to_dep_info") is True, "dist was not bound to Cargo dep-info")
    current_source = capture_product_source(root)
    require(receipt.get("source_before") == receipt.get("source_after"), "Build source before/after differs")
    require(receipt.get("source_after") == current_source, "Current product source differs from build receipt")
    current_dist = capture_dist(root)
    require(receipt.get("dist_after") == current_dist, "Current dist differs from build receipt")
    current_dep_info = capture_dep_info(root)
    require(receipt.get("cargo_dep_info") == current_dep_info, "Current Cargo dep-info differs from build receipt")
    current_desktop = describe_file(DESKTOP_PATH, root)
    require(receipt.get("tested_desktop") == current_desktop, "Current desktop executable differs from build receipt")
    freshness = source_freshness(
        current_source,
        current_dist,
        current_desktop,
        build_started_unix_ns=receipt["build_started_unix_ns"],
        build_finished_unix_ns=receipt["build_finished_unix_ns"],
    )
    require(receipt.get("freshness") == freshness and freshness["passed"], "Build receipt freshness failed")


def create_build_receipt(receipt_path: Path, root: Path = ROOT) -> dict[str, Any]:
    root = root.resolve()
    require(
        receipt_path.resolve() == (root / RECEIPT_PATH).resolve(),
        f"Build receipt must use the fixed repository path: {RECEIPT_PATH}",
    )
    receipt_path.unlink(missing_ok=True)
    source_before = capture_product_source(root)
    build_started_unix_ns = time.time_ns()
    build_started_at_utc = utc_now()
    completed = subprocess.run(BUILD_COMMAND, cwd=root, check=False)
    build_finished_unix_ns = time.time_ns()
    build_finished_at_utc = utc_now()
    require(completed.returncode == 0, f"Frozen Tauri build failed with exit code {completed.returncode}")
    source_after = capture_product_source(root)
    require(source_before == source_after, "Product source changed during the coordinated build")
    dist_after = capture_dist(root)
    dep_info = capture_dep_info(root)
    desktop = describe_file(DESKTOP_PATH, root)
    freshness = source_freshness(
        source_after,
        dist_after,
        desktop,
        build_started_unix_ns=build_started_unix_ns,
        build_finished_unix_ns=build_finished_unix_ns,
    )
    require(freshness["passed"], "Built desktop failed source/dist freshness binding")
    receipt = {
        "schema_version": RECEIPT_SCHEMA,
        "kind": "quickpls_diagnostic_packaged_frozen_build_v1",
        "passed": True,
        "generated_at_utc": utc_now(),
        "build_command": list(BUILD_COMMAND),
        "build_started_at_utc": build_started_at_utc,
        "build_finished_at_utc": build_finished_at_utc,
        "build_started_unix_ns": build_started_unix_ns,
        "build_finished_unix_ns": build_finished_unix_ns,
        "build_exit_code": completed.returncode,
        "source_before": source_before,
        "source_after": source_after,
        "source_stable_during_build": True,
        "dist_after": dist_after,
        "cargo_dep_info": dep_info,
        "tested_desktop": desktop,
        "dist_bound_to_dep_info": True,
        "freshness": freshness,
    }
    write_json(receipt_path, receipt)
    validate_build_receipt(receipt, root)
    return receipt


def capture_gate_snapshot(receipt_path: Path, root: Path = ROOT) -> dict[str, Any]:
    root = root.resolve()
    receipt_path = _assert_safe_path(receipt_path, root, require_file=True)
    receipt = _load_json(receipt_path)
    validate_build_receipt(receipt, root)
    snapshot = {
        "schema_version": SNAPSHOT_SCHEMA,
        "discovery_contract": DISCOVERY_CONTRACT,
        "product_source": capture_product_source(root),
        "gate_only": capture_gate_only(root),
        "dist": capture_dist(root),
        "cargo_dep_info": capture_dep_info(root),
        "tested_desktop": describe_file(DESKTOP_PATH, root),
        "build_receipt": describe_file(receipt_path, root),
        "freshness": receipt["freshness"],
    }
    return snapshot


def _stable_snapshot(value: dict[str, Any]) -> dict[str, Any]:
    return {key: item for key, item in value.items() if key != "captured_at_utc"}


def finish_gate_evidence(
    receipt_path: Path,
    before_path: Path,
    output_path: Path,
    root: Path = ROOT,
) -> dict[str, Any]:
    root = root.resolve()
    before = _load_json(_assert_safe_path(before_path, root, require_file=True))
    after = capture_gate_snapshot(receipt_path, root)
    require(_stable_snapshot(before) == _stable_snapshot(after), "Source/build artifacts changed during packaged gate")
    receipt = _load_json(_assert_safe_path(receipt_path, root, require_file=True))
    evidence = {
        "schema_version": EVIDENCE_SCHEMA,
        "discovery_contract": DISCOVERY_CONTRACT,
        "build_receipt_path": normalize_relative(receipt_path, root),
        "build_receipt": receipt,
        "before": before,
        "after": after,
        "source_stable_during_gate": True,
        "freshness": after["freshness"],
    }
    validate_gate_evidence(evidence, root)
    write_json(output_path, evidence)
    return evidence


def validate_gate_evidence(evidence: dict[str, Any], root: Path = ROOT) -> None:
    root = root.resolve()
    require(evidence.get("schema_version") == EVIDENCE_SCHEMA, "Source evidence schema drifted")
    require(evidence.get("discovery_contract") == DISCOVERY_CONTRACT, "Source discovery contract drifted")
    require(evidence.get("source_stable_during_gate") is True, "Source was not stable during gate")
    receipt_path_value = evidence.get("build_receipt_path")
    require(isinstance(receipt_path_value, str), "Build receipt path is missing")
    receipt_path = _assert_safe_path(root / receipt_path_value, root, require_file=True)
    receipt = _load_json(receipt_path)
    require(evidence.get("build_receipt") == receipt, "Embedded build receipt differs from receipt artifact")
    validate_build_receipt(receipt, root)
    current = capture_gate_snapshot(receipt_path, root)
    before = evidence.get("before")
    after = evidence.get("after")
    require(isinstance(before, dict) and isinstance(after, dict), "Gate source snapshots are missing")
    require(_stable_snapshot(before) == _stable_snapshot(after), "Before/after source snapshots differ")
    require(_stable_snapshot(after) == _stable_snapshot(current), "Current source snapshot differs from gate evidence")
    require(evidence.get("freshness") == current["freshness"], "Gate freshness evidence drifted")
    require(current["freshness"]["passed"], "Tested desktop is stale")


def main() -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    build_parser = subparsers.add_parser("build", help="Run the one coordinated Tauri build and write its receipt")
    build_parser.add_argument("--receipt", type=Path, default=ROOT / RECEIPT_PATH)
    snapshot_parser = subparsers.add_parser("snapshot", help="Verify the receipt and capture gate-start source state")
    snapshot_parser.add_argument("--receipt", type=Path, default=ROOT / RECEIPT_PATH)
    snapshot_parser.add_argument("--output", type=Path, required=True)
    finish_parser = subparsers.add_parser("finish-gate", help="Capture gate-end state and require byte stability")
    finish_parser.add_argument("--receipt", type=Path, default=ROOT / RECEIPT_PATH)
    finish_parser.add_argument("--before", type=Path, required=True)
    finish_parser.add_argument("--output", type=Path, required=True)
    verify_parser = subparsers.add_parser("verify-receipt", help="Independently verify an existing build receipt")
    verify_parser.add_argument("--receipt", type=Path, default=ROOT / RECEIPT_PATH)
    args = parser.parse_args()
    try:
        if args.command == "build":
            outcome = create_build_receipt(args.receipt.resolve())
        elif args.command == "snapshot":
            outcome = capture_gate_snapshot(args.receipt.resolve())
            write_json(args.output.resolve(), outcome)
        elif args.command == "finish-gate":
            outcome = finish_gate_evidence(args.receipt.resolve(), args.before.resolve(), args.output.resolve())
        else:
            outcome = _load_json(args.receipt.resolve())
            validate_build_receipt(outcome)
        print(json.dumps({"passed": True, "command": args.command, "result": outcome}, indent=2))
        return 0
    except (SourceManifestFailure, OSError, ValueError, json.JSONDecodeError, tomllib.TOMLDecodeError) as error:
        print(str(error), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
