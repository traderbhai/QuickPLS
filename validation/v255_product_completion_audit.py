from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import tomllib
import zipfile
from datetime import datetime, timezone
from pathlib import Path, PurePosixPath

from v255_release_waiver import (
    DPI_WAIVER_CASE_ID,
    DPI_WAIVER_MANIFEST_DECLARATION,
    exact_approved_waiver_contract,
    exact_cross_report_waiver_binding,
    exact_population_status,
    exact_release_waiver_receipt,
    exact_release_waiver_matches_observation,
    exact_waived_index_entry,
    exact_waived_observation,
)


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = ROOT / "validation" / "results" / "v255_product_completion_audit.json"
EXPECTED_CONSOLIDATED_STEPS = [
    "diff_check",
    "v255_evidence_contract",
    "v255_rebased_contract",
    "frontend_full_vitest",
    "rust_authority",
    "rust_archive_schema6_authoring",
    "rust_archive_three_way",
    "rust_desktop_three_way",
    "frontend_typecheck",
    "frontend_build",
    "python_export_semantic_readback",
    "rebaselined_interactions",
    "method_setup_crawler",
    "v255_final_evidence_contract",
]
EXPECTED_SETUP_CONTRACT_SHA256 = (
    "4a506f59e5d7afc56b4d975ae567b547b276a8fc841b5a8c9a51836048534fe3"
)
EXPECTED_SPECIALIZED_CONTRACT_SHA256 = (
    "dfe09e79a231bffe5b88381dd3ee436d6ef7292571bfbdad3662607bb28062ba"
)
EXPECTED_CROSS_METHOD_CONTRACT_SHA256 = (
    "e894da0b67cca47224c16fddf8b6e13172c38027967b6ff8906c6cc278ee8ed1"
)
NAMED_ROUTE_MANIFEST_PATH = "validation/v255_named_case_manifest.json"
CROSS_METHOD_ROUTE_MANIFEST_PATH = "validation/v255_cross_method_case_manifest.json"
EXPECTED_TRUSTED_DRIVER_SUITES = {
    "quickpls_v255_live_calculation_lifecycle_smoke_v1": 1,
    "quickpls_v255_method_evidence_crawler_v2": 2,
    "quickpls_v255_frozen_archive_reopen_crawler_v1": 1,
    "quickpls_v255_posthoc_minimum_sample_packaged_smoke_v1": 1,
    "quickpls_v255_named_case_driver_v1": 1,
    "quickpls_v255_cross_method_candidate_wrapper_v1": 1,
}
EXPECTED_PREEXISTING_FIXED_CASE_IDS = {
    "cross_method:observability:setup screenshot",
    "cross_method:observability:completed Results screenshot",
    "cross_method:accessibility:1024x700",
    "cross_method:observability:machine-readable observation",
    "cross_method:observability:zero unexplained skip",
    "cross_method:observability:running or progress screenshot",
    "cross_method:persistence:save and fresh reopen",
    "cross_method:packaged:offline request observation",
}
EXPECTED_CROSS_METHOD_WRAPPER_CASE_IDS = {
    "cross_method:imports:CSV",
    "cross_method:imports:XLSX",
    "cross_method:imports:SPSS SAV",
    "cross_method:imports:ODS",
    "cross_method:exports:CSV",
    "cross_method:exports:XLSX",
    "cross_method:exports:HTML",
    "cross_method:exports:PDF",
    "cross_method:exports:SVG",
    "cross_method:exports:PNG",
    "cross_method:persistence:autosave recovery",
    "cross_method:persistence:legacy migration",
    "cross_method:persistence:future read-only archive",
    "cross_method:persistence:unsaved-close guard",
    "cross_method:accessibility:actual Windows 200 percent scaling",
    "cross_method:packaged:isolated local CDP",
    "cross_method:packaged:PID-scoped cleanup only",
}
NAMED_ROUTE_ACTIONS = {
    "goto_packaged",
    "create_project",
    "set_viewport",
    "set_view",
    "load_fixture",
    "load_named_sem_fixture",
    "inspect_archive_identity",
    "prepare_calculation_revision",
    "exercise_advanced_parameter_revision",
    "save_and_reopen_case_revision",
    "run_calculation",
    "open_archive",
    "select_result",
    "select_result_table",
    "wait_for",
    "click",
    "double_click",
    "fill",
    "select_option",
    "press",
    "native_file_dialog",
    "assert",
}
NAMED_ROUTE_QUERY_KINDS = {
    "viewport",
    "candidate_surface",
    "active_element",
    "calculation_dialog",
    "result_table",
    "specialized_result",
    "cfa_compatibility_result",
    "result_surface",
    "file",
    "text",
    "attribute",
    "count",
    "visible",
    "enabled",
    "input_value",
    "selected_text",
}
NAMED_SEM_FIXTURES = {
    "single_mediation",
    "parallel_mediation",
    "serial_mediation",
    "simultaneous_two_way",
    "three_way",
    "moderated_mediation_first",
    "moderated_mediation_second",
    "binary_moderation",
    "hoc_rr",
    "hoc_rf",
    "hoc_fr",
    "hoc_ff",
    "cfa",
    "recursive_sem",
}
NAMED_ROUTE_SUPPORT_FILES = {
    NAMED_ROUTE_MANIFEST_PATH,
    CROSS_METHOD_ROUTE_MANIFEST_PATH,
    "validation/v255_named_evidence_index.json",
    "validation/v255_named_case_driver.mjs",
    "validation/v255_named_archive_identity.py",
    "validation/v255_release_waiver.py",
    "validation/windows_native_owned_file_dialog.py",
    "src/data/v255NamedSemEvidenceFixtures.ts",
    "validation/run_v255_installed_portable_smoke.ps1",
}
POST_GATE_RELEASE_CONTENT_EXACT = {
    "CHANGELOG.md",
    "README.md",
    "validation/v255_evidence_bundle_manifest.json",
    "validation/v255_frozen_result_archive_index.json",
    "validation/v255_named_evidence_index.json",
}
POST_GATE_VERSION_ONLY_EXACT = {
    "Cargo.lock",
    "Cargo.toml",
    "package-lock.json",
    "package.json",
    "src-tauri/tauri.conf.json",
    "src/native/NativeDesktopApp.tsx",
    "src/v2/NativePrototypeApp.tsx",
    "validation/quickpls_release_channels.json",
    "validation/test_package_release_artifacts.py",
}
POST_GATE_VERSION_SUBSTITUTIONS = (
    ("2.54.0", "2.55.0"),
    ("v2_54_0_canvas_results", "v2_55_0_calculate_evidence"),
)
POST_GATE_VERSION_PATH_SUBSTITUTIONS = {
    "validation/test_package_release_artifacts.py": (("v2.53.0", "v2.54.0"),),
}
CANONICAL_TOOL_INPUTS = {
    "CargoPath": "cargo",
    "NpmPath": "npm.cmd",
    "NodePath": "node",
    "PythonPath": "python",
    "GitPath": "git",
}
CONSOLIDATED_DISK_RESERVES = {
    "frontend_full_vitest": 2.5,
    "rust_authority": 2.5,
    "rust_archive_schema6_authoring": 2.5,
    "rust_archive_three_way": 2.5,
    "rust_desktop_three_way": 2.5,
    "frontend_typecheck": 1.0,
    "frontend_build": 1.5,
    "python_export_semantic_readback": 0.5,
    "rebaselined_interactions": 0.5,
    "method_setup_crawler": 0.5,
}
BUILD_DISK_FLOOR_GIB = 20.0
BUILD_DISK_FLOOR_BYTES = 20 * 1024**3
BUILD_PREFLIGHT_RESERVE_GIB = {"C": 6.5, "D": 0.5}
BUILD_PREFLIGHT_REQUIRED_GIB = {"C": 26.5, "D": 20.5}
BUILD_PREFLIGHT_REQUIRED_BYTES = {
    "C": 26 * 1024**3 + 512 * 1024**2,
    "D": 20 * 1024**3 + 512 * 1024**2,
}
BUILD_DISK_POLL_INTERVAL_MS = 1000
BUILD_DISK_BREACH_ACTION = "terminate_only_exact_wrapper_owned_process_tree"
MAX_EVIDENCE_ZIP_BYTES = 2 * 1024 * 1024 * 1024
MAX_EVIDENCE_UNCOMPRESSED_BYTES = 4 * 1024 * 1024 * 1024
MAX_EVIDENCE_FILE_MEMBERS = 512
MAX_ARCHIVE_MEMBER_BYTES = 256 * 1024 * 1024
MAX_SCREENSHOT_MEMBER_BYTES = 64 * 1024 * 1024
MAX_RECEIPT_MEMBER_BYTES = 16 * 1024 * 1024
WINDOWS_RESERVED_MEMBER = re.compile(
    r"^(?:CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\.|$)", re.IGNORECASE
)
POST_GATE_EVIDENCE_SUFFIXES = {
    ".csv",
    ".html",
    ".json",
    ".log",
    ".md",
    ".pdf",
    ".png",
    ".qpls",
    ".sha256",
    ".svg",
    ".txt",
    ".xlsx",
}


def read_text(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


def read_json(relative: str) -> object:
    return json.loads(read_text(relative))


def sha256(relative: str) -> str:
    return hashlib.sha256((ROOT / relative).read_bytes()).hexdigest()


def sha256_path(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def is_sha256(value: object) -> bool:
    return isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value) is not None


def is_sha256_any_case(value: object) -> bool:
    return (
        isinstance(value, str) and re.fullmatch(r"[0-9a-fA-F]{64}", value) is not None
    )


def hash_matches(value: object, expected_lowercase: str) -> bool:
    return is_sha256_any_case(value) and str(value).lower() == expected_lowercase


def sha256_lines(values: list[str]) -> str:
    return hashlib.sha256("\n".join(values).encode("utf-8")).hexdigest()


def existing_repo_source_file(value: object) -> bool:
    if not isinstance(value, str) or not value or "\\" in value:
        return False
    declared = Path(value)
    if declared.is_absolute():
        return False
    candidate = (ROOT / declared).resolve()
    try:
        candidate.relative_to(ROOT.resolve())
    except ValueError:
        return False
    return candidate.is_file()


def git_tracked_repo_source_files(values: list[object]) -> bool:
    normalized = sorted(
        {
            str(value).replace("\\", "/")
            for value in values
            if isinstance(value, str) and value
        }
    )
    if not normalized or not all(existing_repo_source_file(value) for value in normalized):
        return False
    completed = subprocess.run(
        ["git", "-C", str(ROOT), "ls-files", "--error-unmatch", "--", *normalized],
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    tracked = {
        line.strip().replace("\\", "/")
        for line in completed.stdout.splitlines()
        if line.strip()
    }
    return completed.returncode == 0 and tracked == set(normalized)


def load_toml(relative: str) -> dict[str, object]:
    with (ROOT / relative).open("rb") as source:
        payload = tomllib.load(source)
    if not isinstance(payload, dict):
        raise ValueError(f"TOML root must be a table: {relative}")
    return payload


def safe_zip_member(member: object, *, file_only: bool = False) -> bool:
    if not isinstance(member, str) or not member or "\\" in member or "\x00" in member:
        return False
    if member.startswith("/") or ":" in member:
        return False
    normalized = member[:-1] if member.endswith("/") else member
    if not normalized:
        return False
    path = PurePosixPath(normalized)
    return (
        path.as_posix() == normalized
        and not path.is_absolute()
        and all(part not in {"", ".", ".."} for part in path.parts)
        and all(
            part == part.rstrip(" .") and WINDOWS_RESERVED_MEMBER.match(part) is None
            for part in path.parts
        )
        and (not file_only or not member.endswith("/"))
    )


def git_show_text(commit: str, relative: str) -> str | None:
    completed = subprocess.run(
        ["git", "-C", str(ROOT), "show", f"{commit}:{relative}"],
        check=False,
        capture_output=True,
    )
    if completed.returncode != 0:
        return None
    try:
        return completed.stdout.decode("utf-8").replace("\r\n", "\n")
    except UnicodeDecodeError:
        return None


def exact_version_only_post_gate_change(final_commit: str, relative: str) -> bool:
    before = git_show_text(final_commit, relative)
    current_path = ROOT / relative
    if before is None or not current_path.is_file():
        return False
    after = current_path.read_text(encoding="utf-8").replace("\r\n", "\n")
    expected = before
    for old, new in POST_GATE_VERSION_SUBSTITUTIONS:
        expected = expected.replace(old, new)
    for old, new in POST_GATE_VERSION_PATH_SUBSTITUTIONS.get(relative, ()):
        expected = expected.replace(old, new)
    return expected != before and after == expected


def safe_post_gate_evidence_path(relative: str) -> bool:
    return (
        relative.startswith("validation/results/v255_")
        and Path(relative).suffix.lower() in POST_GATE_EVIDENCE_SUFFIXES
    )


def exact_simple_disk_snapshots(payload: object, labels: list[str]) -> bool:
    return (
        isinstance(payload, list)
        and len(payload) == len(labels)
        and [row.get("label") for row in payload if isinstance(row, dict)] == labels
        and all(
            isinstance(row, dict)
            and isinstance(row.get("captured_at"), str)
            and isinstance(row.get("drives"), dict)
            and set(row["drives"]) == {"C", "D"}
            and all(
                isinstance(row["drives"].get(drive), (int, float))
                and row["drives"][drive] > 20.0
                for drive in ("C", "D")
            )
            for row in payload
        )
    )


def json_integer(value: object) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def utc_timestamp(value: object) -> datetime | None:
    if not isinstance(value, str) or not value:
        return None
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return None
    if parsed.tzinfo is None:
        return None
    return parsed.astimezone(timezone.utc)


def exact_drive_bytes(value: object) -> dict[str, int] | None:
    if (
        not isinstance(value, dict)
        or set(value) != {"C", "D"}
        or any(
            not json_integer(value.get(drive)) or value[drive] < 0
            for drive in ("C", "D")
        )
    ):
        return None
    return {"C": int(value["C"]), "D": int(value["D"])}


def exact_build_disk_watcher(
    value: object,
    *,
    started_at: object,
    completed_at: object,
) -> bool:
    started = utc_timestamp(started_at)
    completed = utc_timestamp(completed_at)
    if started is None or completed is None or completed <= started:
        return False
    if not isinstance(value, dict) or set(value) != {
        "policy",
        "preflight",
        "samples",
        "breach_detected",
        "exact_pid_tree_only",
    }:
        return False
    if value.get("breach_detected") is not False or value.get("exact_pid_tree_only") is not True:
        return False
    if value.get("policy") != {
        "minimum_free_gib_exclusive": BUILD_DISK_FLOOR_GIB,
        "minimum_free_bytes_exclusive": BUILD_DISK_FLOOR_BYTES,
        "preflight_reserve_gib": BUILD_PREFLIGHT_RESERVE_GIB,
        "preflight_required_free_gib_exclusive": BUILD_PREFLIGHT_REQUIRED_GIB,
        "preflight_required_free_bytes_exclusive": BUILD_PREFLIGHT_REQUIRED_BYTES,
        "poll_interval_ms": BUILD_DISK_POLL_INTERVAL_MS,
        "breach_action": BUILD_DISK_BREACH_ACTION,
    }:
        return False

    preflight = value.get("preflight")
    if not isinstance(preflight, dict) or set(preflight) != {
        "captured_at",
        "observed_free_bytes",
        "required_free_bytes_exclusive",
        "required_free_gib_exclusive",
        "passed",
    }:
        return False
    preflight_at = utc_timestamp(preflight.get("captured_at"))
    observed = exact_drive_bytes(preflight.get("observed_free_bytes"))
    if (
        preflight.get("passed") is not True
        or preflight.get("required_free_bytes_exclusive") != BUILD_PREFLIGHT_REQUIRED_BYTES
        or preflight.get("required_free_gib_exclusive") != BUILD_PREFLIGHT_REQUIRED_GIB
        or preflight_at is None
        or preflight_at > started
        or observed is None
        or any(
            observed[drive] <= BUILD_PREFLIGHT_REQUIRED_BYTES[drive]
            for drive in ("C", "D")
        )
    ):
        return False

    command_ids = ("tauri_desktop_bundle", "locked_release_cli")
    samples = value.get("samples")
    if not isinstance(samples, list) or not samples:
        return False
    states: dict[str, list[str]] = {command_id: [] for command_id in command_ids}
    root_pids: dict[str, int] = {}
    previous_time = started
    previous_command_index = 0
    for sample in samples:
        if not isinstance(sample, dict) or set(sample) != {
            "captured_at",
            "command_id",
            "root_pid",
            "process_tree_pids",
            "state",
            "free_bytes",
            "floor_breached",
        }:
            return False
        command_id = sample.get("command_id")
        if not isinstance(command_id, str) or command_id not in states:
            return False
        command_index = command_ids.index(command_id)
        if command_index < previous_command_index:
            return False
        previous_command_index = command_index
        state = sample.get("state")
        if not isinstance(state, str) or state not in {"running", "completed"}:
            return False
        root_pid = sample.get("root_pid")
        if not json_integer(root_pid) or root_pid <= 0:
            return False
        if command_id in root_pids and root_pids[command_id] != root_pid:
            return False
        root_pids[command_id] = root_pid
        process_tree = sample.get("process_tree_pids")
        if (
            not isinstance(process_tree, list)
            or not process_tree
            or any(not json_integer(pid) or pid <= 0 for pid in process_tree)
            or process_tree != sorted(set(process_tree))
            or root_pid not in process_tree
        ):
            return False
        free_bytes = exact_drive_bytes(sample.get("free_bytes"))
        captured_at = utc_timestamp(sample.get("captured_at"))
        if (
            sample.get("floor_breached") is not False
            or free_bytes is None
            or any(
                free_bytes[drive] <= BUILD_DISK_FLOOR_BYTES for drive in ("C", "D")
            )
            or captured_at is None
            or captured_at < previous_time
            or captured_at > completed
        ):
            return False
        previous_time = captured_at
        states[command_id].append(state)
    return all(
        values
        and values[-1] == "completed"
        and values.count("completed") == 1
        and "running" in values
        for values in states.values()
    )


def release_version_authority_checks(expected_version: str) -> dict[str, bool]:
    checks: dict[str, bool] = {}
    try:
        package = read_json("package.json")
        package_lock = read_json("package-lock.json")
        tauri = read_json("src-tauri/tauri.conf.json")
        release_channels = read_json("validation/quickpls_release_channels.json")
        cargo = load_toml("Cargo.toml")
        cargo_lock = load_toml("Cargo.lock")
    except (OSError, ValueError, json.JSONDecodeError, tomllib.TOMLDecodeError):
        return {"all_product_version_authorities_are_parseable": False}

    checks["all_product_version_authorities_are_parseable"] = all(
        isinstance(value, dict)
        for value in (package, package_lock, tauri, release_channels, cargo, cargo_lock)
    )
    package_lock_root = (
        package_lock.get("packages", {}) if isinstance(package_lock, dict) else {}
    )
    checks["npm_and_tauri_version_authorities_match"] = (
        package.get("version") == expected_version
        and isinstance(package_lock_root, dict)
        and package_lock.get("version") == expected_version
        and isinstance(package_lock_root.get(""), dict)
        and package_lock_root[""].get("version") == expected_version
        and tauri.get("version") == expected_version
    )

    workspace = cargo.get("workspace", {}) if isinstance(cargo, dict) else {}
    workspace_package = (
        workspace.get("package", {}) if isinstance(workspace, dict) else {}
    )
    members = workspace.get("members", []) if isinstance(workspace, dict) else []
    member_names: list[str] = []
    member_manifests_valid = isinstance(members, list) and bool(members)
    if member_manifests_valid:
        for member in members:
            if not isinstance(member, str):
                member_manifests_valid = False
                continue
            try:
                manifest = load_toml(f"{member}/Cargo.toml")
            except (OSError, ValueError, tomllib.TOMLDecodeError):
                member_manifests_valid = False
                continue
            package_table = manifest.get("package", {})
            member_version = (
                package_table.get("version")
                if isinstance(package_table, dict)
                else None
            )
            if (
                not isinstance(package_table, dict)
                or not isinstance(package_table.get("name"), str)
                or not isinstance(member_version, dict)
                or member_version.get("workspace") is not True
            ):
                # tomllib represents `version.workspace = true` as the nested
                # package.version.workspace table.
                member_manifests_valid = False
            if isinstance(package_table, dict) and isinstance(
                package_table.get("name"), str
            ):
                member_names.append(str(package_table["name"]))

    lock_packages = (
        cargo_lock.get("package", []) if isinstance(cargo_lock, dict) else []
    )
    lock_versions: dict[str, list[object]] = {}
    if isinstance(lock_packages, list):
        for row in lock_packages:
            if isinstance(row, dict) and isinstance(row.get("name"), str):
                lock_versions.setdefault(str(row["name"]), []).append(
                    row.get("version")
                )
    checks["cargo_workspace_and_lock_version_authorities_match"] = (
        isinstance(workspace_package, dict)
        and workspace_package.get("version") == expected_version
        and member_manifests_valid
        and len(member_names) == len(set(member_names)) == len(members)
        and all(lock_versions.get(name) == [expected_version] for name in member_names)
    )

    artifact_label = (
        "v2_55_0_calculate_evidence"
        if expected_version == "2.55.0"
        else "v2_54_0_canvas_results"
    )
    package_scripts = package.get("scripts", {}) if isinstance(package, dict) else {}
    release_test = read_text("validation/test_package_release_artifacts.py")
    native_desktop = read_text("src/native/NativeDesktopApp.tsx")
    prototype = read_text("src/v2/NativePrototypeApp.tsx")
    readme = read_text("README.md")
    installation = read_text("docs/INSTALLATION.md")
    changelog = read_text("CHANGELOG.md")
    checks["release_channel_and_packaging_authorities_match"] = (
        release_channels.get("product_version") == expected_version
        and isinstance(package_scripts, dict)
        and package_scripts.get("qpls:release:artifacts")
        == (
            "powershell -NoProfile -ExecutionPolicy Bypass -File "
            f"validation/run_v255_unsigned_candidate_build.ps1 -Label {artifact_label}"
        )
        and re.search(
            rf'^REPOSITORY_RELEASE_VERSION = "{re.escape(expected_version)}"$',
            release_test,
            re.MULTILINE,
        )
        is not None
        and re.search(
            rf'^REPOSITORY_ARTIFACT_LABEL = "{re.escape(artifact_label)}"$',
            release_test,
            re.MULTILINE,
        )
        is not None
    )
    checks["visible_application_and_documentation_versions_match"] = (
        native_desktop.count(f"<dt>Version</dt><dd>{expected_version}</dd>") == 1
        and prototype.count(f'const releaseVersion = "{expected_version}";') == 1
        and prototype.count(f"<h3>QuickPLS {expected_version}</h3>") == 1
        and f"Current source version: **{expected_version}**." in readme
        and f"Current source version: **{expected_version}**." in installation
        and f"## [{expected_version}]" in changelog
    )
    return checks


def parse_json_path(path: Path) -> dict[str, object] | None:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    return payload if isinstance(payload, dict) else None


def report_bound_file(report_path: Path, declared: object) -> Path | None:
    if not isinstance(declared, str) or not declared:
        return None
    candidate = Path(declared)
    if not candidate.is_absolute():
        candidate = report_path.parent / candidate
    try:
        resolved = candidate.resolve(strict=True)
        resolved.relative_to(report_path.parent.resolve())
    except (OSError, ValueError):
        return None
    return resolved if resolved.is_file() else None


def repository_bound_file(declared: object) -> Path | None:
    if not isinstance(declared, str) or not declared:
        return None
    candidate = Path(declared)
    if not candidate.is_absolute():
        candidate = ROOT / candidate
    try:
        resolved = candidate.resolve(strict=True)
        resolved.relative_to(ROOT.resolve())
    except (OSError, ValueError):
        return None
    return resolved if resolved.is_file() else None


def resolved_declared_path(declared: object) -> Path | None:
    if not isinstance(declared, str) or not declared:
        return None
    try:
        return Path(declared).resolve()
    except (OSError, ValueError):
        return None


def canonical_consolidated_step_executable(
    report: dict[str, object], declared: object
) -> object:
    if not isinstance(declared, str):
        return declared
    policy = report.get("policy", {})
    tools = policy.get("tools", {}) if isinstance(policy, dict) else {}
    if not isinstance(tools, dict):
        return declared
    for key, requested in CANONICAL_TOOL_INPUTS.items():
        declaration = tools.get(key)
        if (
            not isinstance(declaration, dict)
            or set(declaration) != {"requested", "resolved_path", "sha256"}
            or declaration.get("requested") != requested
            or declaration.get("resolved_path") != declared
            or not is_sha256(declaration.get("sha256"))
        ):
            continue
        try:
            resolved = Path(declared).resolve(strict=True)
        except OSError:
            continue
        if resolved.is_file() and sha256_path(resolved) == declaration.get("sha256"):
            return requested
    return declared


def normalized_step_contract(
    report_path: Path, report: dict[str, object]
) -> list[tuple[object, ...]]:
    root = str(report_path.parent.resolve()).replace("\\", "/")
    result: list[tuple[object, ...]] = []
    for raw in report.get("steps", []):
        if not isinstance(raw, dict):
            continue
        arguments = tuple(
            str(value).replace("\\", "/").replace(root, "<EVIDENCE_DIR>")
            for value in raw.get("arguments", [])
        )
        result.append(
            (
                raw.get("id"),
                raw.get("description"),
                canonical_consolidated_step_executable(
                    report, raw.get("executable")
                ),
                arguments,
            )
        )
    return result


def expected_consolidated_step_contract() -> list[tuple[object, ...]]:
    e = "<EVIDENCE_DIR>"
    return [
        ("diff_check", "Git whitespace-error check", "git", ("diff", "--check")),
        (
            "v255_evidence_contract",
            "Validate the 18-method evidence matrix and interaction-first rebaseline",
            "python",
            (
                "validation/v255_product_completion_audit.py",
                "--output",
                f"{e}/v255_source_contract_audit.json",
            ),
        ),
        (
            "v255_rebased_contract",
            "Validate the 17 replacement interaction contracts",
            "node",
            ("validation/v255_rebased_interaction_contracts.mjs", "--mode", "contract"),
        ),
        (
            "frontend_full_vitest",
            "One full current frontend and domain Vitest traversal with machine-readable assertions",
            "npm.cmd",
            (
                "run",
                "test",
                "--",
                "--reporter=json",
                "--outputFile",
                f"{e}/v255_full_vitest.json",
            ),
        ),
        (
            "rust_authority",
            "Focused exact Registry authority test",
            "cargo",
            (
                "test",
                "--locked",
                "-p",
                "qpls-core",
                "embedded_registry_is_the_exact_option_cell_authority",
                "--",
                "--nocapture",
            ),
        ),
        (
            "rust_archive_schema6_authoring",
            "Focused schema-6 author/save/reopen authority test",
            "cargo",
            (
                "test",
                "--locked",
                "-p",
                "qpls-project",
                "--test",
                "schema6_sem_model_v4_authoring_shapes",
                "section_3_1_shapes_author_serialize_and_reopen_through_standalone_schema6",
                "--",
                "--exact",
                "--nocapture",
            ),
        ),
        (
            "rust_archive_three_way",
            "Focused three-way canonical append/reopen/tamper lifecycle",
            "cargo",
            (
                "test",
                "--locked",
                "-p",
                "qpls-project",
                "project_schema_v6::tests::three_way_canonical_append_reopen_and_tamper_fail_closed",
                "--lib",
                "--",
                "--exact",
                "--nocapture",
            ),
        ),
        (
            "rust_desktop_three_way",
            "Focused desktop three-way execute/build/append/reopen lifecycle",
            "cargo",
            (
                "test",
                "--locked",
                "-p",
                "quickpls-desktop",
                "recipe_v4_general_sem_canonical_result::tests::strict_v3_colon_ids_execute_build_append_and_reopen_three_way_canonical_result",
                "--lib",
                "--",
                "--exact",
                "--nocapture",
            ),
        ),
        (
            "frontend_typecheck",
            "Full frontend typecheck",
            "npm.cmd",
            ("run", "typecheck:full"),
        ),
        (
            "frontend_build",
            "Production frontend bundle",
            "npm.cmd",
            ("run", "build:bundle"),
        ),
        (
            "python_export_semantic_readback",
            "Focused canonical CSV/XLSX/HTML/PDF/SVG/PNG semantic export readback",
            "python",
            ("validation/test_general_sem_rank0_export_semantic_readback.py",),
        ),
        (
            "rebaselined_interactions",
            "1024x700 interaction evidence reconciled with exact Vitest assertions",
            "node",
            (
                "validation/v255_rebased_interaction_contracts.mjs",
                "--mode",
                "browser",
                "--evidence-dir",
                f"{e}/interaction_contracts",
                "--vitest-report",
                f"{e}/v255_full_vitest.json",
                "--port",
                "57655",
            ),
        ),
        (
            "method_setup_crawler",
            "Serial complete Calculate setup and pre-candidate reusable-archive inventory crawl",
            "node",
            (
                "validation/v255_method_evidence_crawler.mjs",
                "--mode",
                "preview",
                "--result-evidence-phase",
                "source",
                "--evidence-dir",
                f"{e}/method_evidence",
                "--vitest-report",
                f"{e}/v255_full_vitest.json",
                "--port",
                "57656",
            ),
        ),
        (
            "v255_final_evidence_contract",
            "Require all 17 rebaselined contracts and hash their reports",
            "python",
            (
                "validation/v255_product_completion_audit.py",
                "--output",
                f"{e}/v255_final_contract_audit.json",
                "--final-stage",
                "--vitest-report",
                f"{e}/v255_full_vitest.json",
                "--rebaseline-report",
                f"{e}/interaction_contracts/v255_rebased_interaction_contracts.json",
            ),
        ),
    ]


def consolidated_summary_matches(payload: dict[str, object]) -> bool:
    steps = payload.get("steps", [])
    summary = payload.get("summary", {})
    snapshots = payload.get("disk_snapshots", [])
    if (
        not isinstance(steps, list)
        or not isinstance(summary, dict)
        or not isinstance(snapshots, list)
    ):
        return False
    failed = [
        step.get("id")
        for step in steps
        if isinstance(step, dict) and step.get("status") == "failed"
    ]
    skipped = [
        step.get("id")
        for step in steps
        if isinstance(step, dict) and step.get("status") == "skipped"
    ]
    valid_records = (
        len(steps) == len(EXPECTED_CONSOLIDATED_STEPS)
        and all(
            isinstance(step, dict)
            and step.get("status") in {"passed", "failed", "skipped"}
            for step in steps
        )
        and len({step.get("id") for step in steps if isinstance(step, dict)})
        == len(steps)
    )
    snapshots_passed = bool(snapshots) and all(
        isinstance(snapshot, dict) and snapshot.get("passed") is True
        for snapshot in snapshots
    )
    return (
        valid_records
        and summary.get("total") == len(steps)
        and summary.get("failed") == failed
        and summary.get("skipped") == skipped
        and payload.get("passed") is (not failed and not skipped and snapshots_passed)
    )


def consolidated_policy_and_disk_contract(
    report_path: Path, payload: dict[str, object]
) -> bool:
    policy = payload.get("policy", {})
    steps = payload.get("steps", [])
    snapshots = payload.get("disk_snapshots", [])
    if (
        not isinstance(policy, dict)
        or not isinstance(steps, list)
        or not isinstance(snapshots, list)
    ):
        return False
    tools = policy.get("tools", {})
    if not isinstance(tools, dict) or set(tools) != set(CANONICAL_TOOL_INPUTS):
        return False
    for key, requested in CANONICAL_TOOL_INPUTS.items():
        declaration = tools.get(key)
        if (
            not isinstance(declaration, dict)
            or set(declaration) != {"requested", "resolved_path", "sha256"}
            or declaration.get("requested") != requested
        ):
            return False
        resolved = declaration.get("resolved_path")
        if not isinstance(resolved, str) or not is_sha256(declaration.get("sha256")):
            return False
        try:
            resolved_path = Path(resolved).resolve(strict=True)
        except OSError:
            return False
        if not resolved_path.is_file() or sha256_path(resolved_path) != declaration.get(
            "sha256"
        ):
            return False

    target_dir = policy.get("cargo_target_dir")
    try:
        target_dir_matches = (
            isinstance(target_dir, str)
            and Path(target_dir).resolve() == (ROOT / "target").resolve()
        )
    except OSError:
        target_dir_matches = False
    if not (
        set(policy)
        == {
            "serial",
            "maximum_concurrent_calculations",
            "code_signing",
            "repeated_scientific_qualification_matrices",
            "batch_fix_then_identical_rerun",
            "canonical_parameters_locked",
            "minimum_free_gib_exclusive",
            "emergency_free_gib_exclusive",
            "disk_watch_interval_milliseconds",
            "default_disk_step_headroom_gib",
            "cargo_incremental",
            "cargo_target_dir",
            "target_drive_reused",
            "tools",
            "step_reserves_gib",
        }
        and policy.get("serial") is True
        and policy.get("maximum_concurrent_calculations") == 1
        and policy.get("code_signing") is False
        and policy.get("repeated_scientific_qualification_matrices") is False
        and policy.get("batch_fix_then_identical_rerun") is True
        and policy.get("canonical_parameters_locked") is True
        and policy.get("minimum_free_gib_exclusive") == 20.0
        and policy.get("emergency_free_gib_exclusive") == 20.25
        and policy.get("disk_watch_interval_milliseconds") == 500
        and policy.get("default_disk_step_headroom_gib") == 0.5
        and policy.get("cargo_incremental") == 0
        and target_dir_matches
        and policy.get("target_drive_reused") == "D: workspace target"
        and policy.get("step_reserves_gib") == CONSOLIDATED_DISK_RESERVES
    ):
        return False

    expected_labels = ["before_consolidated_pass"]
    for step_id in EXPECTED_CONSOLIDATED_STEPS:
        if step_id in CONSOLIDATED_DISK_RESERVES:
            expected_labels.extend((f"before_{step_id}", f"after_{step_id}"))
    expected_labels.append("after_consolidated_pass")
    if [
        snapshot.get("label") for snapshot in snapshots if isinstance(snapshot, dict)
    ] != expected_labels:
        return False
    snapshots_by_label = {
        str(snapshot.get("label")): snapshot
        for snapshot in snapshots
        if isinstance(snapshot, dict)
    }
    for snapshot in snapshots:
        if (
            not isinstance(snapshot, dict)
            or set(snapshot) != {"label", "captured_at", "passed", "drives"}
            or snapshot.get("passed") is not True
            or not isinstance(snapshot.get("captured_at"), str)
        ):
            return False
        drives = snapshot.get("drives", [])
        drive_rows = (
            {str(row.get("name")): row for row in drives if isinstance(row, dict)}
            if isinstance(drives, list)
            else {}
        )
        if len(drives) != len(drive_rows) or set(drive_rows) != {"C", "D"}:
            return False
        label = str(snapshot.get("label"))
        reserve = 0.0
        if label.startswith("before_"):
            reserve = CONSOLIDATED_DISK_RESERVES.get(label.removeprefix("before_"), 0.0)
        for row in drive_rows.values():
            free = row.get("free_gib")
            if not (
                set(row)
                == {
                    "name",
                    "free_gib",
                    "floor_gib_exclusive",
                    "required_free_gib_exclusive",
                    "reserved_headroom_gib",
                    "passed",
                }
                and isinstance(free, (int, float))
                and row.get("floor_gib_exclusive") == 20.0
                and row.get("reserved_headroom_gib") == reserve
                and row.get("required_free_gib_exclusive") == 20.0 + reserve
                and row.get("passed") is True
                and free > 20.0 + reserve
            ):
                return False

    step_by_id = {str(step.get("id")): step for step in steps if isinstance(step, dict)}
    for step_id in EXPECTED_CONSOLIDATED_STEPS:
        step = step_by_id.get(step_id)
        if not isinstance(step, dict):
            return False
        watcher = step.get("disk_watcher")
        if step_id not in CONSOLIDATED_DISK_RESERVES:
            if watcher is not None or "disk_gate" in step:
                return False
            continue
        disk_gate = step.get("disk_gate")
        if not isinstance(disk_gate, dict) or not isinstance(watcher, dict):
            return False
        if (
            set(disk_gate) != {"before", "after"}
            or set(watcher)
            != {
                "launched_pid",
                "process_tree_termination_is_pid_scoped",
                "emergency_free_gib_exclusive",
                "poll_interval_milliseconds",
                "stopped_for_low_disk",
                "samples",
                "minimum_free_gib",
            }
            or disk_gate.get("before") != snapshots_by_label.get(f"before_{step_id}")
            or disk_gate.get("after") != snapshots_by_label.get(f"after_{step_id}")
            or not isinstance(watcher.get("launched_pid"), int)
            or watcher.get("launched_pid", 0) <= 0
            or watcher.get("process_tree_termination_is_pid_scoped") is not True
            or watcher.get("emergency_free_gib_exclusive") != 20.25
            or watcher.get("poll_interval_milliseconds") != 500
            or watcher.get("stopped_for_low_disk") is not False
            or not isinstance(watcher.get("samples"), int)
            or watcher.get("samples", -1) < 0
        ):
            return False
        minimums = watcher.get("minimum_free_gib", {})
        if not isinstance(minimums, dict) or set(minimums) != {"C", "D"}:
            return False
        if watcher.get("samples", 0) > 0 and any(
            not isinstance(minimums.get(drive), (int, float))
            or minimums[drive] <= 20.25
            for drive in ("C", "D")
        ):
            return False
    return True


def consolidated_diagnostic_checks(
    first_path: Path | None,
    final_path: Path | None,
    final_vitest_path: Path | None,
    final_rebaseline_path: Path | None,
) -> tuple[
    dict[str, bool],
    dict[str, object],
    dict[str, object] | None,
    dict[str, object] | None,
]:
    checks = {
        "publication_first_consolidated_report_is_supplied": first_path is not None
        and first_path.is_file(),
        "publication_final_consolidated_report_is_supplied": final_path is not None
        and final_path.is_file(),
        "publication_consolidated_reports_are_distinct": first_path is not None
        and final_path is not None
        and first_path.resolve() != final_path.resolve(),
    }
    evidence: dict[str, object] = {}
    if not all(checks.values()):
        return checks, evidence, None, None
    assert first_path is not None and final_path is not None
    first = parse_json_path(first_path)
    final = parse_json_path(final_path)
    checks["publication_consolidated_reports_are_parseable"] = (
        first is not None and final is not None
    )
    evidence.update(
        {
            "first_consolidated_report": str(first_path),
            "first_consolidated_report_sha256": sha256_path(first_path),
            "final_consolidated_report": str(final_path),
            "final_consolidated_report_sha256": sha256_path(final_path),
        }
    )
    if first is None or final is None:
        return checks, evidence, first, final

    def exact_suite(payload: dict[str, object]) -> bool:
        steps = payload.get("steps", [])
        summary = payload.get("summary", {})
        source = payload.get("source", {})
        return (
            payload.get("schema_version") == 1
            and payload.get("suite_id")
            == "quickpls_v255_calculate_evidence_consolidated_diagnostics_v1"
            and payload.get("target_release") == "2.55.0"
            and isinstance(steps, list)
            and all(isinstance(step, dict) for step in steps)
            and [step.get("id") for step in steps] == EXPECTED_CONSOLIDATED_STEPS
            and len(steps) == len(EXPECTED_CONSOLIDATED_STEPS)
            and isinstance(summary, dict)
            and summary.get("total") == len(EXPECTED_CONSOLIDATED_STEPS)
            and isinstance(source, dict)
            and re.fullmatch(r"[0-9a-f]{40}", str(source.get("commit", ""))) is not None
            and source.get("worktree_clean") is True
            and source.get("package_version") == "2.54.0"
            and source.get("gate_script")
            == "validation/run_v255_consolidated_diagnostics.ps1"
            and is_sha256(source.get("gate_script_sha256"))
        )

    def complete_step_records(report_path: Path, payload: dict[str, object]) -> bool:
        for step in payload.get("steps", []):
            if not isinstance(step, dict) or step.get("status") not in {
                "passed",
                "failed",
            }:
                return False
            exit_code = step.get("exit_code")
            if not isinstance(exit_code, int):
                return False
            if step.get("status") == "passed" and (
                exit_code != 0 or step.get("error") is not None
            ):
                return False
            if (
                step.get("status") == "failed"
                and exit_code == 0
                and not step.get("error")
            ):
                return False
            for field in ("stdout", "stderr"):
                artifact = report_bound_file(report_path, step.get(field))
                if artifact is None or sha256_path(artifact) != step.get(
                    f"{field}_sha256"
                ):
                    return False
        return True

    checks["publication_first_and_final_use_exact_consolidated_suite"] = exact_suite(
        first
    ) and exact_suite(final)
    checks["publication_first_consolidated_pass_executed_every_step"] = (
        complete_step_records(first_path, first)
    )
    checks["publication_first_consolidated_step_summary_is_exact"] = (
        consolidated_summary_matches(first)
    )
    final_summary = (
        final.get("summary", {}) if isinstance(final.get("summary"), dict) else {}
    )
    final_source = (
        final.get("source", {}) if isinstance(final.get("source"), dict) else {}
    )
    first_summary = (
        first.get("summary", {}) if isinstance(first.get("summary"), dict) else {}
    )
    first_source = (
        first.get("source", {}) if isinstance(first.get("source"), dict) else {}
    )
    checks["publication_final_consolidated_pass_is_fully_green"] = (
        final.get("passed") is True
        and complete_step_records(final_path, final)
        and consolidated_summary_matches(final)
        and final_summary.get("failed") == []
        and final_summary.get("skipped") == []
        and all(
            isinstance(step, dict) and step.get("status") == "passed"
            for step in final.get("steps", [])
        )
    )
    expected_contract = expected_consolidated_step_contract()
    checks["publication_first_and_final_use_exact_tool_invocation_contract"] = (
        normalized_step_contract(first_path, first) == expected_contract
        and normalized_step_contract(final_path, final) == expected_contract
    )
    checks["publication_first_and_final_use_canonical_disk_and_tool_policy"] = (
        consolidated_policy_and_disk_contract(first_path, first)
        and consolidated_policy_and_disk_contract(final_path, final)
    )
    checks["publication_consolidated_rerun_used_identical_gate_and_step_contract"] = (
        first_source.get("gate_script_sha256") == final_source.get("gate_script_sha256")
        and final_source.get("gate_script_sha256")
        == sha256("validation/run_v255_consolidated_diagnostics.ps1")
        and normalized_step_contract(first_path, first)
        == normalized_step_contract(final_path, final)
    )
    checks["publication_failure_batch_semantics_are_consistent"] = first.get(
        "passed"
    ) is True or (
        first.get("passed") is False
        and bool(first_summary.get("failed"))
        and first_source.get("commit") != final_source.get("commit")
    )

    first_artifacts = first.get("artifacts", {})
    first_rebaseline = report_bound_file(
        first_path,
        first_artifacts.get("rebaseline_report")
        if isinstance(first_artifacts, dict)
        else None,
    )
    first_vitest_artifact = report_bound_file(
        first_path,
        first_artifacts.get("vitest_report")
        if isinstance(first_artifacts, dict)
        else None,
    )
    first_rebaseline_payload = (
        parse_json_path(first_rebaseline) if first_rebaseline is not None else None
    )
    checks["publication_first_consolidated_rebaseline_artifact_is_hash_bound"] = (
        first_rebaseline is not None
        and first_vitest_artifact is not None
        and sha256_path(first_rebaseline)
        == first_artifacts.get("rebaseline_report_sha256")
        and sha256_path(first_vitest_artifact)
        == first_artifacts.get("vitest_report_sha256")
        and first_rebaseline_payload is not None
        and first_rebaseline_payload.get("schema_version") == 3
        and first_rebaseline_payload.get("suite_id")
        == "quickpls_v255_rebased_interaction_contracts_v3"
        and first_rebaseline_payload.get("vitest_report_sha256")
        == first_artifacts.get("vitest_report_sha256")
    )
    final_artifacts = final.get("artifacts", {})
    final_vitest = report_bound_file(
        final_path,
        final_artifacts.get("vitest_report")
        if isinstance(final_artifacts, dict)
        else None,
    )
    checks["publication_final_consolidated_vitest_is_hash_bound"] = (
        final_vitest is not None
        and final_vitest_path is not None
        and final_vitest_path.is_file()
        and final_vitest.resolve() == final_vitest_path.resolve()
        and sha256_path(final_vitest) == final_artifacts.get("vitest_report_sha256")
    )
    final_rebaseline = report_bound_file(
        final_path,
        final_artifacts.get("rebaseline_report")
        if isinstance(final_artifacts, dict)
        else None,
    )
    checks["publication_final_consolidated_rebaseline_is_exactly_hash_bound"] = (
        final_rebaseline is not None
        and final_rebaseline_path is not None
        and final_rebaseline_path.is_file()
        and final_rebaseline.resolve() == final_rebaseline_path.resolve()
        and sha256_path(final_rebaseline)
        == final_artifacts.get("rebaseline_report_sha256")
    )
    return checks, evidence, first, final


def git_output(*arguments: str) -> tuple[int, str]:
    completed = subprocess.run(
        ["git", "-C", str(ROOT), *arguments],
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    return completed.returncode, completed.stdout.strip()


def publication_source_lineage_checks(
    first: dict[str, object] | None,
    final: dict[str, object] | None,
) -> tuple[dict[str, bool], dict[str, object]]:
    checks: dict[str, bool] = {}
    evidence: dict[str, object] = {}
    first_commit = (
        first.get("source", {}).get("commit") if isinstance(first, dict) else None
    )
    final_commit = (
        final.get("source", {}).get("commit") if isinstance(final, dict) else None
    )
    head_exit, current_commit = git_output("rev-parse", "HEAD")
    status_exit, status = git_output(
        "status", "--porcelain=v1", "--untracked-files=all"
    )
    checks["publication_current_source_is_clean_committed_2_55"] = (
        head_exit == 0
        and status_exit == 0
        and re.fullmatch(r"[0-9a-f]{40}", current_commit) is not None
        and status == ""
    )
    evidence["publication_source_commit"] = current_commit if head_exit == 0 else None
    if (
        not isinstance(first_commit, str)
        or not isinstance(final_commit, str)
        or head_exit != 0
    ):
        checks["publication_source_descends_from_both_consolidated_passes"] = False
        checks["publication_post_gate_changes_are_release_only"] = False
        checks["publication_version_authority_changes_are_content_exact"] = False
        return checks, evidence
    first_ancestor = (
        subprocess.run(
            [
                "git",
                "-C",
                str(ROOT),
                "merge-base",
                "--is-ancestor",
                first_commit,
                final_commit,
            ],
            check=False,
        ).returncode
        == 0
    )
    final_ancestor = (
        subprocess.run(
            [
                "git",
                "-C",
                str(ROOT),
                "merge-base",
                "--is-ancestor",
                final_commit,
                current_commit,
            ],
            check=False,
        ).returncode
        == 0
    )
    checks["publication_source_descends_from_both_consolidated_passes"] = (
        first_ancestor and final_ancestor
    )
    diff_exit, changed_text = git_output(
        "diff", "--no-renames", "--name-status", f"{final_commit}..{current_commit}"
    )
    changed_rows: list[tuple[str, str]] = []
    for line in changed_text.splitlines():
        if not line:
            continue
        parts = line.split("\t", 1)
        if len(parts) != 2:
            changed_rows.append(("invalid", line.replace("\\", "/")))
        else:
            changed_rows.append((parts[0], parts[1].replace("\\", "/")))
    changed = [path for _, path in changed_rows]
    release_only = all(
        status in {"A", "M"}
        and (
            path in POST_GATE_RELEASE_CONTENT_EXACT
            or path in POST_GATE_VERSION_ONLY_EXACT
            or path.startswith("docs/")
            or safe_post_gate_evidence_path(path)
        )
        for status, path in changed_rows
    )
    exact_version_changes = all(
        status == "M" and exact_version_only_post_gate_change(final_commit, path)
        for status, path in changed_rows
        if path in POST_GATE_VERSION_ONLY_EXACT
    )
    checks["publication_post_gate_changes_are_release_only"] = (
        diff_exit == 0 and release_only
    )
    checks["publication_version_authority_changes_are_content_exact"] = (
        diff_exit == 0
        and exact_version_changes
        and POST_GATE_VERSION_ONLY_EXACT.issubset(set(changed))
    )
    evidence["post_gate_changed_files"] = changed
    evidence["post_gate_changed_file_statuses"] = [
        {"status": status, "path": path} for status, path in changed_rows
    ]
    return checks, evidence


def final_rebaseline_checks(
    rebaseline: dict[str, object],
    vitest_report_path: Path | None,
    rebaseline_report_path: Path | None,
) -> tuple[dict[str, bool], dict[str, object]]:
    checks: dict[str, bool] = {
        "final_vitest_report_is_supplied": vitest_report_path is not None
        and vitest_report_path.is_file(),
        "final_rebaseline_report_is_supplied": rebaseline_report_path is not None
        and rebaseline_report_path.is_file(),
    }
    evidence: dict[str, object] = {}
    if not all(checks.values()):
        return checks, evidence

    assert vitest_report_path is not None
    assert rebaseline_report_path is not None
    try:
        vitest_payload = json.loads(vitest_report_path.read_text(encoding="utf-8"))
        rebaseline_payload = json.loads(
            rebaseline_report_path.read_text(encoding="utf-8")
        )
    except (OSError, json.JSONDecodeError):
        checks["final_reports_are_parseable_json"] = False
        return checks, evidence

    checks["final_reports_are_parseable_json"] = isinstance(
        vitest_payload, dict
    ) and isinstance(rebaseline_payload, dict)
    evidence["vitest_report"] = str(vitest_report_path)
    evidence["vitest_report_sha256"] = sha256_path(vitest_report_path)
    evidence["rebaseline_report"] = str(rebaseline_report_path)
    evidence["rebaseline_report_sha256"] = sha256_path(rebaseline_report_path)
    if not checks["final_reports_are_parseable_json"]:
        return checks, evidence

    items = rebaseline.get("items", [])
    cases = rebaseline_payload.get("cases", [])
    case_by_id = {
        str(case.get("id", "")): case for case in cases if isinstance(case, dict)
    }
    expected_ids = {str(item.get("id", "")) for item in items if isinstance(item, dict)}
    checks["final_rebaseline_report_declares_exact_passing_suite"] = (
        rebaseline_payload.get("schema_version") == 3
        and rebaseline_payload.get("suite_id")
        == "quickpls_v255_rebased_interaction_contracts_v3"
        and rebaseline_payload.get("passed") is True
        and rebaseline_payload.get("failures") == []
        and rebaseline_payload.get("console_errors") == []
    )
    checks["all_17_rebaseline_cases_are_recorded_once"] = (
        len(cases) == 17 and set(case_by_id) == expected_ids and len(case_by_id) == 17
    )
    checks["all_17_rebaseline_cases_passed"] = checks[
        "all_17_rebaseline_cases_are_recorded_once"
    ] and all(
        case_by_id[str(item.get("id"))].get("status") == "passed"
        for item in items
        if isinstance(item, dict)
    )

    test_types = {
        "component",
        "domain",
        "service",
        "governance",
        "public_command_interaction",
        "component_domain_integration",
        "service_component_integration",
        "public_service_interaction",
        "store_authority_interaction",
        "public_domain_contract",
        "store_archive_activation",
        "public_request_readiness_integration",
        "public_routing_contract",
        "component_render_contract",
        "public_projection_contract",
        "component_registry_integration",
        "public_async_service_interaction",
        "native_service_boundary",
        "static_registry_governance",
    }
    test_backed = []
    browser_backed = []
    for item in items:
        if not isinstance(item, dict):
            continue
        case = case_by_id.get(str(item.get("id", "")), {})
        evidence_type = item.get("evidence_type")
        if evidence_type in test_types:
            identity = (
                case.get("assertion_identity") if isinstance(case, dict) else None
            )
            test_backed.append(
                case.get("replacement_file") == item.get("replacement_file")
                and case.get("replacement_test") == item.get("replacement_test")
                and isinstance(identity, dict)
                and str(identity.get("file", ""))
                .replace("\\", "/")
                .endswith(str(item.get("replacement_file", "")).lstrip("./"))
                and identity.get("title") == item.get("replacement_test")
                and identity.get("status") == "passed"
                and case.get("executed") is True
            )
        elif evidence_type == "browser":
            browser_backed.append(
                case.get("executed") is True
                and case.get("status") == "passed"
                and isinstance(case.get("screenshot"), str)
                and bool(case.get("screenshot"))
            )
        else:
            test_backed.append(False)
    checks["test_backed_cases_bind_exact_passing_assertion_identity"] = all(test_backed)
    checks["browser_backed_cases_are_executed_and_screenshot_bound"] = all(
        browser_backed
    )
    checks["rebaseline_report_hash_matches_declared_vitest_input"] = (
        rebaseline_payload.get("vitest_report_sha256")
        == evidence["vitest_report_sha256"]
    )
    return checks, evidence


def closed_evidence_zip_checks(
    bundle_path: Path,
    frozen_archive_index: dict[str, object],
    named_evidence_index: dict[str, object],
    bundle_manifest: dict[str, object],
) -> tuple[dict[str, bool], dict[str, object]]:
    checks: dict[str, bool] = {}
    evidence: dict[str, object] = {}
    declarations: dict[str, dict[str, object]] = {}
    declaration_conflict = False

    def add_declared(payload: object, family: str, minimum: int, maximum: int) -> None:
        nonlocal declaration_conflict
        if not isinstance(payload, dict):
            declaration_conflict = True
            return
        member = payload.get("member")
        digest = payload.get("sha256")
        if not safe_zip_member(member, file_only=True) or not is_sha256(digest):
            declaration_conflict = True
            return
        assert isinstance(member, str)
        declaration = {
            "family": family,
            "sha256": digest,
            "minimum": minimum,
            "maximum": maximum,
        }
        prior = declarations.get(member)
        if prior is not None and prior != declaration:
            declaration_conflict = True
            return
        declarations[member] = declaration

    frozen_methods = frozen_archive_index.get("methods", [])
    if not isinstance(frozen_methods, list):
        declaration_conflict = True
    else:
        for method in frozen_methods:
            rows = method.get("evidence", []) if isinstance(method, dict) else None
            if not isinstance(rows, list) or not rows:
                declaration_conflict = True
                continue
            for row in rows:
                if not isinstance(row, dict):
                    declaration_conflict = True
                    continue
                add_declared(row.get("archive"), "archive", 1, MAX_ARCHIVE_MEMBER_BYTES)
                add_declared(
                    row.get("screenshot"), "screenshot", 8, MAX_SCREENSHOT_MEMBER_BYTES
                )
                add_declared(row.get("receipt"), "receipt", 2, MAX_RECEIPT_MEMBER_BYTES)
    frozen_declaration_members = set(declarations)

    named_entries = named_evidence_index.get("entries", [])
    if not isinstance(named_entries, list):
        declaration_conflict = True
    else:
        for entry in named_entries:
            if not isinstance(entry, dict) or entry.get("status") not in {
                "verified",
                "waived",
            }:
                declaration_conflict = True
                continue
            if entry.get("status") == "waived" and not exact_waived_index_entry(
                entry, require_artifacts=True
            ):
                declaration_conflict = True
                continue
            add_declared(
                entry.get("screenshot"), "screenshot", 8, MAX_SCREENSHOT_MEMBER_BYTES
            )
            add_declared(entry.get("receipt"), "receipt", 2, MAX_RECEIPT_MEMBER_BYTES)

    named_manifest = (
        bundle_manifest.get("named_evidence", {})
        if isinstance(bundle_manifest.get("named_evidence"), dict)
        else {}
    )
    bundle_declaration = (
        bundle_manifest.get("bundle", {})
        if isinstance(bundle_manifest.get("bundle"), dict)
        else {}
    )
    collector_ref = named_manifest.get("collector_report")
    candidate_ref = named_manifest.get("candidate_report")
    observation_schema_ref = named_manifest.get("observation_schema")
    add_declared(collector_ref, "source_report", 2, MAX_RECEIPT_MEMBER_BYTES)
    add_declared(candidate_ref, "source_report", 2, MAX_RECEIPT_MEMBER_BYTES)
    add_declared(observation_schema_ref, "source_report", 2, MAX_RECEIPT_MEMBER_BYTES)
    named_declaration_members = set(declarations) - frozen_declaration_members

    checks["publication_zip_member_declarations_are_complete_safe_and_consistent"] = (
        frozen_archive_index.get("status") == "verified"
        and (
            named_evidence_index.get("status") == "verified"
            or exact_population_status(
                named_entries, named_evidence_index.get("status")
            )
        )
        and not declaration_conflict
        and 1 <= len(declarations) <= MAX_EVIDENCE_FILE_MEMBERS
    )
    evidence["evidence_zip_max_file_members"] = MAX_EVIDENCE_FILE_MEMBERS
    evidence["evidence_zip_max_bytes"] = MAX_EVIDENCE_ZIP_BYTES
    evidence["evidence_zip_max_uncompressed_bytes"] = MAX_EVIDENCE_UNCOMPRESSED_BYTES

    try:
        bundle_size = bundle_path.stat().st_size
        archive = zipfile.ZipFile(bundle_path, "r")
    except (OSError, zipfile.BadZipFile):
        checks["publication_zip_membership_is_closed_against_curated_indices"] = False
        checks["publication_zip_members_are_safe_size_bounded_and_hash_bound"] = False
        return checks, evidence
    evidence["evidence_zip_size_bytes"] = bundle_size

    hashes_match = True
    bounded_and_safe = 0 < bundle_size <= MAX_EVIDENCE_ZIP_BYTES
    with archive:
        infos = archive.infolist()
        names = [info.filename for info in infos]
        file_infos = [info for info in infos if not info.is_dir()]
        info_by_name = {info.filename: info for info in infos}
        collector_payload: dict[str, object] | None = None
        collector_member = (
            collector_ref.get("member") if isinstance(collector_ref, dict) else None
        )
        collector_digest = (
            collector_ref.get("sha256") if isinstance(collector_ref, dict) else None
        )
        if isinstance(collector_member, str) and collector_member in info_by_name:
            collector_info = info_by_name[collector_member]
            if (
                not collector_info.is_dir()
                and 2 <= collector_info.file_size <= MAX_RECEIPT_MEMBER_BYTES
                and (collector_info.flag_bits & 0x1) == 0
            ):
                try:
                    collector_bytes = archive.read(collector_info)
                    parsed_collector = json.loads(collector_bytes.decode("utf-8-sig"))
                    if (
                        isinstance(parsed_collector, dict)
                        and hashlib.sha256(collector_bytes).hexdigest()
                        == collector_digest
                    ):
                        collector_payload = parsed_collector
                except (
                    UnicodeDecodeError,
                    json.JSONDecodeError,
                    RuntimeError,
                    NotImplementedError,
                    KeyError,
                    zipfile.BadZipFile,
                ):
                    collector_payload = None
        raw_collector_members = (
            collector_payload.get("bundle_members", [])
            if isinstance(collector_payload, dict)
            else []
        )
        collector_members = (
            [member for member in raw_collector_members if isinstance(member, str)]
            if isinstance(raw_collector_members, list)
            else []
        )
        collector_members_valid = (
            collector_payload is not None
            and collector_payload.get("schema_version") == 1
            and collector_payload.get("suite_id")
            == "quickpls_v255_named_evidence_collector_v1"
            and collector_payload.get("target_release") == "2.55.0"
            and collector_payload.get("passed") is True
            and len(collector_members) == len(raw_collector_members)
            and len(collector_members) == len(set(collector_members))
            and all(
                safe_zip_member(member, file_only=True) for member in collector_members
            )
            and named_declaration_members.issubset(set(collector_members))
        )
        expected_files = set(collector_members) | set(declarations)
        expected_directories: set[str] = set()
        for member in expected_files:
            parent = PurePosixPath(member).parent
            while parent.as_posix() != ".":
                expected_directories.add(f"{parent.as_posix()}/")
                parent = parent.parent
        directory_names = {info.filename for info in infos if info.is_dir()}
        closed = (
            collector_members_valid
            and 1 <= len(expected_files) <= MAX_EVIDENCE_FILE_MEMBERS
            and len(names) == len({name.casefold() for name in names})
            and {info.filename for info in file_infos} == expected_files
            and directory_names.issubset(expected_directories)
            and all(safe_zip_member(name) for name in names)
            and len(file_infos) <= MAX_EVIDENCE_FILE_MEMBERS
        )
        total_uncompressed = sum(info.file_size for info in file_infos)
        total_compressed = sum(info.compress_size for info in file_infos)
        bounded_and_safe = bounded_and_safe and (
            total_uncompressed <= MAX_EVIDENCE_UNCOMPRESSED_BYTES
            and total_compressed <= MAX_EVIDENCE_ZIP_BYTES
            and (total_compressed > 0 or total_uncompressed == 0)
            and (
                total_uncompressed == 0
                or total_uncompressed <= max(total_compressed, 1) * 100
            )
            and archive.comment == b""
            and bundle_declaration.get("member_count") == len(names)
            and bundle_declaration.get("ordered_member_names_sha256")
            == hashlib.sha256("\n".join(sorted(names)).encode("utf-8")).hexdigest()
            and bundle_declaration.get("compressed_bytes") == total_compressed
            and bundle_declaration.get("uncompressed_bytes") == total_uncompressed
        )
        for info in infos:
            mode = (info.external_attr >> 16) & 0o170000
            if (
                info.flag_bits & 0x1
                or info.compress_type not in {zipfile.ZIP_STORED, zipfile.ZIP_DEFLATED}
                or mode == 0o120000
                or (info.is_dir() and (info.file_size != 0 or info.compress_size != 0))
            ):
                bounded_and_safe = False
        for info in file_infos:
            declaration = declarations.get(info.filename)
            minimum = declaration.get("minimum") if declaration is not None else 2
            maximum = (
                declaration.get("maximum")
                if declaration is not None
                else MAX_RECEIPT_MEMBER_BYTES
            )
            if (
                not isinstance(minimum, int)
                or not isinstance(maximum, int)
                or not minimum <= info.file_size <= maximum
                or info.compress_size <= 0
                or info.file_size > info.compress_size * 100
                or (
                    declaration is None
                    and PurePosixPath(info.filename).suffix.lower() != ".json"
                )
            ):
                bounded_and_safe = False
                continue
            digest = hashlib.sha256()
            actual_size = 0
            try:
                with archive.open(info, "r") as source:
                    while True:
                        chunk = source.read(1024 * 1024)
                        if not chunk:
                            break
                        actual_size += len(chunk)
                        if actual_size > maximum:
                            bounded_and_safe = False
                            break
                        digest.update(chunk)
            except (OSError, RuntimeError, NotImplementedError, zipfile.BadZipFile):
                hashes_match = False
                continue
            if actual_size != info.file_size or (
                declaration is not None
                and digest.hexdigest() != declaration.get("sha256")
            ):
                hashes_match = False
        checks["publication_zip_membership_is_closed_against_curated_indices"] = closed
        checks["publication_zip_members_are_safe_size_bounded_and_hash_bound"] = (
            closed and bounded_and_safe and hashes_match
        )
        evidence["evidence_zip_observed_file_members"] = len(file_infos)
        evidence["evidence_zip_observed_members"] = len(names)
        evidence["evidence_zip_expected_file_members"] = len(expected_files)
        evidence["evidence_zip_total_uncompressed_bytes"] = total_uncompressed
        evidence["evidence_zip_total_compressed_member_bytes"] = total_compressed
    return checks, evidence


def publication_report_checks(
    method_report_path: Path | None,
    installed_report_path: Path | None,
    frozen_reopen_report_path: Path | None,
    evidence_bundle_path: Path | None,
    bundle_manifest: dict[str, object],
    matrix_hash: str,
    index_hash: str,
    inventory_hash: str,
    bundle_manifest_hash: str,
    final_vitest_hash: str | None,
    final_consolidated_hash: str | None,
    final_source_commit: str | None,
    publication_source_commit: str | None,
    frozen_archive_index: dict[str, object],
    named_evidence_index: dict[str, object],
) -> tuple[dict[str, bool], dict[str, object]]:
    paths = {
        "method_publication_report": method_report_path,
        "installed_portable_report": installed_report_path,
        "frozen_archive_reopen_report": frozen_reopen_report_path,
        "evidence_bundle": evidence_bundle_path,
    }
    checks = {
        f"publication_{name}_is_supplied": path is not None and path.is_file()
        for name, path in paths.items()
    }
    evidence: dict[str, object] = {}
    if not all(checks.values()):
        return checks, evidence
    assert all(path is not None for path in paths.values())
    try:
        method_report = json.loads(method_report_path.read_text(encoding="utf-8"))
        installed_report = json.loads(installed_report_path.read_text(encoding="utf-8"))
        frozen_report = json.loads(
            frozen_reopen_report_path.read_text(encoding="utf-8")
        )
    except (OSError, json.JSONDecodeError):
        checks["publication_reports_are_parseable_json"] = False
        return checks, evidence
    for name, path in paths.items():
        assert path is not None
        evidence[f"{name}_sha256"] = sha256_path(path)
        evidence[name] = str(path)
    checks["publication_reports_are_parseable_json"] = all(
        isinstance(value, dict)
        for value in (method_report, installed_report, frozen_report)
    )
    if not checks["publication_reports_are_parseable_json"]:
        return checks, evidence
    method_sources = (
        method_report.get("sources", {})
        if isinstance(method_report.get("sources"), dict)
        else {}
    )
    checks["publication_method_crawler_passed_exact_suite_and_hashes"] = (
        method_report.get("schema_version") == 2
        and method_report.get("suite_id") == "quickpls_v255_method_evidence_crawler_v2"
        and method_report.get("target_release") == "2.55.0"
        and method_report.get("mode") == "packaged"
        and method_report.get("result_evidence_phase") == "publication"
        and method_report.get("serial") is True
        and method_report.get("maximum_concurrent_calculations") == 1
        and method_report.get("passed") is True
        and not method_report.get("failures")
        and method_sources.get("matrix_sha256") == matrix_hash
        and method_sources.get("frozen_archive_index_sha256") == index_hash
        and method_sources.get("reusable_archive_inventory_sha256") == inventory_hash
        and method_sources.get("evidence_bundle_manifest_sha256")
        == bundle_manifest_hash
        and method_sources.get("vitest_report_sha256") == final_vitest_hash
        and method_report.get("offline", {}).get("passed") is True
        and method_report.get("console_errors") == []
        and method_report.get("evidence_bundle", {}).get("passed") is True
    )
    setups = method_report.get("setups", [])
    setup_keys = (
        {
            (entry.get("kind"), entry.get("setup_case"))
            for entry in setups
            if isinstance(entry, dict)
        }
        if isinstance(setups, list)
        else set()
    )
    checks["publication_has_all_64_setup_cases_executed_once"] = (
        isinstance(setups, list)
        and len(setups) == len(setup_keys) == 64
        and all(
            isinstance(entry, dict)
            and entry.get("status") == "passed"
            and entry.get("executed") is True
            for entry in setups
        )
    )
    captures = method_report.get("calculate_captures", [])
    checks["publication_has_one_current_calculate_capture_for_each_public_method"] = (
        isinstance(captures, list)
        and len(captures) == 18
        and {entry.get("kind") for entry in captures if isinstance(entry, dict)}
        == set(catalogue_kinds())
        and all(
            isinstance(entry, dict)
            and entry.get("status") == "passed"
            and entry.get("executed") is True
            and isinstance(entry.get("screenshot"), str)
            for entry in captures
        )
    )
    archive_inventory = method_report.get("archive_inventory", [])
    checks["publication_method_report_has_18_verified_result_archives"] = (
        isinstance(archive_inventory, list)
        and len(archive_inventory) == 18
        and {
            entry.get("kind") for entry in archive_inventory if isinstance(entry, dict)
        }
        == set(catalogue_kinds())
        and all(
            isinstance(entry, dict) and entry.get("passed") is True
            for entry in archive_inventory
        )
    )

    outcomes = installed_report.get("outcomes", [])
    outcome_rows = (
        [entry for entry in outcomes if isinstance(entry, dict)]
        if isinstance(outcomes, list)
        else []
    )
    outcome_by_name = {str(entry.get("name", "")): entry for entry in outcome_rows}
    executable_paths = [
        str(entry.get("executable", "")).lower() for entry in outcome_rows
    ]
    release_report_path = repository_bound_file(
        installed_report.get("release_artifact_report")
    )
    install_receipt_path = repository_bound_file(
        installed_report.get("install_receipt")
    )
    release_report = (
        parse_json_path(release_report_path)
        if release_report_path is not None
        else None
    )
    install_receipt = (
        parse_json_path(install_receipt_path)
        if install_receipt_path is not None
        else None
    )
    release_source = (
        release_report.get("source", {})
        if isinstance(release_report, dict)
        and isinstance(release_report.get("source"), dict)
        else {}
    )
    release_trust = (
        release_report.get("trust", {})
        if isinstance(release_report, dict)
        and isinstance(release_report.get("trust"), dict)
        else {}
    )
    release_build = (
        release_report.get("build", {})
        if isinstance(release_report, dict)
        and isinstance(release_report.get("build"), dict)
        else {}
    )
    release_build_source = (
        release_build.get("source", {})
        if isinstance(release_build.get("source"), dict)
        else {}
    )
    build_commands = release_build.get("commands", [])
    build_rows = (
        [row for row in build_commands if isinstance(row, dict)]
        if isinstance(build_commands, list)
        else []
    )
    expected_build_commands = [
        (
            "tauri_desktop_bundle",
            "npm.cmd",
            [
                "run",
                "tauri",
                "--",
                "build",
                "--bundles",
                "nsis",
                "--ci",
                "--",
                "--locked",
            ],
        ),
        (
            "locked_release_cli",
            "cargo.exe",
            ["build", "--locked", "--release", "-p", "qpls-cli"],
        ),
    ]
    release_build_commands_valid = len(build_rows) == len(expected_build_commands)
    for row, (expected_id, expected_executable, expected_arguments) in zip(
        build_rows, expected_build_commands, strict=False
    ):
        executable = resolved_declared_path(row.get("executable"))
        logs_valid = True
        for stream in ("stdout", "stderr"):
            binding = row.get(stream)
            log_path = (
                resolved_declared_path(binding.get("path"))
                if isinstance(binding, dict)
                else None
            )
            if (
                not isinstance(binding, dict)
                or log_path is None
                or not log_path.is_file()
                or binding.get("bytes") != log_path.stat().st_size
                or not hash_matches(binding.get("sha256"), sha256_path(log_path))
            ):
                logs_valid = False
        if not (
            row.get("id") == expected_id
            and executable is not None
            and executable.is_file()
            and executable.name.casefold() == expected_executable.casefold()
            and row.get("arguments") == expected_arguments
            and row.get("exit_code") == 0
            and logs_valid
        ):
            release_build_commands_valid = False
    release_build_target = resolved_declared_path(
        release_build.get("target_directory")
    )
    raw_build_receipt_path = release_build.get("receipt_path")
    release_build_receipt = resolved_declared_path(raw_build_receipt_path)
    release_build_receipt_payload = (
        parse_json_path(release_build_receipt)
        if release_build_receipt is not None and release_build_receipt.is_file()
        else None
    )
    release_build_without_receipt_binding = {
        key: value
        for key, value in release_build.items()
        if key not in {"receipt_path", "receipt_sha256"}
    }
    release_build_receipt_valid = (
        isinstance(raw_build_receipt_path, str)
        and Path(raw_build_receipt_path).is_absolute()
        and release_build_target is not None
        and release_build_target.is_dir()
        and release_build_receipt is not None
        and release_build_receipt.is_file()
        and release_build_receipt.name == "v255_build_session.json"
        and release_build_receipt.parent == release_build_target
        and hash_matches(
            release_build.get("receipt_sha256"),
            sha256_path(release_build_receipt),
        )
        and release_build_receipt_payload == release_build_without_receipt_binding
    )
    release_build_policy_valid = (
        set(release_build)
        == {
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
            "receipt_path",
            "receipt_sha256",
        }
        and release_build.get("schema_version") == 2
        and release_build.get("suite_id")
        == "quickpls_unsigned_candidate_build_session_v2"
        and release_build.get("passed") is True
        and release_build.get("target_release") == "2.55.0"
        and release_build_source == release_source
        and release_build.get("target_preexisting") is False
        and release_build.get("environment") == {"CARGO_INCREMENTAL": "0"}
        and release_build.get("minimum_free_gib") == 20.0
        and exact_simple_disk_snapshots(
            release_build.get("disk_snapshots"),
            [
                "before unsigned 2.55 candidate build",
                "after locked unsigned 2.55 candidate build",
            ],
        )
        and exact_build_disk_watcher(
            release_build.get("disk_watcher"),
            started_at=release_build.get("started_at_utc"),
            completed_at=release_build.get("completed_at_utc"),
        )
        and release_build_commands_valid
        and release_build_receipt_valid
    )
    authority_paths = {
        "package.json",
        "package-lock.json",
        "Cargo.toml",
        "Cargo.lock",
        "src-tauri/tauri.conf.json",
        "validation/quickpls_release_channels.json",
    }
    raw_authority_rows = release_source.get("version_authorities", [])
    authority_rows = raw_authority_rows if isinstance(raw_authority_rows, list) else []
    authority_by_path = {
        str(row.get("path", "")): row for row in authority_rows if isinstance(row, dict)
    }
    release_authorities_valid = (
        len(authority_rows) == len(authority_by_path) == len(authority_paths)
        and set(authority_by_path) == authority_paths
        and all(
            (ROOT / relative).is_file()
            and row.get("bytes") == (ROOT / relative).stat().st_size
            and hash_matches(row.get("sha256"), sha256(relative))
            for relative, row in authority_by_path.items()
        )
    )
    release_artifacts = (
        release_report.get("artifacts", []) if isinstance(release_report, dict) else []
    )
    release_rows = (
        [row for row in release_artifacts if isinstance(row, dict)]
        if isinstance(release_artifacts, list)
        else []
    )
    release_by_role = {str(row.get("role", "")): row for row in release_rows}
    release_artifact_paths: dict[str, Path] = {}
    release_artifacts_valid = len(release_rows) == len(release_by_role) == 4 and set(
        release_by_role
    ) == {"portable", "cli", "setup", "checksums"}
    for role, row in release_by_role.items():
        artifact = repository_bound_file(row.get("path"))
        if (
            artifact is None
            or not hash_matches(row.get("sha256"), sha256_path(artifact))
            or row.get("bytes") != artifact.stat().st_size
            or row.get("copy_verified") is not True
        ):
            release_artifacts_valid = False
            continue
        release_artifact_paths[role] = artifact

    build_commit = installed_report.get("candidate_build_source_commit")
    final_to_build = (
        isinstance(final_source_commit, str)
        and isinstance(build_commit, str)
        and subprocess.run(
            [
                "git",
                "-C",
                str(ROOT),
                "merge-base",
                "--is-ancestor",
                final_source_commit,
                build_commit,
            ],
            check=False,
        ).returncode
        == 0
    )
    build_to_publication = (
        isinstance(build_commit, str)
        and isinstance(publication_source_commit, str)
        and subprocess.run(
            [
                "git",
                "-C",
                str(ROOT),
                "merge-base",
                "--is-ancestor",
                build_commit,
                publication_source_commit,
            ],
            check=False,
        ).returncode
        == 0
    )
    build_tree_exit, actual_build_tree = (
        git_output("rev-parse", f"{build_commit}^{{tree}}")
        if isinstance(build_commit, str)
        else (1, "")
    )
    tracked_manifest = (
        subprocess.run(
            [
                "git",
                "-C",
                str(ROOT),
                "ls-tree",
                "-r",
                "-z",
                "--full-tree",
                str(build_commit),
            ],
            check=False,
            capture_output=True,
        )
        if isinstance(build_commit, str)
        else None
    )
    actual_manifest_sha = (
        hashlib.sha256(tracked_manifest.stdout).hexdigest()
        if tracked_manifest is not None
        and tracked_manifest.returncode == 0
        and tracked_manifest.stdout
        else None
    )
    installed_release_waivers = installed_report.get("release_waivers")
    installed_waiver_state_valid = (
        installed_report.get("qualification_status") == "passed"
        and installed_release_waivers == []
    ) or (
        installed_report.get("qualification_status") == "passed_with_waiver"
        and isinstance(installed_release_waivers, list)
        and len(installed_release_waivers) == 1
        and exact_release_waiver_receipt(installed_release_waivers[0])
    )
    checks["publication_installed_and_portable_report_has_exact_provenance"] = (
        installed_report.get("schema_version") == 3
        and installed_report.get("suite_id")
        == "quickpls_v255_installed_portable_smoke_v3"
        and installed_report.get("passed") is True
        and installed_report.get("target_release") == "2.55.0"
        and installed_report.get("package_version") == "2.55.0"
        and installed_report.get("minimum_free_gib") == 20.0
        and installed_report.get("source_worktree_clean") is True
        and installed_report.get("release_publication_evidence_verified") is True
        and installed_report.get("code_signing") is False
        and installed_waiver_state_valid
        and not installed_report.get("error")
        and installed_report.get("consolidated_report_sha256")
        == final_consolidated_hash
        and installed_report.get("tested_source_commit") == final_source_commit
        and installed_report.get("publication_source_commit")
        == publication_source_commit
        and re.fullmatch(r"[0-9a-f]{40}", str(build_commit or "")) is not None
        and re.fullmatch(
            r"[0-9a-f]{40}",
            str(installed_report.get("candidate_build_source_tree", "")),
        )
        is not None
        and is_sha256_any_case(installed_report.get("candidate_source_manifest_sha256"))
        and installed_report.get("candidate_build_source_tree")
        == release_source.get("tree")
        and installed_report.get("candidate_source_manifest_sha256")
        == release_source.get("tracked_manifest_sha256")
        and final_to_build
        and build_to_publication
        and installed_report.get("vitest_report_sha256") == final_vitest_hash
        and installed_report.get("evidence_bundle_sha256")
        == evidence["evidence_bundle_sha256"]
        and release_report_path is not None
        and hash_matches(
            installed_report.get("release_artifact_report_sha256"),
            sha256_path(release_report_path),
        )
        and install_receipt_path is not None
        and hash_matches(
            installed_report.get("install_receipt_sha256"),
            sha256_path(install_receipt_path),
        )
        and len(outcome_rows) == len(outcome_by_name) == 2
        and set(outcome_by_name) == {"portable", "installed"}
        and [str(entry.get("name", "")) for entry in outcome_rows]
        == ["installed", "portable"]
        and len(set(executable_paths)) == 2
        and all(entry.get("status") == "passed" for entry in outcome_rows)
        and all(
            str(entry.get("product_version", "")).startswith("2.55.0")
            for entry in outcome_rows
        )
        and all(
            is_sha256_any_case(entry.get("executable_sha256")) for entry in outcome_rows
        )
    )
    checks["publication_release_artifact_report_is_exact_source_bound_candidate"] = (
        release_report is not None
        and release_report.get("schema_version") == 3
        and release_report.get("passed") is True
        and release_report.get("version") == "2.55.0"
        and release_report.get("release_channel") == "unsigned-preview"
        and release_report.get("label") == "v2_55_0_calculate_evidence"
        and release_trust.get("authenticode_verification_performed") is False
        and release_trust.get("stable_eligible") is False
        and release_source.get("schema_version") == 1
        and release_source.get("worktree_clean") is True
        and release_source.get("commit") == build_commit
        and release_source.get("tree")
        == installed_report.get("candidate_build_source_tree")
        and release_source.get("tracked_manifest_sha256")
        == installed_report.get("candidate_source_manifest_sha256")
        and build_tree_exit == 0
        and actual_build_tree == release_source.get("tree")
        and actual_manifest_sha is not None
        and str(release_source.get("tracked_manifest_sha256", "")).lower()
        == actual_manifest_sha
        and release_authorities_valid
        and release_build_policy_valid
        and release_build_source.get("commit") == build_commit
        and release_artifacts_valid
    )

    install_executable = resolved_declared_path(
        install_receipt.get("installed_executable")
        if isinstance(install_receipt, dict)
        else None
    )
    install_root = resolved_declared_path(
        install_receipt.get("install_root")
        if isinstance(install_receipt, dict)
        else None
    )
    install_release_report = resolved_declared_path(
        install_receipt.get("release_artifact_report")
        if isinstance(install_receipt, dict)
        else None
    )
    install_setup = resolved_declared_path(
        install_receipt.get("setup") if isinstance(install_receipt, dict) else None
    )
    install_portable = resolved_declared_path(
        install_receipt.get("portable_artifact")
        if isinstance(install_receipt, dict)
        else None
    )
    installer_arguments = (
        install_receipt.get("installer_arguments", [])
        if isinstance(install_receipt, dict)
        else []
    )
    installer_destination = (
        resolved_declared_path(installer_arguments[1][3:])
        if isinstance(installer_arguments, list)
        and len(installer_arguments) == 2
        and isinstance(installer_arguments[1], str)
        and installer_arguments[1].startswith("/D=")
        else None
    )
    install_disk = (
        install_receipt.get("disk_snapshots", [])
        if isinstance(install_receipt, dict)
        else []
    )
    install_preflight = (
        install_receipt.get("installation_preflight", {})
        if isinstance(install_receipt, dict)
        and isinstance(install_receipt.get("installation_preflight"), dict)
        else {}
    )
    install_disk_valid = exact_simple_disk_snapshots(
        install_disk,
        ["before isolated NSIS install", "after isolated NSIS install"],
    )
    checks["publication_isolated_nsis_install_receipt_is_exactly_bound"] = (
        install_receipt is not None
        and install_receipt.get("schema_version") == 1
        and install_receipt.get("suite_id") == "quickpls_v255_isolated_nsis_install_v1"
        and install_receipt.get("passed") is True
        and install_receipt.get("target_release") == "2.55.0"
        and install_receipt.get("installation_kind") == "nsis_silent_fresh_destination"
        and install_preflight
        == {
            "running_quickpls_processes": 0,
            "existing_quickpls_registrations": 0,
            "user_installation_preserved": True,
        }
        and install_receipt.get("source_commit") == build_commit
        and install_receipt.get("source_tree") == release_source.get("tree")
        and install_receipt.get("source_manifest_sha256")
        == release_source.get("tracked_manifest_sha256")
        and release_report_path is not None
        and install_release_report == release_report_path
        and hash_matches(
            install_receipt.get("release_artifact_report_sha256"),
            sha256_path(release_report_path),
        )
        and release_artifact_paths.get("setup") is not None
        and install_setup == release_artifact_paths.get("setup")
        and hash_matches(
            install_receipt.get("setup_sha256"),
            sha256_path(release_artifact_paths["setup"]),
        )
        and release_artifact_paths.get("portable") is not None
        and install_portable == release_artifact_paths.get("portable")
        and hash_matches(
            install_receipt.get("portable_artifact_sha256"),
            sha256_path(release_artifact_paths["portable"]),
        )
        and install_receipt.get("install_root_preexisting") is False
        and install_root is not None
        and install_executable is not None
        and install_executable.is_file()
        and install_executable != release_artifact_paths.get("portable")
        and install_executable.is_relative_to(install_root)
        and hash_matches(
            install_receipt.get("installed_executable_sha256"),
            sha256_path(install_executable),
        )
        and sha256_path(install_executable)
        == sha256_path(release_artifact_paths["portable"])
        and isinstance(install_receipt.get("installer_pid"), int)
        and install_receipt.get("installer_pid", 0) > 0
        and isinstance(installer_arguments, list)
        and len(installer_arguments) == 2
        and installer_arguments[0] == "/S"
        and installer_destination == install_root
        and install_receipt.get("installer_exit_code") == 0
        and str(install_receipt.get("product_version", "")).startswith("2.55.0")
        and install_receipt.get("minimum_free_gib") == 20.0
        and install_disk_valid
    )

    portable_posthoc = outcome_by_name.get("portable", {}).get("posthoc_collection", {})
    posthoc_execute_path = (
        repository_bound_file(portable_posthoc.get("execute_receipt"))
        if isinstance(portable_posthoc, dict)
        else None
    )
    posthoc_execute = (
        parse_json_path(posthoc_execute_path)
        if posthoc_execute_path is not None
        else None
    )
    posthoc_reopen_path = repository_bound_file(
        posthoc_execute.get("reopen_verification", {}).get("receipt_path")
        if isinstance(posthoc_execute, dict)
        and isinstance(posthoc_execute.get("reopen_verification"), dict)
        else None
    )
    posthoc_reopen = (
        parse_json_path(posthoc_reopen_path)
        if posthoc_reopen_path is not None
        else None
    )
    posthoc_identity = (
        posthoc_execute.get("scientific_identity", {})
        if isinstance(posthoc_execute, dict)
        and isinstance(posthoc_execute.get("scientific_identity"), dict)
        else {}
    )
    posthoc_result = (
        posthoc_execute.get("result_verification", {})
        if isinstance(posthoc_execute, dict)
        and isinstance(posthoc_execute.get("result_verification"), dict)
        else {}
    )
    posthoc_result_identity = (
        posthoc_execute.get("result_identity", {})
        if isinstance(posthoc_execute, dict)
        and isinstance(posthoc_execute.get("result_identity"), dict)
        else {}
    )
    posthoc_lifecycle = (
        posthoc_execute.get("lifecycle", {})
        if isinstance(posthoc_execute, dict)
        and isinstance(posthoc_execute.get("lifecycle"), dict)
        else {}
    )
    checks["publication_posthoc_new_run_and_fresh_reopen_are_exactly_bound"] = (
        isinstance(portable_posthoc, dict)
        and portable_posthoc.get("status") == "passed"
        and portable_posthoc.get("generated") is True
        and resolved_declared_path(portable_posthoc.get("candidate"))
        == release_artifact_paths.get("portable")
        and release_artifact_paths.get("portable") is not None
        and hash_matches(
            portable_posthoc.get("candidate_sha256"),
            sha256_path(release_artifact_paths["portable"]),
        )
        and portable_posthoc.get("build_source_commit") == build_commit
        and release_report_path is not None
        and hash_matches(
            portable_posthoc.get("release_artifact_report_sha256"),
            sha256_path(release_report_path),
        )
        and posthoc_execute_path is not None
        and portable_posthoc.get("execute_receipt_sha256")
        == sha256_path(posthoc_execute_path)
        and posthoc_execute is not None
        and posthoc_execute.get("schema")
        == "quickpls.v255.posthoc_minimum_sample_packaged_smoke.v1"
        and posthoc_execute.get("suite_id")
        == "quickpls_v255_posthoc_minimum_sample_packaged_smoke_v1"
        and posthoc_execute.get("target_release") == "2.55.0"
        and posthoc_execute.get("status") == "passed"
        and posthoc_execute.get("phase") == "execute"
        and posthoc_execute.get("method_kind")
        == "pls_posthoc_technical_minimum_sample_size"
        and posthoc_execute.get("new_result_id") == posthoc_result_identity.get("value")
        and posthoc_identity.get("capability_cell")
        == "qpls3.pls.posthoc_technical_minimum_sample_size"
        and posthoc_identity.get("capability_version")
        == "pls_posthoc_technical_minimum_sample_size_v2"
        and posthoc_identity.get("method_version") == "inverse_square_root_posthoc_v2"
        and posthoc_lifecycle.get("terminal_state") == "completed"
        and posthoc_result.get("selected_run_id")
        == posthoc_execute.get("new_result_id")
        and posthoc_result.get("selected_table_title") == "Post-hoc minimum sample size"
        and posthoc_reopen_path is not None
        and resolved_declared_path(portable_posthoc.get("reopen_receipt"))
        == posthoc_reopen_path
        and portable_posthoc.get("reopen_receipt_sha256")
        == sha256_path(posthoc_reopen_path)
        and posthoc_reopen is not None
        and posthoc_reopen.get("schema")
        == "quickpls.v255.posthoc_minimum_sample_reopen.v1"
        and posthoc_reopen.get("suite_id")
        == "quickpls_v255_posthoc_minimum_sample_packaged_smoke_v1"
        and posthoc_reopen.get("target_release") == "2.55.0"
        and posthoc_reopen.get("status") == "passed"
        and posthoc_reopen.get("phase") == "reopen"
        and posthoc_reopen.get("same_result_identity") is True
        and posthoc_reopen.get("same_archive_sha256") is True
        and posthoc_reopen.get("new_result_id") == posthoc_execute.get("new_result_id")
        and posthoc_reopen.get("archive_sha256")
        == posthoc_execute.get("archive_sha256")
        and portable_posthoc.get("result_id") == posthoc_execute.get("new_result_id")
    )

    nested_reports_valid = True
    nested_method_hashes: dict[str, str] = {}
    nested_method_paths: dict[str, Path] = {}
    all_candidate_pids: list[int] = []

    def installed_report_member(declared: object) -> Path | None:
        if not isinstance(declared, str) or not declared:
            return None
        candidate = Path(declared)
        if not candidate.is_absolute():
            candidate = installed_report_path.parent / candidate
        try:
            resolved = candidate.resolve(strict=True)
            resolved.relative_to(installed_report_path.parent.resolve())
        except (OSError, ValueError):
            return None
        return resolved if resolved.is_file() else None

    for name, outcome in outcome_by_name.items():
        executable = resolved_declared_path(outcome.get("executable"))
        lifecycle = installed_report_member(outcome.get("lifecycle"))
        nested_method = installed_report_member(outcome.get("evidence"))
        pids = outcome.get("launched_pids", [])
        expected_executable = (
            install_executable
            if name == "installed"
            else release_artifact_paths.get("portable")
        )
        expected_candidate_kind = (
            "fresh_nsis_install" if name == "installed" else "portable_release_artifact"
        )
        if (
            executable is None
            or not executable.is_file()
            or executable != expected_executable
            or not hash_matches(
                outcome.get("executable_sha256"), sha256_path(executable)
            )
            or outcome.get("candidate_kind") != expected_candidate_kind
            or outcome.get("build_source_commit") != build_commit
            or outcome.get("source_tree") != release_source.get("tree")
            or outcome.get("source_manifest_sha256")
            != release_source.get("tracked_manifest_sha256")
            or resolved_declared_path(outcome.get("release_artifact_report"))
            != release_report_path
            or release_report_path is None
            or not hash_matches(
                outcome.get("release_artifact_report_sha256"),
                sha256_path(release_report_path),
            )
            or (
                name == "installed"
                and (
                    resolved_declared_path(outcome.get("install_receipt"))
                    != install_receipt_path
                    or install_receipt_path is None
                    or not hash_matches(
                        outcome.get("install_receipt_sha256"),
                        sha256_path(install_receipt_path),
                    )
                )
            )
            or (
                name == "portable"
                and (
                    outcome.get("install_receipt") is not None
                    or outcome.get("install_receipt_sha256") is not None
                )
            )
            or lifecycle is None
            or sha256_path(lifecycle) != outcome.get("lifecycle_sha256")
            or nested_method is None
            or sha256_path(nested_method) != outcome.get("evidence_sha256")
            or not isinstance(pids, list)
            or len(pids) < 3
            or len(pids) != len(set(pids))
            or not all(isinstance(pid, int) and pid > 0 for pid in pids)
        ):
            nested_reports_valid = False
            continue
        all_candidate_pids.extend(pids)
        nested_method_paths[name] = nested_method
        lifecycle_payload = parse_json_path(lifecycle)
        nested_method_payload = parse_json_path(nested_method)
        nested_setups = (
            nested_method_payload.get("setups", [])
            if isinstance(nested_method_payload, dict)
            else []
        )
        nested_captures = (
            nested_method_payload.get("calculate_captures", [])
            if isinstance(nested_method_payload, dict)
            else []
        )
        nested_archives = (
            nested_method_payload.get("archive_inventory", [])
            if isinstance(nested_method_payload, dict)
            else []
        )
        nested_sources = (
            nested_method_payload.get("sources", {})
            if isinstance(nested_method_payload, dict)
            and isinstance(nested_method_payload.get("sources"), dict)
            else {}
        )
        if (
            lifecycle_payload is None
            or lifecycle_payload.get("schema_version") != 1
            or lifecycle_payload.get("suite_id")
            != "quickpls_v255_live_calculation_lifecycle_smoke_v1"
            or lifecycle_payload.get("target_release") != "2.55.0"
            or lifecycle_payload.get("complete") is not True
            or lifecycle_payload.get("passed") is not True
            or lifecycle_payload.get("failures")
            or nested_method_payload is None
            or nested_method_payload.get("schema_version") != 2
            or nested_method_payload.get("suite_id")
            != "quickpls_v255_method_evidence_crawler_v2"
            or nested_method_payload.get("target_release") != "2.55.0"
            or nested_method_payload.get("mode") != "packaged"
            or nested_method_payload.get("passed") is not True
            or nested_method_payload.get("result_evidence_phase") != "publication"
            or nested_method_payload.get("offline", {}).get("passed") is not True
            or nested_method_payload.get("console_errors") != []
            or nested_method_payload.get("evidence_bundle", {}).get("passed")
            is not True
            or nested_sources.get("matrix_sha256") != matrix_hash
            or nested_sources.get("frozen_archive_index_sha256") != index_hash
            or nested_sources.get("reusable_archive_inventory_sha256") != inventory_hash
            or nested_sources.get("evidence_bundle_manifest_sha256")
            != bundle_manifest_hash
            or nested_sources.get("vitest_report_sha256") != final_vitest_hash
            or not isinstance(nested_setups, list)
            or len(nested_setups) != 64
            or len(
                {
                    (row.get("kind"), row.get("setup_case"))
                    for row in nested_setups
                    if isinstance(row, dict)
                }
            )
            != 64
            or any(
                not isinstance(row, dict)
                or row.get("status") != "passed"
                or row.get("executed") is not True
                for row in nested_setups
            )
            or not isinstance(nested_captures, list)
            or len(nested_captures) != 18
            or {row.get("kind") for row in nested_captures if isinstance(row, dict)}
            != set(catalogue_kinds())
            or any(
                not isinstance(row, dict)
                or row.get("status") != "passed"
                or row.get("executed") is not True
                for row in nested_captures
            )
            or not isinstance(nested_archives, list)
            or len(nested_archives) != 18
            or {row.get("kind") for row in nested_archives if isinstance(row, dict)}
            != set(catalogue_kinds())
            or any(
                not isinstance(row, dict) or row.get("passed") is not True
                for row in nested_archives
            )
        ):
            nested_reports_valid = False
        nested_method_hashes[name] = sha256_path(nested_method)
    checks["publication_nested_candidate_reports_and_pid_ownership_are_verified"] = (
        nested_reports_valid
        and len(all_candidate_pids) >= 6
        and len(all_candidate_pids) == len(set(all_candidate_pids))
    )
    checks["publication_supplied_method_report_is_the_portable_candidate_report"] = (
        outcome_by_name.get("portable", {}).get("evidence_sha256")
        == evidence["method_publication_report_sha256"]
        and nested_method_hashes.get("portable")
        == evidence["method_publication_report_sha256"]
        and nested_method_paths.get("portable") == method_report_path.resolve()
    )

    # The curated named-evidence bundle verifies the observations a second
    # time, but the candidate smoke is the trust boundary that launched the
    # drivers. Verify every bound source report here as well so a publication
    # report cannot omit, substitute, or merely name a generic/cross driver.
    named_manifest = read_json(NAMED_ROUTE_MANIFEST_PATH)
    cross_manifest = read_json(CROSS_METHOD_ROUTE_MANIFEST_PATH)
    named_manifest_cases = (
        [row for row in named_manifest.get("cases", []) if isinstance(row, dict)]
        if isinstance(named_manifest, dict)
        and isinstance(named_manifest.get("cases"), list)
        else []
    )
    named_manifest_ids_by_candidate = {
        candidate: {
            str(row.get("id"))
            for row in named_manifest_cases
            if row.get("candidate") == candidate and isinstance(row.get("id"), str)
        }
        for candidate in ("installed", "portable")
    }
    index_rows = named_evidence_index.get("entries", [])
    index_by_id = {
        str(row.get("id")): row
        for row in index_rows
        if isinstance(index_rows, list)
        and isinstance(row, dict)
        and isinstance(row.get("id"), str)
    }
    operation_by_group = (
        named_evidence_index.get("collector_contract", {}).get(
            "operation_by_group", {}
        )
        if isinstance(named_evidence_index.get("collector_contract"), dict)
        else {}
    )
    operation_by_group = (
        operation_by_group if isinstance(operation_by_group, dict) else {}
    )
    expected_operation_by_id = {
        case_id: operation_by_group.get(row.get("group"))
        for case_id, row in index_by_id.items()
    }

    bound_driver_reports_valid = True
    bound_report_paths: list[Path] = []
    bound_report_hashes: list[str] = []
    observed_case_ids: list[str] = []
    observed_waived_case_ids: list[str] = []
    observed_screenshot_paths: list[Path] = []
    observed_screenshot_hashes: list[str] = []
    generic_reports_by_candidate: dict[str, int] = {
        "installed": 0,
        "portable": 0,
    }
    cross_report_count = 0

    for candidate, outcome in outcome_by_name.items():
        raw_bindings = outcome.get("named_evidence_driver_reports", [])
        bindings = (
            [row for row in raw_bindings if isinstance(row, dict)]
            if isinstance(raw_bindings, list)
            else []
        )
        expected_binding_count = 1 if candidate == "installed" else 2
        if len(bindings) != len(raw_bindings) or len(bindings) != expected_binding_count:
            bound_driver_reports_valid = False
        for binding in bindings:
            report_member = installed_report_member(binding.get("path"))
            if (
                set(binding) != {"path", "sha256"}
                or report_member is None
                or not hash_matches(
                    binding.get("sha256"), sha256_path(report_member)
                )
            ):
                bound_driver_reports_valid = False
                continue
            payload = parse_json_path(report_member)
            if payload is None:
                bound_driver_reports_valid = False
                continue
            bound_report_paths.append(report_member)
            bound_report_hashes.append(sha256_path(report_member))
            suite_id = payload.get("suite_id")
            observations = payload.get("named_evidence_observations", [])
            observation_rows = (
                [row for row in observations if isinstance(row, dict)]
                if isinstance(observations, list)
                else []
            )
            if len(observation_rows) != len(observations):
                bound_driver_reports_valid = False

            if suite_id == "quickpls_v255_named_case_driver_v1":
                generic_reports_by_candidate[candidate] += 1
                sources = (
                    payload.get("sources", {})
                    if isinstance(payload.get("sources"), dict)
                    else {}
                )
                candidate_process = (
                    payload.get("candidate_process", {})
                    if isinstance(payload.get("candidate_process"), dict)
                    else {}
                )
                process_safety = (
                    payload.get("process_safety", {})
                    if isinstance(payload.get("process_safety"), dict)
                    else {}
                )
                outcome_executable = resolved_declared_path(
                    outcome.get("executable")
                )
                outcome_pids = outcome.get("launched_pids", [])
                cases = payload.get("cases", [])
                case_rows = (
                    [row for row in cases if isinstance(row, dict)]
                    if isinstance(cases, list)
                    else []
                )
                case_ids = [str(row.get("id", "")) for row in case_rows]
                observation_ids = [
                    str(row.get("case_id", "")) for row in observation_rows
                ]
                if not (
                    payload.get("schema_version") == 1
                    and payload.get("target_release") == "2.55.0"
                    and payload.get("status") == "passed"
                    and payload.get("passed") is True
                    and payload.get("candidate") == candidate
                    and payload.get("failures") == []
                    and payload.get("serial") is True
                    and payload.get("maximum_concurrent_cases") == 1
                    and payload.get("offline", {}).get("passed") is True
                    and isinstance(candidate_process.get("pid"), int)
                    and candidate_process.get("pid") > 0
                    and isinstance(outcome_pids, list)
                    and candidate_process.get("pid") in outcome_pids
                    and resolved_declared_path(
                        candidate_process.get("executable")
                    )
                    == outcome_executable
                    and outcome_executable is not None
                    and hash_matches(
                        candidate_process.get("executable_sha256"),
                        sha256_path(outcome_executable),
                    )
                    and process_safety
                    == {
                        "wrapper_owns_candidate_process": True,
                        "candidate_pid_bound": True,
                        "candidate_executable_bound": True,
                        "driver_launches_candidate_processes": False,
                        "driver_terminates_candidate_processes": False,
                        "driver_closes_browser_page_or_context": False,
                    }
                    and repository_bound_file(sources.get("manifest"))
                    == (ROOT / NAMED_ROUTE_MANIFEST_PATH).resolve()
                    and hash_matches(
                        sources.get("manifest_sha256"),
                        sha256(NAMED_ROUTE_MANIFEST_PATH),
                    )
                    and repository_bound_file(sources.get("index"))
                    == (ROOT / "validation/v255_named_evidence_index.json").resolve()
                    and hash_matches(
                        sources.get("index_sha256"),
                        sha256("validation/v255_named_evidence_index.json"),
                    )
                    and len(case_rows) == len(cases) == len(observation_rows)
                    and len(case_ids) == len(set(case_ids))
                    and set(case_ids)
                    == named_manifest_ids_by_candidate.get(candidate, set())
                    and observation_ids == case_ids
                    and all(row.get("status") == "passed" for row in case_rows)
                    and all(
                        row.get("observation") == observation
                        for row, observation in zip(
                            case_rows, observation_rows, strict=True
                        )
                    )
                ):
                    bound_driver_reports_valid = False
            elif suite_id == "quickpls_v255_cross_method_candidate_wrapper_v1":
                cross_report_count += 1
                cross_release_waivers = payload.get("release_waivers")
                cross_waiver_state_valid = (
                    payload.get("qualification_status") == "passed"
                    and cross_release_waivers == []
                ) or (
                    payload.get("qualification_status") == "passed_with_waiver"
                    and isinstance(cross_release_waivers, list)
                    and len(cross_release_waivers) == 1
                    and exact_release_waiver_receipt(cross_release_waivers[0])
                )
                manifest_binding = (
                    payload.get("manifest", {})
                    if isinstance(payload.get("manifest"), dict)
                    else {}
                )
                release_binding = (
                    payload.get("release_artifact_report", {})
                    if isinstance(payload.get("release_artifact_report"), dict)
                    else {}
                )
                candidate_binding = (
                    payload.get("candidate", {})
                    if isinstance(payload.get("candidate"), dict)
                    else {}
                )
                source_state = (
                    payload.get("source_state", {})
                    if isinstance(payload.get("source_state"), dict)
                    else {}
                )
                allowed_runtime_root = resolved_declared_path(
                    source_state.get("allowed_runtime_evidence_root")
                )
                observation_ids = [
                    str(row.get("case_id", "")) for row in observation_rows
                ]
                if not (
                    candidate == "portable"
                    and payload.get("schema_version") == 1
                    and payload.get("target_release") == "2.55.0"
                    and payload.get("passed") is True
                    and cross_waiver_state_valid
                    and payload.get("failures") == []
                    and payload.get("source_commit") == build_commit
                    and payload.get("publication_commit")
                    == publication_source_commit
                    and source_state.get("tracked_worktree_clean") is True
                    and source_state.get(
                        "untracked_paths_confined_to_runtime_evidence"
                    )
                    is True
                    and allowed_runtime_root == installed_report_path.parent.resolve()
                    and repository_bound_file(manifest_binding.get("path"))
                    == (ROOT / CROSS_METHOD_ROUTE_MANIFEST_PATH).resolve()
                    and hash_matches(
                        manifest_binding.get("sha256"),
                        sha256(CROSS_METHOD_ROUTE_MANIFEST_PATH),
                    )
                    and resolved_declared_path(release_binding.get("path"))
                    == release_report_path
                    and release_report_path is not None
                    and hash_matches(
                        release_binding.get("sha256"),
                        sha256_path(release_report_path),
                    )
                    and resolved_declared_path(candidate_binding.get("path"))
                    == release_artifact_paths.get("portable")
                    and release_artifact_paths.get("portable") is not None
                    and hash_matches(
                        candidate_binding.get("sha256"),
                        sha256_path(release_artifact_paths["portable"]),
                    )
                    and candidate_binding.get("product_version") == "2.55.0"
                    and len(observation_ids)
                    == len(set(observation_ids))
                    == len(EXPECTED_CROSS_METHOD_WRAPPER_CASE_IDS)
                    and set(observation_ids)
                    == EXPECTED_CROSS_METHOD_WRAPPER_CASE_IDS
                ):
                    bound_driver_reports_valid = False
            else:
                bound_driver_reports_valid = False

            for observation in observation_rows:
                case_id = observation.get("case_id")
                waived = exact_waived_observation(observation)
                installed_waiver_matches_observation = (
                    not waived
                    or (
                        isinstance(installed_release_waivers, list)
                        and len(installed_release_waivers) == 1
                        and exact_release_waiver_matches_observation(
                            installed_release_waivers[0], observation
                        )
                    )
                )
                cross_report_waiver_matches_observation = (
                    not waived
                    or exact_cross_report_waiver_binding(payload, observation)
                )
                operation = expected_operation_by_id.get(str(case_id))
                assertion = (
                    observation.get("assertion", {})
                    if isinstance(observation.get("assertion"), dict)
                    else {}
                )
                screenshot = (
                    observation.get("screenshot", {})
                    if isinstance(observation.get("screenshot"), dict)
                    else {}
                )
                screenshot_path = installed_report_member(screenshot.get("path"))
                if not (
                    observation.get("schema_version") == 1
                    and isinstance(case_id, str)
                    and case_id in index_by_id
                    and observation.get("operation") == operation
                    and assertion.get("id") == f"{operation}:{case_id}"
                    and (
                        waived
                        or (
                            observation.get("status") is None
                            and observation.get("waiver") is None
                            and assertion.get("passed") is True
                            and assertion.get("expected") is not None
                            and assertion.get("observed") == assertion.get("expected")
                        )
                    )
                    and installed_waiver_matches_observation
                    and cross_report_waiver_matches_observation
                    and screenshot_path is not None
                    and hash_matches(
                        screenshot.get("sha256"), sha256_path(screenshot_path)
                    )
                    and screenshot_path.read_bytes().startswith(b"\x89PNG\r\n\x1a\n")
                ):
                    bound_driver_reports_valid = False
                    continue
                observed_case_ids.append(case_id)
                if waived:
                    observed_waived_case_ids.append(case_id)
                observed_screenshot_paths.append(screenshot_path)
                observed_screenshot_hashes.append(sha256_path(screenshot_path))

    expected_driver_case_ids = set().union(
        *named_manifest_ids_by_candidate.values(),
        EXPECTED_CROSS_METHOD_WRAPPER_CASE_IDS,
    )
    checks[
        "publication_candidate_bound_named_and_cross_driver_reports_are_exact"
    ] = (
        bound_driver_reports_valid
        and generic_reports_by_candidate == {"installed": 1, "portable": 1}
        and cross_report_count == 1
        and len(bound_report_paths) == len(set(bound_report_paths)) == 3
        and len(bound_report_hashes) == len(set(bound_report_hashes)) == 3
        and len(observed_case_ids) == len(set(observed_case_ids)) == 47
        and set(observed_case_ids) == expected_driver_case_ids
        and (
            (
                installed_report.get("qualification_status") == "passed"
                and observed_waived_case_ids == []
            )
            or (
                installed_report.get("qualification_status")
                == "passed_with_waiver"
                and observed_waived_case_ids == [DPI_WAIVER_CASE_ID]
            )
        )
        and len(observed_screenshot_paths)
        == len(set(observed_screenshot_paths))
        == 47
        and len(observed_screenshot_hashes)
        == len(set(observed_screenshot_hashes))
        == 47
    )
    evidence["candidate_bound_named_driver_report_sha256"] = sorted(
        bound_report_hashes
    )

    disk_snapshots = installed_report.get("disk_snapshots", [])
    checks["publication_packaged_smoke_kept_both_drives_above_20_gib"] = (
        exact_simple_disk_snapshots(
            disk_snapshots,
            [
                "before packaged smoke",
                "between installed and portable smoke",
                "after packaged smoke",
            ],
        )
    )

    frozen_receipts = frozen_report.get("method_receipts", [])
    frozen_kinds = (
        [
            entry.get("method_kind")
            for entry in frozen_receipts
            if isinstance(entry, dict)
        ]
        if isinstance(frozen_receipts, list)
        else []
    )
    frozen_members_verified = True
    frozen_payloads_verified = True
    if isinstance(frozen_receipts, list):
        staging = frozen_reopen_report_path.parent.parent

        def staged_artifact(payload: object, require_png: bool = False) -> bool:
            if not isinstance(payload, dict):
                return False
            member = payload.get("member")
            if (
                not isinstance(member, str)
                or "\\" in member
                or member.startswith("/")
                or ":" in member
                or ".." in member.split("/")
            ):
                return False
            candidate = (staging / Path(*member.split("/"))).resolve()
            try:
                candidate.relative_to(staging.resolve())
            except ValueError:
                return False
            if (
                not candidate.is_file()
                or not is_sha256(payload.get("sha256"))
                or sha256_path(candidate) != payload.get("sha256")
            ):
                return False
            if require_png and not candidate.read_bytes().startswith(
                b"\x89PNG\r\n\x1a\n"
            ):
                return False
            return True

        for entry in frozen_receipts:
            if not isinstance(entry, dict):
                frozen_members_verified = False
                continue
            member = entry.get("member")
            if (
                not isinstance(member, str)
                or "\\" in member
                or member.startswith("/")
                or ":" in member
                or ".." in member.split("/")
            ):
                frozen_members_verified = False
                continue
            candidate = (staging / Path(*member.split("/"))).resolve()
            try:
                candidate.relative_to(staging.resolve())
            except ValueError:
                frozen_members_verified = False
                continue
            if not candidate.is_file() or sha256_path(candidate) != entry.get("sha256"):
                frozen_members_verified = False
                continue
            payload = parse_json_path(candidate)
            identity = (
                payload.get("declared_identity", {})
                if isinstance(payload, dict)
                else {}
            )
            identity_check = (
                payload.get("identity_verification", {})
                if isinstance(payload, dict)
                else {}
            )
            source_receipt = (
                payload.get("source_receipt", {}) if isinstance(payload, dict) else {}
            )
            labels = (
                payload.get("observed_results_labels", {})
                if isinstance(payload, dict)
                else {}
            )
            if (
                payload is None
                or payload.get("schema_version") != 1
                or payload.get("suite_id")
                != "quickpls_v255_frozen_archive_reopen_crawler_v1"
                or payload.get("target_release") != "2.55.0"
                or payload.get("status") != "verified_current_ui_capture"
                or payload.get("method_kind") != entry.get("method_kind")
                or not isinstance(identity, dict)
                or identity.get("type")
                not in {"canonical_result_document_id", "schema5_result_run_id"}
                or not isinstance(identity.get("value"), str)
                or not identity.get("value")
                or not isinstance(identity_check, dict)
                or identity_check.get("passed") is not True
                or not isinstance(source_receipt, dict)
                or not (
                    source_receipt.get("declared_identity_directly_bound") is True
                    or source_receipt.get("identity_recovered_from_archive") is True
                )
                or (
                    entry.get("method_kind")
                    == "pls_posthoc_technical_minimum_sample_size"
                    and source_receipt.get("declared_identity_directly_bound")
                    is not True
                )
                or not isinstance(labels, dict)
                or not labels.get("selected_result")
                or not staged_artifact(payload.get("archive"))
                or not staged_artifact(payload.get("source_receipt"))
                or not staged_artifact(payload.get("screenshot"), require_png=True)
            ):
                frozen_payloads_verified = False
    else:
        frozen_members_verified = False
        frozen_payloads_verified = False
    checks["publication_frozen_reopen_crawler_passed_and_inventory_bound"] = (
        frozen_report.get("schema_version") == 1
        and frozen_report.get("suite_id")
        == "quickpls_v255_frozen_archive_reopen_crawler_v1"
        and frozen_report.get("status") == "passed"
        and frozen_report.get("target_release") == "2.55.0"
        and frozen_report.get("source_inventory", {}).get("sha256") == inventory_hash
        and len(frozen_receipts) == len(set(frozen_kinds)) == 18
        and set(frozen_kinds) == set(catalogue_kinds())
        and all(
            entry.get("status") == "verified_current_ui_capture"
            and is_sha256(entry.get("sha256"))
            for entry in frozen_receipts
            if isinstance(entry, dict)
        )
        and frozen_members_verified
        and frozen_payloads_verified
        and not frozen_report.get("failures")
        and frozen_report.get("process_safety", {}).get(
            "wrapper_owns_exact_pid_lifecycle"
        )
        is True
        and frozen_report.get("process_safety", {}).get("driver_launches_app_processes")
        is False
        and frozen_report.get("process_safety", {}).get(
            "driver_terminates_app_processes"
        )
        is False
        and frozen_report.get("process_safety", {}).get(
            "driver_closes_browser_page_or_context"
        )
        is False
    )
    portable_frozen = outcome_by_name.get("portable", {}).get(
        "frozen_archive_collection", {}
    )
    portable_frozen_receipt = (
        installed_report_member(portable_frozen.get("aggregate_receipt"))
        if isinstance(portable_frozen, dict)
        else None
    )
    checks["publication_supplied_frozen_report_is_the_portable_candidate_report"] = (
        isinstance(portable_frozen, dict)
        and portable_frozen.get("status") == "passed"
        and portable_frozen.get("aggregate_receipt_sha256")
        == evidence["frozen_archive_reopen_report_sha256"]
        and portable_frozen_receipt == frozen_reopen_report_path.resolve()
    )
    expected_bundle_hash = (
        bundle_manifest.get("bundle", {}).get("sha256")
        if isinstance(bundle_manifest.get("bundle"), dict)
        else None
    )
    checks["publication_zip_matches_verified_manifest"] = (
        bundle_manifest.get("schema_version") == 1
        and bundle_manifest.get("target_release") == "2.55.0"
        and bundle_manifest.get("status") in {"verified", "verified_with_waiver"}
        and bundle_manifest.get("named_evidence", {}).get(
            "approved_release_waiver"
        )
        == DPI_WAIVER_MANIFEST_DECLARATION
        and isinstance(expected_bundle_hash, str)
        and is_sha256(expected_bundle_hash)
        and sha256_path(evidence_bundle_path) == expected_bundle_hash.lower()
    )
    zip_checks, zip_evidence = closed_evidence_zip_checks(
        evidence_bundle_path,
        frozen_archive_index,
        named_evidence_index,
        bundle_manifest,
    )
    checks.update(zip_checks)
    evidence.update(zip_evidence)
    extraction_capacity = installed_report.get(
        "evidence_bundle_extraction_capacity", {}
    )
    reserve_bytes = 256 * 1024 * 1024
    required_d_bytes = (
        20 * 1024 * 1024 * 1024
        + int(
            zip_evidence.get(
                "evidence_zip_total_uncompressed_bytes", -reserve_bytes - 1
            )
        )
        + reserve_bytes
    )
    checks["publication_bundle_extraction_capacity_is_exactly_bound"] = (
        isinstance(extraction_capacity, dict)
        and extraction_capacity.get("passed") is True
        and extraction_capacity.get("verified_member_count")
        == zip_evidence.get("evidence_zip_observed_members")
        and extraction_capacity.get("verified_compressed_bytes")
        == zip_evidence.get("evidence_zip_total_compressed_member_bytes")
        and extraction_capacity.get("verified_uncompressed_bytes")
        == zip_evidence.get("evidence_zip_total_uncompressed_bytes")
        and extraction_capacity.get("reserve_bytes") == reserve_bytes
        and extraction_capacity.get("required_d_free_bytes_strictly_above")
        == required_d_bytes
        and isinstance(extraction_capacity.get("observed_d_free_bytes"), int)
        and extraction_capacity.get("observed_d_free_bytes", 0) > required_d_bytes
    )
    return checks, evidence


def declared_named_evidence_checks(matrix: dict[str, object]) -> dict[str, bool]:
    def exact_declarations(group: object) -> bool:
        if not isinstance(group, dict):
            return False
        required = group.get("required", [])
        evidence = group.get("evidence", [])
        return (
            isinstance(required, list)
            and isinstance(evidence, list)
            and len(required) == len(evidence)
            and {
                str(item.get("case", "")) for item in evidence if isinstance(item, dict)
            }
            == set(required)
        )

    cross = matrix.get("cross_method_evidence", {})
    return {
        "every_setup_case_has_one_declared_evidence_route": all(
            len(method.get("setup_evidence", [])) == len(method.get("setup_cases", []))
            and {
                str(item.get("setup_case", ""))
                for item in method.get("setup_evidence", [])
                if isinstance(item, dict)
            }
            == set(method.get("setup_cases", []))
            for method in matrix.get("methods", [])
        ),
        "every_public_method_has_one_declared_browser_calculate_capture": (
            len(matrix.get("calculate_capture_evidence", [])) == 18
            and {
                str(item.get("kind", ""))
                for item in matrix.get("calculate_capture_evidence", [])
                if isinstance(item, dict)
            }
            == {
                str(method.get("kind", ""))
                for method in matrix.get("methods", [])
                if isinstance(method, dict)
            }
            and all(
                item.get("evidence_type") == "browser" and item.get("status") == "ready"
                for item in matrix.get("calculate_capture_evidence", [])
                if isinstance(item, dict)
            )
        ),
        "every_cross_method_case_has_one_declared_evidence_route": isinstance(
            cross, dict
        )
        and all(
            exact_declarations(cross.get(group))
            for group in (
                "imports",
                "exports",
                "persistence",
                "accessibility",
                "observability",
                "packaged",
            )
        ),
        "every_specialized_result_case_has_one_declared_evidence_route": exact_declarations(
            matrix.get("specialized_result_evidence")
        ),
    }


def immutable_matrix_contract_checks(matrix: dict[str, object]) -> dict[str, bool]:
    methods = matrix.get("methods", [])
    setup_contract = (
        [
            f"{method.get('kind')}\0{case}"
            for method in methods
            if isinstance(method, dict)
            for case in method.get("setup_cases", [])
            if isinstance(case, str)
        ]
        if isinstance(methods, list)
        else []
    )
    specialized = matrix.get("specialized_result_evidence", {})
    specialized_contract = (
        specialized.get("required", []) if isinstance(specialized, dict) else []
    )
    journeys = matrix.get("cross_method_journeys", {})
    cross_contract = (
        [
            f"{group}\0{case}"
            for group, cases in journeys.items()
            if isinstance(cases, list)
            for case in cases
            if isinstance(case, str)
        ]
        if isinstance(journeys, dict)
        else []
    )
    catalogue_contract = matrix.get("catalogue_contract", {})
    if not isinstance(catalogue_contract, dict):
        catalogue_contract = {}
    return {
        "matrix_freezes_exactly_64_setup_cases": (
            len(setup_contract) == len(set(setup_contract)) == 64
            and sha256_lines(setup_contract) == EXPECTED_SETUP_CONTRACT_SHA256
            and catalogue_contract.get("expected_setup_cases") == 64
            and catalogue_contract.get("setup_case_contract_sha256")
            == EXPECTED_SETUP_CONTRACT_SHA256
        ),
        "matrix_freezes_exactly_26_specialized_result_cases": (
            isinstance(specialized_contract, list)
            and len(specialized_contract) == len(set(specialized_contract)) == 26
            and all(isinstance(case, str) for case in specialized_contract)
            and sha256_lines(specialized_contract)
            == EXPECTED_SPECIALIZED_CONTRACT_SHA256
            and catalogue_contract.get("expected_specialized_result_cases") == 26
            and catalogue_contract.get("specialized_result_contract_sha256")
            == EXPECTED_SPECIALIZED_CONTRACT_SHA256
        ),
        "matrix_freezes_exactly_29_cross_method_cases": (
            len(cross_contract) == len(set(cross_contract)) == 29
            and sha256_lines(cross_contract) == EXPECTED_CROSS_METHOD_CONTRACT_SHA256
            and catalogue_contract.get("expected_cross_method_cases") == 29
            and catalogue_contract.get("cross_method_contract_sha256")
            == EXPECTED_CROSS_METHOD_CONTRACT_SHA256
        ),
    }


def declared_named_evidence_index_checks(
    matrix: dict[str, object], named_index: dict[str, object]
) -> tuple[dict[str, bool], set[str]]:
    expected: list[tuple[str, str, str, str]] = []
    matrix_statuses: list[object] = []
    cross = matrix.get("cross_method_evidence", {})
    if isinstance(cross, dict):
        for group in (
            "imports",
            "exports",
            "persistence",
            "accessibility",
            "observability",
            "packaged",
        ):
            declaration = cross.get(group, {})
            if not isinstance(declaration, dict):
                continue
            required = declaration.get("required", [])
            if isinstance(required, list):
                expected.extend(
                    (f"cross_method:{group}:{case}", "cross_method", group, str(case))
                    for case in required
                    if isinstance(case, str)
                )
            evidence = declaration.get("evidence", [])
            if isinstance(evidence, list):
                matrix_statuses.extend(
                    item.get("status") for item in evidence if isinstance(item, dict)
                )
    specialized = matrix.get("specialized_result_evidence", {})
    if isinstance(specialized, dict):
        required = specialized.get("required", [])
        if isinstance(required, list):
            expected.extend(
                (
                    f"specialized_result:{case}",
                    "specialized_result",
                    "specialized_result_evidence",
                    str(case),
                )
                for case in required
                if isinstance(case, str)
            )
        evidence = specialized.get("evidence", [])
        if isinstance(evidence, list):
            matrix_statuses.extend(
                item.get("status") for item in evidence if isinstance(item, dict)
            )

    raw_entries = named_index.get("entries", [])
    entries = (
        [item for item in raw_entries if isinstance(item, dict)]
        if isinstance(raw_entries, list)
        else []
    )
    by_id = {str(item.get("id", "")): item for item in entries}
    expected_by_id = {
        case_id: {"scope": scope, "group": group, "case": case}
        for case_id, scope, group, case in expected
    }

    def valid_placeholder_or_declaration(entry: dict[str, object]) -> bool:
        screenshot = entry.get("screenshot", {})
        receipt = entry.get("receipt", {})
        if not isinstance(screenshot, dict) or not isinstance(receipt, dict):
            return False
        binding = receipt.get("binding", {})
        if not isinstance(binding, dict) or binding.get("expected_value") != entry.get(
            "id"
        ):
            return False
        if entry.get("status") == "pending":
            return (
                screenshot.get("member") is None
                and screenshot.get("sha256") is None
                and receipt.get("member") is None
                and receipt.get("sha256") is None
                and binding.get("json_pointer") is None
            )
        has_artifacts = all(
            isinstance(value, str) and bool(value)
            for value in (
                screenshot.get("member"),
                screenshot.get("sha256"),
                receipt.get("member"),
                receipt.get("sha256"),
                binding.get("json_pointer"),
            )
        )
        return (
            entry.get("status") == "verified" and has_artifacts
        ) or (
            has_artifacts
            and exact_waived_index_entry(entry, require_artifacts=True)
        )

    statuses = [entry.get("status") for entry in entries]
    checks = {
        "named_evidence_index_is_declared_for_2_55": named_index.get("schema_version")
        == 1
        and named_index.get("target_release") == "2.55.0"
        and named_index.get("matrix") == "validation/v255_method_evidence_matrix.json"
        and named_index.get("bundle_manifest")
        == "validation/v255_evidence_bundle_manifest.json",
        "matrix_named_evidence_statuses_remain_immutable_declarations": bool(
            matrix_statuses
        )
        and all(status in {"ready", "post_candidate"} for status in matrix_statuses),
        "named_evidence_matrix_freezes_29_cross_and_26_specialized_cases": (
            sum(scope == "cross_method" for _, scope, _, _ in expected) == 29
            and sum(scope == "specialized_result" for _, scope, _, _ in expected) == 26
            and len(expected) == 55
        ),
        "named_evidence_index_has_55_unique_entries": len(entries) == len(by_id) == 55,
        "named_evidence_index_exactly_matches_matrix_cases": set(by_id)
        == set(expected_by_id)
        and all(
            all(by_id[case_id].get(field) == value for field, value in row.items())
            for case_id, row in expected_by_id.items()
        ),
        "named_evidence_index_uses_pending_verified_or_exactly_waived_rows": all(
            valid_placeholder_or_declaration(entry) for entry in entries
        ),
        "named_evidence_index_collection_status_matches_rows": (
            (
                all(status == "verified" for status in statuses)
                and named_index.get("status") == "verified"
            )
            or exact_population_status(entries, named_index.get("status"))
            or (
                any(status == "pending" for status in statuses)
                and named_index.get("status") == "pending_collection"
            )
        ),
        "named_evidence_index_declares_hash_receipt_and_safe_path_contract": (
            isinstance(named_index.get("publication_contract"), dict)
            and named_index.get("publication_contract", {}).get("hash_algorithm")
            == "sha256"
            and named_index.get("publication_contract", {}).get(
                "ordered_case_id_set_sha256"
            )
            == "98ed24bc3d4453cec21768b3c084c916c88acfb831baefbd737d01749d3e105f"
            and hashlib.sha256(
                "\n".join(case_id for case_id, _, _, _ in expected).encode("utf-8")
            ).hexdigest()
            == "98ed24bc3d4453cec21768b3c084c916c88acfb831baefbd737d01749d3e105f"
            and named_index.get("publication_contract", {}).get(
                "receipt_target_release"
            )
            == "2.55.0"
            and named_index.get("publication_contract", {}).get(
                "safe_member_paths_only"
            )
            is True
            and exact_approved_waiver_contract(
                named_index.get("collector_contract", {}).get(
                    "approved_release_waiver"
                )
            )
            and named_index.get("publication_contract", {}).get(
                "approved_waiver_count"
            )
            == 1
            and named_index.get("publication_contract", {}).get(
                "approved_waiver_case_id"
            )
            == DPI_WAIVER_CASE_ID
            and named_index.get("publication_contract", {}).get(
                "all_other_cases_must_be_verified"
            )
            is True
        ),
    }
    return checks, set(expected_by_id)


def declared_named_route_manifest_checks(
    named_index: dict[str, object],
    named_manifest: dict[str, object],
    cross_manifest: dict[str, object],
) -> dict[str, bool]:
    raw_index_entries = named_index.get("entries", [])
    index_entries = (
        [entry for entry in raw_index_entries if isinstance(entry, dict)]
        if isinstance(raw_index_entries, list)
        else []
    )
    index_by_id = {
        str(entry.get("id", "")): entry
        for entry in index_entries
        if isinstance(entry.get("id"), str) and entry.get("id")
    }
    expected_ids = set(index_by_id)

    collector = named_index.get("collector_contract", {})
    collector = collector if isinstance(collector, dict) else {}
    raw_trusted = collector.get("trusted_driver_suites", [])
    trusted_rows = (
        [row for row in raw_trusted if isinstance(row, dict)]
        if isinstance(raw_trusted, list)
        else []
    )
    trusted_by_suite = {
        str(row.get("suite_id", "")): row.get("schema_version")
        for row in trusted_rows
        if isinstance(row.get("suite_id"), str) and row.get("suite_id")
    }

    raw_cross_cases = cross_manifest.get("cases", [])
    cross_cases = (
        [case for case in raw_cross_cases if isinstance(case, dict)]
        if isinstance(raw_cross_cases, list)
        else []
    )
    cross_ids = [str(case.get("id", "")) for case in cross_cases]

    raw_named_cases = named_manifest.get("cases", [])
    named_cases = (
        [case for case in raw_named_cases if isinstance(case, dict)]
        if isinstance(raw_named_cases, list)
        else []
    )
    named_ids = [str(case.get("id", "")) for case in named_cases]
    raw_fixed_ids = named_manifest.get("supplied_by_fixed_drivers", [])
    fixed_ids = (
        [value for value in raw_fixed_ids if isinstance(value, str) and value]
        if isinstance(raw_fixed_ids, list)
        else []
    )
    raw_pending = named_manifest.get("pending_cases", [])
    pending = raw_pending if isinstance(raw_pending, list) else [None]
    pending_ids = [
        str(row.get("id", ""))
        for row in pending
        if isinstance(row, dict) and isinstance(row.get("id"), str)
    ]

    expected_fixed_ids = (
        EXPECTED_PREEXISTING_FIXED_CASE_IDS | EXPECTED_CROSS_METHOD_WRAPPER_CASE_IDS
    )
    expected_named_ids = expected_ids - expected_fixed_ids
    partition = [*fixed_ids, *named_ids, *pending_ids]
    coverage = named_manifest.get("coverage", {})
    coverage = coverage if isinstance(coverage, dict) else {}

    operation_by_group = collector.get("operation_by_group", {})
    operation_by_group = (
        operation_by_group if isinstance(operation_by_group, dict) else {}
    )
    candidate_selection = collector.get("candidate_selection", {})
    candidate_selection = (
        candidate_selection if isinstance(candidate_selection, dict) else {}
    )
    candidate_overrides = candidate_selection.get("overrides", {})
    candidate_overrides = (
        candidate_overrides if isinstance(candidate_overrides, dict) else {}
    )
    candidate_default = candidate_selection.get("default")

    used_actions: set[str] = set()
    used_queries: set[str] = set()
    referenced_named_files: list[object] = []
    named_route_hashes_valid = True
    named_routes_valid = len(named_cases) == len(raw_named_cases)

    def nonempty_string(value: object) -> bool:
        return isinstance(value, str) and bool(value.strip())

    def nonempty_unique_strings(value: object) -> bool:
        return (
            isinstance(value, list)
            and bool(value)
            and all(nonempty_string(item) for item in value)
            and len(value) == len(set(value))
        )

    def optional_string_list(value: object) -> bool:
        return value is None or (
            isinstance(value, list) and all(nonempty_string(item) for item in value)
        )

    def nonempty_string_list(value: object) -> bool:
        return (
            isinstance(value, list)
            and bool(value)
            and all(nonempty_string(item) for item in value)
        )

    def route_result_identity_valid(value: object) -> bool:
        if not isinstance(value, dict):
            return False
        cells = value.get("capability_cell_ids")
        return (
            nonempty_string(value.get("method_version"))
            and nonempty_string(value.get("primary_cell_id"))
            and nonempty_string(value.get("execution_cell_id"))
            and nonempty_unique_strings(cells)
            and value.get("primary_cell_id") in cells
            and value.get("execution_cell_id") in cells
        )

    def route_model_identity_valid(value: object) -> bool:
        if not isinstance(value, dict):
            return False
        integer_keys = (
            "ordinary_construct_count",
            "common_factor_count",
            "structural_relation_count",
        )
        return (
            all(
                isinstance(value.get(key), int)
                and not isinstance(value.get(key), bool)
                and value.get(key) >= 0
                for key in integer_keys
            )
            and isinstance(value.get("interaction_orders"), list)
            and all(
                isinstance(order, int)
                and not isinstance(order, bool)
                and order in (2, 3)
                for order in value.get("interaction_orders", [])
            )
            and isinstance(value.get("higher_order_measurement_types"), list)
            and all(
                isinstance(measurement_type, str)
                and
                measurement_type
                in (
                    "reflective_reflective",
                    "reflective_formative",
                    "formative_reflective",
                    "formative_formative",
                )
                for measurement_type in value.get(
                    "higher_order_measurement_types", []
                )
            )
        )

    def fresh_route_common_valid(route: dict[str, object]) -> bool:
        inference = route.get("inference")
        bootstrap_samples = route.get("bootstrap_samples")
        completion_timeout = route.get("completion_timeout_ms", 300_000)
        moderated_stage = route.get("moderated_stage")
        result_counts = route.get("result_counts", {})
        return (
            isinstance(route.get("fixture"), str)
            and route.get("fixture") in NAMED_SEM_FIXTURES
            and isinstance(route.get("method"), str)
            and route.get("method") in ("pls_algorithm", "pls_bootstrap", "cbsem")
            and isinstance(inference, str)
            and inference in ("point", "case_bootstrap")
            and nonempty_string(route.get("table_id"))
            and route_model_identity_valid(route.get("model"))
            and route_result_identity_valid(route.get("result"))
            and (
                bootstrap_samples is None
                or (
                    isinstance(bootstrap_samples, int)
                    and not isinstance(bootstrap_samples, bool)
                    and 500 <= bootstrap_samples <= 10_000
                )
            )
            and (
                (inference == "point" and bootstrap_samples is None)
                or (inference == "case_bootstrap" and bootstrap_samples is not None)
            )
            and (
                (route.get("method") == "pls_algorithm" and inference == "point")
                or (
                    route.get("method") == "pls_bootstrap"
                    and inference == "case_bootstrap"
                )
                or route.get("method") == "cbsem"
            )
            and isinstance(completion_timeout, int)
            and not isinstance(completion_timeout, bool)
            and 5_000 <= completion_timeout <= 300_000
            and moderated_stage in (None, "first_stage", "second_stage")
            and (
                route.get("advanced_parameter_revision") is None
                or isinstance(route.get("advanced_parameter_revision"), bool)
            )
            and isinstance(result_counts, dict)
            and all(
                (
                    isinstance(count, int)
                    and not isinstance(count, bool)
                    and count >= 0
                )
                or isinstance(count, list)
                for count in result_counts.values()
            )
            and nonempty_string_list(route.get("route_contains"))
            and optional_string_list(route.get("result_contains"))
            and nonempty_string_list(route.get("header_contains"))
            and nonempty_string_list(route.get("row_contains"))
            and optional_string_list(route.get("navigation_contains"))
        )

    for case in named_cases:
        case_id = case.get("id")
        indexed = index_by_id.get(str(case_id))
        expected_candidate = candidate_overrides.get(case_id, candidate_default)
        expected_operation = (
            operation_by_group.get(indexed.get("group"))
            if isinstance(indexed, dict)
            else None
        )
        if not (
            isinstance(case_id, str)
            and case_id in expected_named_ids
            and case.get("candidate") == expected_candidate
            and case.get("operation") == expected_operation
        ):
            named_routes_valid = False
            continue

        route = case.get("route")
        if isinstance(route, dict):
            route_kind = route.get("kind")
            if route_kind == "archive_result":
                archive = route.get("archive")
                archive_hash = route.get("archive_sha256")
                archive_identity = route.get("archive_identity")
                result_id = route.get("result_id")
                table_id = route.get("table_id")
                selected_result = route.get("selected_result_value")
                route_valid = (
                    set(route)
                    == {
                        "kind",
                        "archive",
                        "archive_sha256",
                        "result_id",
                        "selected_result_value",
                        "table_id",
                        "header_contains",
                        "row_contains",
                        "navigation_contains",
                        "result_contains",
                        "archive_identity",
                    }
                    and existing_repo_source_file(archive)
                    and is_sha256(archive_hash)
                    and sha256_path((ROOT / str(archive)).resolve()) == archive_hash
                    and nonempty_string(result_id)
                    and nonempty_string(selected_result)
                    and str(selected_result).removeprefix("canonical:") == result_id
                    and nonempty_string(table_id)
                    and isinstance(archive_identity, dict)
                    and archive_identity.get("archive_schema") in (4, 5, 6)
                    and archive_identity.get("result_id")
                    == str(result_id).removeprefix("canonical:")
                    and archive_identity.get("table_id") == table_id
                    and archive_identity.get("status") == "completed"
                    and nonempty_string(archive_identity.get("method"))
                    and nonempty_string(archive_identity.get("method_version"))
                    and nonempty_string(archive_identity.get("model_id"))
                    and (
                        (
                            isinstance(
                                archive_identity.get("table_backing_count"), int
                            )
                            and not isinstance(
                                archive_identity.get("table_backing_count"), bool
                            )
                            and archive_identity.get("table_backing_count") > 0
                        )
                        or (
                            isinstance(archive_identity.get("table_row_count"), int)
                            and archive_identity.get("table_row_count") > 0
                            and isinstance(
                                archive_identity.get("table_column_count"), int
                            )
                            and archive_identity.get("table_column_count") > 0
                        )
                    )
                    and nonempty_string_list(route.get("header_contains"))
                    and nonempty_string_list(route.get("row_contains"))
                    and nonempty_string_list(route.get("navigation_contains"))
                    and optional_string_list(route.get("result_contains"))
                )
                named_routes_valid = named_routes_valid and route_valid
                named_route_hashes_valid = named_route_hashes_valid and route_valid
                referenced_named_files.append(archive)
                used_actions.update(
                    {
                        "open_archive",
                        "inspect_archive_identity",
                        "select_result",
                        "select_result_table",
                    }
                )
                used_queries.add("specialized_result")
            elif route_kind == "fresh_cfa_bootstrap_result":
                route_valid = (
                    set(route)
                    == {
                        "kind",
                        "fixture",
                        "method",
                        "inference",
                        "bootstrap_samples",
                        "table_id",
                        "model",
                        "result",
                        "route_contains",
                        "header_contains",
                        "row_contains",
                        "minimum_rows",
                        "minimum_columns",
                    }
                    and fresh_route_common_valid(route)
                    and route.get("fixture") == "cfa"
                    and route.get("method") == "cbsem"
                    and route.get("inference") == "case_bootstrap"
                    and isinstance(route.get("minimum_rows"), int)
                    and not isinstance(route.get("minimum_rows"), bool)
                    and route.get("minimum_rows") > 0
                    and isinstance(route.get("minimum_columns"), int)
                    and not isinstance(route.get("minimum_columns"), bool)
                    and route.get("minimum_columns") > 0
                )
                named_routes_valid = named_routes_valid and route_valid
                used_actions.update(
                    {
                        "goto_packaged",
                        "create_project",
                        "load_named_sem_fixture",
                        "prepare_calculation_revision",
                        "run_calculation",
                        "select_result_table",
                    }
                )
                used_queries.add("cfa_compatibility_result")
            elif route_kind == "fresh_sem_result":
                fixture_receipt = route.get("fixture_receipt")
                required_route_keys = {
                    "kind",
                    "fixture",
                    "fixture_receipt",
                    "method",
                    "inference",
                    "table_id",
                    "model",
                    "result",
                    "route_contains",
                    "result_contains",
                    "header_contains",
                    "row_contains",
                    "navigation_contains",
                }
                allowed_route_keys = required_route_keys | {
                    "result_counts",
                    "bootstrap_samples",
                    "moderated_stage",
                    "advanced_parameter_revision",
                }
                route_valid = (
                    required_route_keys.issubset(set(route))
                    and set(route).issubset(allowed_route_keys)
                    and fresh_route_common_valid(route)
                    and isinstance(fixture_receipt, dict)
                    and all(
                        isinstance(fixture_receipt.get(key), int)
                        and not isinstance(fixture_receipt.get(key), bool)
                        and fixture_receipt.get(key) >= 0
                        for key in ("constructs", "derived_terms", "paths")
                    )
                    and not (
                        route.get("method") == "cbsem"
                        and route.get("fixture") == "cfa"
                        and route.get("inference") == "case_bootstrap"
                    )
                    and (
                        route.get("advanced_parameter_revision") is True
                    )
                    == (case_id == "specialized_result:Advanced Parameter Table revision")
                    and (
                        route.get("moderated_stage") == "first_stage"
                    )
                    == (case_id == "specialized_result:first-stage moderated mediation")
                    and (
                        route.get("moderated_stage") == "second_stage"
                    )
                    == (case_id == "specialized_result:second-stage moderated mediation")
                )
                named_routes_valid = named_routes_valid and route_valid
                used_actions.update(
                    {
                        "goto_packaged",
                        "create_project",
                        "load_named_sem_fixture",
                        "prepare_calculation_revision",
                        "run_calculation",
                        "select_result_table",
                    }
                )
                if route.get("advanced_parameter_revision") is True:
                    used_actions.update(
                        {
                            "exercise_advanced_parameter_revision",
                            "save_and_reopen_case_revision",
                        }
                    )
                used_queries.add("specialized_result")
            else:
                named_routes_valid = False
            continue

        steps = case.get("steps")
        assertion = case.get("assertion")
        query = assertion.get("query") if isinstance(assertion, dict) else None
        if not (
            isinstance(steps, list)
            and bool(steps)
            and all(isinstance(step, dict) for step in steps)
            and isinstance(assertion, dict)
            and assertion.get("id") == f"{expected_operation}:{case_id}"
            and assertion.get("expected") is not None
            and isinstance(query, dict)
            and query.get("kind") in NAMED_ROUTE_QUERY_KINDS
            and isinstance(case.get("screenshot"), dict)
        ):
            named_routes_valid = False
            continue
        used_queries.add(str(query["kind"]))
        for step in steps:
            action = step.get("action")
            if action not in NAMED_ROUTE_ACTIONS:
                named_routes_valid = False
                continue
            used_actions.add(str(action))
            if action == "assert":
                step_query = step.get("query")
                if not (
                    isinstance(step_query, dict)
                    and step_query.get("kind") in NAMED_ROUTE_QUERY_KINDS
                    and step.get("expected") is not None
                ):
                    named_routes_valid = False
                else:
                    used_queries.add(str(step_query["kind"]))
            if action in {"open_archive", "inspect_archive_identity"}:
                referenced_named_files.append(step.get("path"))
            if action == "native_file_dialog" and step.get("mode") == "open":
                referenced_named_files.append(step.get("target"))

    cross_routes_valid = len(cross_cases) == len(raw_cross_cases)
    cross_waiver_contract_valid = True
    for case in cross_cases:
        case_id = case.get("id")
        indexed = index_by_id.get(str(case_id))
        expected_operation = (
            operation_by_group.get(indexed.get("group"))
            if isinstance(indexed, dict)
            else None
        )
        if not (
            isinstance(case_id, str)
            and case_id in EXPECTED_CROSS_METHOD_WRAPPER_CASE_IDS
            and case.get("operation") == expected_operation
            and isinstance(case.get("phase"), str)
            and bool(case.get("phase"))
            and case.get("expected") is not None
        ):
            cross_routes_valid = False
        if case_id == DPI_WAIVER_CASE_ID:
            approved = case.get("approved_waiver")
            cross_waiver_contract_valid = cross_waiver_contract_valid and (
                isinstance(approved, dict)
                and {
                    "case_id": DPI_WAIVER_CASE_ID,
                    "status": "waived",
                    **approved,
                }
                == DPI_WAIVER_MANIFEST_DECLARATION
            )
        elif case.get("approved_waiver") is not None:
            cross_waiver_contract_valid = False

    fixture_sources = cross_manifest.get("fixture_sources", {})
    fixture_sources = fixture_sources if isinstance(fixture_sources, dict) else {}
    fixture_sha256 = cross_manifest.get("fixture_sha256", {})
    fixture_sha256 = fixture_sha256 if isinstance(fixture_sha256, dict) else {}
    hashed_cross_fixture_roles = {"legacy_schema4", "schema5", "schema6"}
    cross_fixture_hashes_valid = (
        set(fixture_sha256) == hashed_cross_fixture_roles
        and hashed_cross_fixture_roles.issubset(set(fixture_sources))
        and all(
            is_sha256(fixture_sha256.get(role))
            and existing_repo_source_file(fixture_sources.get(role))
            and sha256_path((ROOT / str(fixture_sources[role])).resolve())
            == fixture_sha256.get(role)
            for role in hashed_cross_fixture_roles
        )
    )
    cross_source_fields = (
        "fixture_builder",
        "driver",
        "native_dialog_helper",
        "unsaved_close_helper",
        "wrapper",
    )
    referenced_cross_files: list[object] = [
        *fixture_sources.values(),
        *(cross_manifest.get(field) for field in cross_source_fields),
    ]
    all_route_sources = [
        *NAMED_ROUTE_SUPPORT_FILES,
        *referenced_named_files,
        *referenced_cross_files,
    ]
    source_files_exist = (
        len(fixture_sources) == 4
        and all(existing_repo_source_file(value) for value in all_route_sources)
    )

    named_driver_text = (
        read_text("validation/v255_named_case_driver.mjs")
        if existing_repo_source_file("validation/v255_named_case_driver.mjs")
        else ""
    )
    cross_wrapper_text = (
        read_text("validation/run_v255_cross_method_candidate_smoke.ps1")
        if existing_repo_source_file(
            "validation/run_v255_cross_method_candidate_smoke.ps1"
        )
        else ""
    )
    installed_wrapper_text = (
        read_text("validation/run_v255_installed_portable_smoke.ps1")
        if existing_repo_source_file(
            "validation/run_v255_installed_portable_smoke.ps1"
        )
        else ""
    )
    driver_surfaces_valid = (
        'const SUITE_ID = "quickpls_v255_named_case_driver_v1"' in named_driver_text
        and 'const MANIFEST_SUITE_ID = "quickpls_v255_named_case_manifest_v1"'
        in named_driver_text
        and all(f'"{action}"' in named_driver_text for action in used_actions)
        and all(f'"{kind}"' in named_driver_text for kind in used_queries)
        and '"quickpls_v255_cross_method_candidate_wrapper_v1"'
        in cross_wrapper_text
        and "WaiveActualWindows200PercentScaling" in cross_wrapper_text
        and "Add-WaivedDpiObservation" in cross_wrapper_text
        and "WaiveActualWindows200PercentScaling" in installed_wrapper_text
    )

    return {
        "named_route_manifests_are_exact_source_authorities": (
            named_manifest.get("schema_version") == 1
            and named_manifest.get("suite_id")
            == "quickpls_v255_named_case_manifest_v1"
            and named_manifest.get("target_release") == "2.55.0"
            and named_manifest.get("status") == "ready"
            and named_manifest.get("coverage_status") == "complete"
            and cross_manifest.get("schema_version") == 1
            and cross_manifest.get("suite_id")
            == "quickpls_v255_cross_method_case_manifest_v1"
            and cross_manifest.get("target_release") == "2.55.0"
            and cross_manifest.get("candidate") == "portable"
            and cross_manifest.get("status") == "ready_for_collection"
        ),
        "named_route_partition_is_complete_before_first_diagnostic": (
            len(expected_ids) == 55
            and len(fixed_ids) == len(set(fixed_ids)) == 25
            and set(fixed_ids) == expected_fixed_ids
            and len(named_ids) == len(set(named_ids)) == 30
            and set(named_ids) == expected_named_ids
            and pending == []
            and len(partition) == len(set(partition)) == 55
            and set(partition) == expected_ids
            and coverage.get("frozen_case_count") == 55
            and coverage.get("fixed_driver_case_count") == 25
            and coverage.get("curated_case_count") == 30
            and coverage.get("pending_case_count") == 0
        ),
        "cross_method_wrapper_routes_are_exact_and_ready": (
            cross_routes_valid
            and len(cross_ids) == len(set(cross_ids)) == 17
            and set(cross_ids) == EXPECTED_CROSS_METHOD_WRAPPER_CASE_IDS
            and set(cross_ids).issubset(set(fixed_ids))
            and cross_fixture_hashes_valid
            and cross_waiver_contract_valid
        ),
        "named_executable_routes_are_schema_valid_and_supported": (
            named_routes_valid
            and named_route_hashes_valid
            and used_actions.issubset(NAMED_ROUTE_ACTIONS)
            and used_queries.issubset(NAMED_ROUTE_QUERY_KINDS)
        ),
        "named_route_trust_is_closed_and_exact": (
            len(trusted_rows) == len(trusted_by_suite)
            == len(EXPECTED_TRUSTED_DRIVER_SUITES)
            and trusted_by_suite == EXPECTED_TRUSTED_DRIVER_SUITES
            and collector.get("named_case_manifest_suite_id")
            == "quickpls_v255_named_case_manifest_v1"
            and collector.get("candidate_report_suite_id")
            == "quickpls_v255_installed_portable_smoke_v3"
        ),
        "named_route_driver_and_fixture_sources_exist": source_files_exist,
        "named_route_driver_and_fixture_sources_are_git_tracked": (
            source_files_exist and git_tracked_repo_source_files(all_route_sources)
        ),
        "named_route_driver_surfaces_implement_declared_actions_and_queries": driver_surfaces_valid,
    }


def final_named_evidence_report_checks(
    report_path: Path | None,
    installed_report_path: Path | None,
    evidence_bundle_path: Path | None,
    expected_case_ids: set[str],
    matrix_hash: str,
    named_index_hash: str,
    bundle_manifest_hash: str,
) -> tuple[dict[str, bool], dict[str, object]]:
    checks = {
        "publication_named_evidence_report_is_supplied": report_path is not None
        and report_path.is_file(),
        "publication_named_evidence_can_bind_installed_report": installed_report_path
        is not None
        and installed_report_path.is_file(),
        "publication_named_evidence_can_bind_bundle": evidence_bundle_path is not None
        and evidence_bundle_path.is_file(),
    }
    evidence: dict[str, object] = {}
    if not all(checks.values()):
        return checks, evidence
    assert report_path is not None
    assert installed_report_path is not None
    assert evidence_bundle_path is not None
    try:
        report = json.loads(report_path.read_text(encoding="utf-8"))
        installed = json.loads(installed_report_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        checks["publication_named_evidence_reports_are_parseable"] = False
        return checks, evidence
    evidence.update(
        {
            "named_evidence_report": str(report_path),
            "named_evidence_report_sha256": sha256_path(report_path),
        }
    )
    checks["publication_named_evidence_reports_are_parseable"] = isinstance(
        report, dict
    ) and isinstance(installed, dict)
    if not checks["publication_named_evidence_reports_are_parseable"]:
        return checks, evidence
    sources = report.get("sources", {})
    cases = report.get("cases", [])
    case_by_id = (
        {str(case.get("id", "")): case for case in cases if isinstance(case, dict)}
        if isinstance(cases, list)
        else {}
    )
    bundle_hash = sha256_path(evidence_bundle_path)
    checks["publication_named_evidence_verifier_passed_exact_suite"] = (
        report.get("schema_version") == 1
        and report.get("suite_id") == "quickpls_v255_named_evidence_verifier_v1"
        and report.get("target_release") == "2.55.0"
        and report.get("stage") == "publication"
        and report.get("passed") is True
        and not report.get("failures")
        and isinstance(report.get("checks"), dict)
        and bool(report.get("checks"))
        and all(value is True for value in report.get("checks", {}).values())
    )
    checks[
        "publication_named_evidence_sources_match_current_matrix_index_manifest_and_bundle"
    ] = (
        isinstance(sources, dict)
        and sources.get("matrix_sha256") == matrix_hash
        and sources.get("index_sha256") == named_index_hash
        and sources.get("bundle_manifest_sha256") == bundle_manifest_hash
        and sources.get("evidence_bundle_sha256") == bundle_hash
    )
    installed_release_waivers = installed.get("release_waivers")
    strict_qualification = (
        installed.get("qualification_status") == "passed"
        and installed_release_waivers == []
    )
    waived_qualification = (
        installed.get("qualification_status") == "passed_with_waiver"
        and isinstance(installed_release_waivers, list)
        and len(installed_release_waivers) == 1
        and exact_release_waiver_receipt(installed_release_waivers[0])
    )
    expected_waived_count = 1 if waived_qualification else 0
    expected_verified_count = 55 - expected_waived_count
    checks["publication_named_evidence_matches_exact_bound_qualification"] = (
        (strict_qualification or waived_qualification)
        and isinstance(cases, list)
        and len(cases) == len(case_by_id) == 55
        and set(case_by_id) == expected_case_ids
        and all(
            case.get("status")
            == (
                "waived"
                if expected_waived_count == 1 and case_id == DPI_WAIVER_CASE_ID
                else "passed"
            )
            and isinstance(case.get("checks"), dict)
            and bool(case.get("checks"))
            and all(value is True for value in case.get("checks", {}).values())
            for case_id, case in case_by_id.items()
        )
        and report.get("summary", {}).get("required") == 55
        and report.get("summary", {}).get("cross_method_required") == 29
        and report.get("summary", {}).get("specialized_result_required") == 26
        and report.get("summary", {}).get("verified") == expected_verified_count
        and report.get("summary", {}).get("waived") == expected_waived_count
        and report.get("summary", {}).get("pending") == 0
    )
    checks["installed_smoke_hash_binds_the_executed_named_evidence_report"] = (
        installed.get("named_evidence_stage") == "publication"
        and installed.get("named_evidence_verified") is True
        and installed.get("named_evidence_report_sha256")
        == evidence["named_evidence_report_sha256"]
        and installed.get("evidence_bundle_sha256") == bundle_hash
    )
    return checks, evidence


def catalogue_kinds() -> list[str]:
    source = read_text("src/native/nativeAnalysisCatalog.ts")
    start = source.index("const CATALOG_DRAFTS")
    end = source.index("] as const;", start)
    return re.findall(r'\bkind:\s*"([a-z0-9_]+)"', source[start:end])


def historical_failures() -> list[tuple[str, str]]:
    report = read_json(
        "validation/results/smartpls_expert_product_audit_v253_20260821/"
        "vitest_failed_contracts.json"
    )
    assert isinstance(report, dict)
    failures: list[tuple[str, str]] = []
    for suite in report.get("testResults", []):
        reported = Path(str(suite.get("name", ""))).resolve()
        try:
            file_name = reported.relative_to(ROOT.resolve()).as_posix()
        except ValueError:
            file_name = reported.name
        for assertion in suite.get("assertionResults", []):
            if assertion.get("status") == "failed":
                # The curated rebaseline stores the stable `it()` title and
                # file rather than a fragile describe-chain prefix. File plus
                # title is still required to be one-to-one across all 17.
                failures.append((file_name, str(assertion.get("title", ""))))
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--expected-product-version")
    parser.add_argument("--final-stage", action="store_true")
    parser.add_argument("--publication-stage", action="store_true")
    parser.add_argument("--vitest-report", type=Path)
    parser.add_argument("--rebaseline-report", type=Path)
    parser.add_argument("--method-publication-report", type=Path)
    parser.add_argument("--installed-portable-report", type=Path)
    parser.add_argument("--frozen-archive-reopen-report", type=Path)
    parser.add_argument("--named-evidence-report", type=Path)
    parser.add_argument("--evidence-bundle", type=Path)
    parser.add_argument("--first-consolidated-report", type=Path)
    parser.add_argument("--final-consolidated-report", type=Path)
    args = parser.parse_args()

    matrix = read_json("validation/v255_method_evidence_matrix.json")
    rebaseline = read_json("validation/v255_regression_rebaseline.json")
    frozen_archive_index = read_json("validation/v255_frozen_result_archive_index.json")
    reusable_archive_inventory = read_json(
        "validation/v255_reusable_archive_inventory.json"
    )
    evidence_bundle_manifest = read_json(
        "validation/v255_evidence_bundle_manifest.json"
    )
    named_evidence_index = read_json("validation/v255_named_evidence_index.json")
    named_route_manifest = read_json(NAMED_ROUTE_MANIFEST_PATH)
    cross_method_route_manifest = read_json(CROSS_METHOD_ROUTE_MANIFEST_PATH)
    assert isinstance(matrix, dict)
    assert isinstance(rebaseline, dict)
    assert isinstance(frozen_archive_index, dict)
    assert isinstance(reusable_archive_inventory, dict)
    assert isinstance(evidence_bundle_manifest, dict)
    assert isinstance(named_evidence_index, dict)
    assert isinstance(named_route_manifest, dict)
    assert isinstance(cross_method_route_manifest, dict)

    source_kinds = catalogue_kinds()
    matrix_methods = matrix.get("methods", [])
    matrix_kinds = [str(item.get("kind", "")) for item in matrix_methods]
    archive_methods = frozen_archive_index.get("methods", [])
    archive_kinds = [str(item.get("kind", "")) for item in archive_methods]
    historical = historical_failures()
    rebaseline_items = rebaseline.get("items", [])
    indexed_rebaseline = {
        (str(item.get("historical_file", "")), str(item.get("historical_test", "")))
        for item in rebaseline_items
    }

    expected_imports = {"CSV", "XLSX", "SPSS SAV", "ODS"}
    expected_exports = {"CSV", "XLSX", "HTML", "PDF", "SVG", "PNG"}
    cross_method = matrix.get("cross_method_journeys", {})
    package = read_json("package.json")
    assert isinstance(package, dict)
    expected_product_version = args.expected_product_version or "2.54.0"

    checks = {
        "version_authority_is_valid_for_gate_stage": str(package.get("version"))
        == expected_product_version,
        "catalogue_has_exactly_18_methods": len(source_kinds) == 18,
        "catalogue_method_ids_are_unique": len(source_kinds) == len(set(source_kinds)),
        "matrix_has_exactly_18_methods": len(matrix_kinds) == 18,
        "matrix_method_ids_are_unique": len(matrix_kinds) == len(set(matrix_kinds)),
        "matrix_exactly_matches_catalogue": matrix_kinds == source_kinds,
        "frozen_archive_index_has_exactly_18_methods": len(archive_kinds) == 18,
        "frozen_archive_index_exactly_matches_catalogue": archive_kinds == source_kinds,
        "frozen_archive_result_families_exactly_match_matrix": all(
            archive_methods[index].get("representative_results")
            == matrix_methods[index].get("result_families")
            for index in range(min(len(archive_methods), len(matrix_methods)))
        )
        and len(archive_methods) == len(matrix_methods),
        "frozen_archive_index_has_no_silent_skips": all(
            item.get("status") in {"pending", "verified", "not_applicable"}
            for item in archive_methods
        ),
        "every_method_has_setup_cases": all(
            item.get("setup_cases") for item in matrix_methods
        ),
        "every_method_has_result_families": all(
            item.get("result_families") for item in matrix_methods
        ),
        "all_four_import_families_are_required": set(cross_method.get("imports", []))
        == expected_imports,
        "all_six_export_families_are_required": set(cross_method.get("exports", []))
        == expected_exports,
        "historical_report_contains_17_failures": len(historical) == 17,
        "rebaseline_contains_17_unique_items": len(rebaseline_items) == 17
        and len(indexed_rebaseline) == 17,
        "rebaseline_covers_every_historical_failure": set(historical)
        == indexed_rebaseline,
        "rebaseline_preserves_18_method_contract": rebaseline.get("policy", {}).get(
            "preserve_public_calculate_method_count"
        )
        == 18,
        "rebaseline_requires_interaction_first_evidence": rebaseline.get(
            "policy", {}
        ).get("replacement_evidence")
        == "interaction_first",
        "frozen_result_archive_index_is_declared": (
            ROOT / "validation" / "v255_frozen_result_archive_index.json"
        ).is_file(),
        "interaction_fixture_manifest_is_declared": (
            ROOT / "validation" / "v255_interaction_fixture_manifest.json"
        ).is_file(),
        "live_packaged_lifecycle_source_is_declared": (
            ROOT / "validation" / "v255_live_calculation_lifecycle_smoke.mjs"
        ).is_file(),
        "pending_frozen_result_rows_are_release_blockers": "release blocker"
        in str(frozen_archive_index.get("purpose", "")).lower(),
        "verified_frozen_result_rows_require_hash_bound_receipts": frozen_archive_index.get(
            "verified_evidence_contract", {}
        ).get("hash_algorithm")
        == "sha256"
        and "method_kind_json_pointer"
        in frozen_archive_index.get("verified_evidence_contract", {})
        .get("receipt", {})
        .get("required", []),
        "frozen_result_archive_index_uses_release_attachable_bundle_schema": frozen_archive_index.get(
            "schema_version"
        )
        == 3
        and frozen_archive_index.get("evidence_bundle_manifest")
        == "validation/v255_evidence_bundle_manifest.json"
        and frozen_archive_index.get("reusable_source_inventory")
        == "validation/v255_reusable_archive_inventory.json"
        and (ROOT / "validation" / "v255_evidence_bundle_manifest.json").is_file(),
        "reusable_archive_inventory_covers_17_reusable_and_one_new_run": (
            len(reusable_archive_inventory.get("public_methods", [])) == 18
            and sum(
                1
                for item in reusable_archive_inventory.get("public_methods", [])
                if item.get("reuse_state") == "reusable_verified_prior_release"
            )
            == 17
            and [
                item.get("public_kind")
                for item in reusable_archive_inventory.get("public_methods", [])
                if item.get("new_scientific_run_required") is True
            ]
            == ["pls_posthoc_technical_minimum_sample_size"]
        ),
        "reusable_archive_inventory_requires_current_255_captures": all(
            item.get("current_ui_capture_required") is True
            for item in reusable_archive_inventory.get("public_methods", [])
        ),
    }
    checks.update(release_version_authority_checks(expected_product_version))
    checks.update(declared_named_evidence_checks(matrix))
    checks.update(immutable_matrix_contract_checks(matrix))
    named_index_checks, named_case_ids = declared_named_evidence_index_checks(
        matrix, named_evidence_index
    )
    checks.update(named_index_checks)
    checks.update(
        declared_named_route_manifest_checks(
            named_evidence_index,
            named_route_manifest,
            cross_method_route_manifest,
        )
    )
    if args.expected_product_version:
        checks["product_version_matches_requested_gate"] = (
            str(package.get("version")) == args.expected_product_version
        )
    if args.publication_stage:
        checks["publication_requires_explicit_2_55_version_authority"] = (
            args.expected_product_version == "2.55.0"
            and str(package.get("version")) == "2.55.0"
        )

    final_evidence: dict[str, object] = {}
    if args.final_stage or args.publication_stage:
        final_checks, final_evidence = final_rebaseline_checks(
            rebaseline, args.vitest_report, args.rebaseline_report
        )
        checks.update(final_checks)
        if args.publication_stage:
            (
                consolidated_checks,
                consolidated_evidence,
                first_consolidated,
                final_consolidated,
            ) = consolidated_diagnostic_checks(
                args.first_consolidated_report,
                args.final_consolidated_report,
                args.vitest_report,
                args.rebaseline_report,
            )
            checks.update(consolidated_checks)
            final_evidence.update(consolidated_evidence)
            lineage_checks, lineage_evidence = publication_source_lineage_checks(
                first_consolidated, final_consolidated
            )
            checks.update(lineage_checks)
            final_evidence.update(lineage_evidence)
            named_rows = named_evidence_index.get("entries", [])
            checks["publication_named_evidence_index_is_fully_populated"] = (
                isinstance(named_rows, list)
                and (
                    (
                        named_evidence_index.get("status") == "verified"
                        and len(named_rows) == 55
                        and all(
                            isinstance(row, dict)
                            and row.get("status") == "verified"
                            for row in named_rows
                        )
                    )
                    or exact_population_status(
                        named_rows, named_evidence_index.get("status")
                    )
                )
            )
            publication_checks, publication_evidence = publication_report_checks(
                args.method_publication_report,
                args.installed_portable_report,
                args.frozen_archive_reopen_report,
                args.evidence_bundle,
                evidence_bundle_manifest,
                sha256("validation/v255_method_evidence_matrix.json"),
                sha256("validation/v255_frozen_result_archive_index.json"),
                sha256("validation/v255_reusable_archive_inventory.json"),
                sha256("validation/v255_evidence_bundle_manifest.json"),
                final_evidence.get("vitest_report_sha256")
                if isinstance(final_evidence.get("vitest_report_sha256"), str)
                else None,
                final_evidence.get("final_consolidated_report_sha256")
                if isinstance(
                    final_evidence.get("final_consolidated_report_sha256"), str
                )
                else None,
                final_consolidated.get("source", {}).get("commit")
                if isinstance(final_consolidated, dict)
                and isinstance(final_consolidated.get("source"), dict)
                else None,
                lineage_evidence.get("publication_source_commit")
                if isinstance(lineage_evidence.get("publication_source_commit"), str)
                else None,
                frozen_archive_index,
                named_evidence_index,
            )
            checks.update(publication_checks)
            final_evidence.update(publication_evidence)
            named_checks, named_evidence = final_named_evidence_report_checks(
                args.named_evidence_report,
                args.installed_portable_report,
                args.evidence_bundle,
                named_case_ids,
                sha256("validation/v255_method_evidence_matrix.json"),
                sha256("validation/v255_named_evidence_index.json"),
                sha256("validation/v255_evidence_bundle_manifest.json"),
            )
            checks.update(named_checks)
            final_evidence.update(named_evidence)

    failed = [name for name, passed in checks.items() if not passed]
    payload = {
        "schema_version": 1,
        "target_release": "2.55.0",
        "passed": not failed,
        "checks": checks,
        "failed": failed,
        "catalogue_kinds": source_kinds,
        "source_sha256": {
            "catalogue": sha256("src/native/nativeAnalysisCatalog.ts"),
            "matrix": sha256("validation/v255_method_evidence_matrix.json"),
            "rebaseline": sha256("validation/v255_regression_rebaseline.json"),
            "historical_failures": sha256(
                "validation/results/smartpls_expert_product_audit_v253_20260821/"
                "vitest_failed_contracts.json"
            ),
            "frozen_archive_index": sha256(
                "validation/v255_frozen_result_archive_index.json"
            ),
            "evidence_bundle_manifest": sha256(
                "validation/v255_evidence_bundle_manifest.json"
            ),
            "named_evidence_index": sha256("validation/v255_named_evidence_index.json"),
            "named_route_manifest": sha256(NAMED_ROUTE_MANIFEST_PATH),
            "cross_method_route_manifest": sha256(
                CROSS_METHOD_ROUTE_MANIFEST_PATH
            ),
            "reusable_archive_inventory": sha256(
                "validation/v255_reusable_archive_inventory.json"
            ),
        },
        "final_stage": args.final_stage or args.publication_stage,
        "publication_stage": args.publication_stage,
        "final_evidence": final_evidence,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(payload, indent=2))
    return 0 if not failed else 1


if __name__ == "__main__":
    raise SystemExit(main())
