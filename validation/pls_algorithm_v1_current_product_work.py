#!/usr/bin/env python3
"""Generate fail-closed, non-promotional PLS current-product work evidence.

This module never invokes the customer CLI.  It builds and directly invokes one
ignored, environment-gated ``quickpls-desktop`` unit-test harness that crosses
the real Recipe-v4 compiler, qpls-runner PLS adapter, desktop canonical-result
projection, and qpls-project schema-6 atomic append/reopen boundaries.

The resulting envelope is work evidence only.  It cannot emit or attach a
QualificationSpec receipt and it cannot change Registry or manifest state.
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import subprocess
import sys
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Mapping, Sequence


VALIDATION = Path(__file__).resolve().parent
ROOT = VALIDATION.parent
sys.path.insert(0, str(VALIDATION))

import pls_algorithm_v1_current_source_oracle as oracle  # noqa: E402
import pls_algorithm_v1_qualification_factory as factory  # noqa: E402
from qualification_spec_v2 import canonical_sha256, strict_load_json  # noqa: E402


HARNESS_RELATIVE_PATH = (
    "src-tauri/src/pls_algorithm_current_product_qualification.rs"
)
HARNESS_PACKAGE = "quickpls-desktop"
HARNESS_TARGET = "quickpls_desktop_lib"
HARNESS_TEST_NAME = (
    "pls_algorithm_current_product_qualification::"
    "current_product_pls_qualification_work_harness"
)
SOURCE_SET_ENV = "QPLS_PLS_QUALIFICATION_SOURCE_SET_SHA256"
SCENARIO_SET_ENV = "QPLS_PLS_QUALIFICATION_SCENARIO_SET_SHA256"
ACTIVATION_ENV = "QPLS_INTERNAL_PLS_QUALIFICATION"
OUTPUT_ENV = "QPLS_PLS_QUALIFICATION_OUTPUT_DIR"

VARIANTS: Mapping[str, tuple[str, str]] = {
    "path_mode_a": ("path", "mode_a"),
    "path_mode_b": ("path", "mode_b"),
    "factor_mode_a": ("factor", "mode_a"),
    "pca_mode_a": ("pca", "mode_a"),
}
ORACLE_CONSTRUCT_BLOCKS: Mapping[str, tuple[str, ...]] = {
    "x": ("x1", "x2"),
    "y": ("y1", "y2"),
}
ORACLE_STRUCTURAL_PATHS: tuple[tuple[str, str], ...] = (("x", "y"),)

# Every layer needed for the claimed current-product path must participate in
# the factory's exact source-set identity before execution is permitted.
REQUIRED_PRODUCT_SEAM_SOURCE_PATHS = (
    "Cargo.lock",
    "Cargo.toml",
    "crates/qpls-core/Cargo.toml",
    "crates/qpls-core/src/analysis_recipe_v4.rs",
    "crates/qpls-core/src/canonical_result_v2.rs",
    "crates/qpls-core/src/capability_registry_v2.rs",
    "crates/qpls-core/src/compiled_pls_plan_v2.rs",
    "crates/qpls-core/src/contract.rs",
    "crates/qpls-core/src/lib.rs",
    "crates/qpls-core/src/recipe_v4_compiler.rs",
    "crates/qpls-core/src/sem_model_v4.rs",
    "crates/qpls-core/src/validation.rs",
    "crates/qpls-data/Cargo.toml",
    "crates/qpls-data/src/lib.rs",
    "crates/qpls-estimation/Cargo.toml",
    "crates/qpls-estimation/src/lib.rs",
    "crates/qpls-estimation/src/pls.rs",
    "crates/qpls-project/Cargo.toml",
    "crates/qpls-project/src/archive_integrity.rs",
    "crates/qpls-project/src/canonical_result_document_v2.rs",
    "crates/qpls-project/src/lib.rs",
    "crates/qpls-project/src/pls_score_execution_v2.rs",
    "crates/qpls-project/src/project_schema_v6.rs",
    "crates/qpls-runner/Cargo.toml",
    "crates/qpls-runner/src/lib.rs",
    "crates/qpls-runner/src/recipe_v4_pls_execution.rs",
    "src-tauri/Cargo.toml",
    "src-tauri/build.rs",
    "src-tauri/src/lib.rs",
    HARNESS_RELATIVE_PATH,
    "src-tauri/src/project_schema6_result_append.rs",
    "src-tauri/src/project_schema6_result_read.rs",
    "src-tauri/src/recipe_v4_canonical_result.rs",
    "validation/fixtures/simple_reflective.csv",
    "validation/pls_algorithm_v1_current_product_work.py",
)


def _now() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def _write_json_new(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n"
    with path.open("x", encoding="utf-8", newline="\n") as handle:
        handle.write(payload)
        handle.flush()
        os.fsync(handle.fileno())


def _descriptor(path: Path, root: Path) -> dict[str, Any]:
    resolved = path.resolve(strict=True)
    base = root.resolve(strict=True)
    try:
        relative = resolved.relative_to(base)
    except ValueError as error:
        raise ValueError(f"artifact escapes work root: {resolved}") from error
    return {
        "path": relative.as_posix(),
        "size_bytes": resolved.stat().st_size,
        "sha256": factory.sha256_file(resolved),
    }


def _run(
    command: Sequence[str],
    *,
    transcript_root: Path,
    label: str,
    extra_env: Mapping[str, str] | None = None,
    timeout: int = 1_200,
) -> tuple[subprocess.CompletedProcess[str], dict[str, Any]]:
    started_at = _now()
    started = time.monotonic()
    environment = {
        **os.environ,
        "CARGO_BUILD_JOBS": "1",
        "CARGO_INCREMENTAL": "0",
        **dict(extra_env or {}),
    }
    completed = subprocess.run(
        list(command),
        cwd=ROOT,
        env=environment,
        capture_output=True,
        text=True,
        timeout=timeout,
        check=False,
    )
    stdout_path = transcript_root / "commands" / f"{label}.stdout.txt"
    stderr_path = transcript_root / "commands" / f"{label}.stderr.txt"
    stdout_path.parent.mkdir(parents=True, exist_ok=True)
    stdout_path.write_text(completed.stdout, encoding="utf-8", newline="\n")
    stderr_path.write_text(completed.stderr, encoding="utf-8", newline="\n")
    return completed, {
        "label": label,
        "command": list(command),
        "returncode": completed.returncode,
        "started_at_utc": started_at,
        "finished_at_utc": _now(),
        "duration_seconds": round(time.monotonic() - started, 6),
        "stdout": _descriptor(stdout_path, transcript_root),
        "stderr": _descriptor(stderr_path, transcript_root),
    }


def _cargo_build_command(cargo: str) -> list[str]:
    return [
        cargo,
        "test",
        "-p",
        HARNESS_PACKAGE,
        "--lib",
        "--no-run",
        "--message-format=json",
    ]


def _harness_command(executable: Path) -> list[str]:
    return [
        str(executable),
        "--ignored",
        "--exact",
        HARNESS_TEST_NAME,
        "--nocapture",
        "--test-threads=1",
    ]


def _extract_harness_executable(cargo_stdout: str) -> Path:
    candidates: set[Path] = set()
    for raw_line in cargo_stdout.splitlines():
        try:
            row = json.loads(raw_line)
        except json.JSONDecodeError:
            continue
        if row.get("reason") != "compiler-artifact":
            continue
        target = row.get("target")
        profile = row.get("profile")
        executable = row.get("executable")
        if (
            isinstance(target, dict)
            and target.get("name") == HARNESS_TARGET
            and isinstance(profile, dict)
            and profile.get("test") is True
            and isinstance(executable, str)
            and executable
        ):
            candidates.add(Path(executable).resolve())
    if len(candidates) != 1:
        raise ValueError(
            "Cargo output must identify exactly one quickpls-desktop lib test executable"
        )
    return next(iter(candidates))


def _assert_registry_contract(registry: Mapping[str, Any]) -> dict[str, Any]:
    rows = [
        row
        for row in registry.get("capabilities", [])
        if row.get("capability_id") == factory.CAPABILITY_ID
    ]
    if len(rows) != 1:
        raise ValueError("base PLS Registry capability must exist exactly once")
    cells = [
        cell
        for cell in rows[0].get("option_cells", [])
        if cell.get("cell_id") == factory.CELL_ID
    ]
    if len(cells) != 1:
        raise ValueError("base PLS Registry option cell must exist exactly once")
    cell = cells[0]
    exact = {
        "capability_id": cell.get("capability_id"),
        "cell_id": cell.get("cell_id"),
        "capability_version": cell.get("capability_version"),
        "coverage_state": cell.get("coverage_state"),
        "evidence_state": cell.get("evidence_state"),
        "surface": cell.get("surface"),
    }
    expected = {
        "capability_id": factory.CAPABILITY_ID,
        "cell_id": factory.CELL_ID,
        "capability_version": factory.METHOD_VERSION,
        "coverage_state": "partial",
        "evidence_state": "release_qualified",
        "surface": "standard",
    }
    if exact != expected:
        raise ValueError(
            "base PLS qualification harness requires the exact scoped-Standard "
            "partial/release-qualified/Standard Registry cell"
        )
    return exact


def _assert_exact_identity(
    *, source_set_sha256: str, scenario_set_sha256: str
) -> dict[str, Any]:
    sources = factory.source_descriptors()
    current_source_hash = canonical_sha256(sources)
    if source_set_sha256 != current_source_hash:
        raise ValueError(
            "source-set SHA mismatch; regenerate the source-contract snapshot first"
        )
    observed_paths = {row["path"] for row in sources}
    missing = sorted(set(REQUIRED_PRODUCT_SEAM_SOURCE_PATHS) - observed_paths)
    if missing:
        raise ValueError(
            "current-product source contract omits required seam paths: "
            + ", ".join(missing)
        )
    spec = strict_load_json(factory.SPEC_PATH)
    current_scenario_hash = canonical_sha256(spec["scenario_contract"])
    if scenario_set_sha256 != current_scenario_hash:
        raise ValueError("scenario-set SHA mismatch")
    registry = strict_load_json(factory.REGISTRY_PATH)
    registry_cell = _assert_registry_contract(registry)
    return {
        "source_artifacts": sources,
        "source_set_sha256": current_source_hash,
        "scenario_set_sha256": current_scenario_hash,
        "spec_sha256": factory.sha256_file(factory.SPEC_PATH),
        "registry": {
            "path": factory.repository_path(factory.REGISTRY_PATH),
            "raw_sha256": factory.sha256_file(factory.REGISTRY_PATH),
            "canonical_sha256": canonical_sha256(registry),
            "registry_id": registry["registry_id"],
            "registry_version": registry["registry_version"],
            "exact_cell": registry_cell,
        },
    }


def _bind_harness_binary(executable: Path, identity: Mapping[str, Any]) -> dict[str, Any]:
    executable = executable.resolve(strict=True)
    if not executable.is_file() or executable.is_symlink():
        raise ValueError("qualification harness executable must be a regular file")
    newest_source_ns = max(
        (ROOT / row["path"]).stat().st_mtime_ns
        for row in identity["source_artifacts"]
    )
    if executable.stat().st_mtime_ns < newest_source_ns:
        raise ValueError("qualification harness binary predates a bound source")
    return {
        "path": factory.repository_path(executable),
        "sha256": factory.sha256_file(executable),
        "size_bytes": executable.stat().st_size,
        "mtime_ns": executable.stat().st_mtime_ns,
        "newest_bound_source_mtime_ns": newest_source_ns,
    }


def _read_csv(path: Path) -> tuple[list[str], list[list[float]]]:
    with path.open(newline="", encoding="utf-8-sig") as handle:
        rows = list(csv.DictReader(handle))
    if not rows:
        raise ValueError(f"empty fixture: {path}")
    variables = list(rows[0])
    return variables, [[float(row[name]) for name in variables] for row in rows]


def _oracle_values(result: Mapping[str, Any]) -> dict[tuple[str, str, str, str], float]:
    values: dict[tuple[str, str, str, str], float] = {}
    for row in result["paths"]:
        values[("path", row["source"], row["target"], "")] = float(
            row["coefficient"]
        )
    for row in result["outer_estimates"]:
        values[("loading", row["construct"], "", row["indicator"])] = float(
            row["loading"]
        )
        values[("weight", row["construct"], "", row["indicator"])] = float(
            row["weight"]
        )
    return values


def _product_values(result: Mapping[str, Any]) -> dict[tuple[str, str, str, str], float]:
    estimation = result.get("estimation")
    if not isinstance(estimation, Mapping):
        payload = result.get("payload")
        if isinstance(payload, Mapping):
            estimation = payload.get("estimation")
    if not isinstance(estimation, Mapping):
        raise ValueError("product result has no PLS estimation payload")
    return _oracle_values(estimation)


def _derive_stable_construct_identity_map(
    request: Mapping[str, Any],
    expected_constructs: Mapping[str, Sequence[str]],
    expected_paths: Sequence[tuple[str, str]],
) -> dict[str, str]:
    """Bind logical oracle blocks to exact SemModelV4 stable identities.

    Mapping is derived from measurement membership and then checked against the
    structural graph. Labels and identifier prefixes are deliberately ignored:
    neither is a scientific identity contract.
    """

    model = request.get("model")
    if not isinstance(model, Mapping):
        raise ValueError("request omitted its immutable SemModelV4 model")
    variables = model.get("variables")
    relations = model.get("relations")
    if not isinstance(variables, list) or not isinstance(relations, list):
        raise ValueError("SemModelV4 variables and relations must be arrays")

    composite_ids: list[str] = []
    observed_sources: dict[str, str] = {}
    for variable in variables:
        if not isinstance(variable, Mapping):
            raise ValueError("SemModelV4 variable is not an object")
        kind = variable.get("kind")
        stable_id = variable.get("id")
        if not isinstance(stable_id, str) or not stable_id:
            raise ValueError("SemModelV4 variable has no stable identity")
        if kind == "composite":
            composite_ids.append(stable_id)
        elif kind == "observed":
            source_column = variable.get("source_column")
            if not isinstance(source_column, str) or not source_column:
                raise ValueError("SemModelV4 observed variable has no source column")
            if stable_id in observed_sources:
                raise ValueError("SemModelV4 observed identity is duplicated")
            observed_sources[stable_id] = source_column
        else:
            raise ValueError(f"bounded PLS harness found unexpected variable kind: {kind}")
    if not composite_ids or len(composite_ids) != len(set(composite_ids)):
        raise ValueError("SemModelV4 composite identities are missing or duplicated")
    if len(observed_sources) != len(set(observed_sources.values())):
        raise ValueError("SemModelV4 observed source-column identities are non-bijective")

    stable_blocks: dict[str, list[str]] = {stable_id: [] for stable_id in composite_ids}
    measurement_pairs: set[tuple[str, str]] = set()
    structural_edges: list[tuple[str, str]] = []
    measured_observed_ids: set[str] = set()
    for relation in relations:
        if not isinstance(relation, Mapping):
            raise ValueError("SemModelV4 relation is not an object")
        kind = relation.get("kind")
        if kind in {"measurement_effect", "measurement_causal"}:
            construct_field = "construct" if kind == "measurement_effect" else "composite"
            construct = relation.get(construct_field)
            indicator = relation.get("indicator")
            if (
                not isinstance(construct, str)
                or not isinstance(indicator, str)
                or construct not in stable_blocks
                or indicator not in observed_sources
            ):
                raise ValueError("SemModelV4 measurement relation has an unknown identity")
            pair = (construct, indicator)
            if pair in measurement_pairs:
                raise ValueError("SemModelV4 measurement relation is duplicated")
            measurement_pairs.add(pair)
            measured_observed_ids.add(indicator)
            stable_blocks[construct].append(observed_sources[indicator])
        elif kind == "structural":
            source = relation.get("source")
            target = relation.get("target")
            if (
                not isinstance(source, str)
                or not isinstance(target, str)
                or source not in stable_blocks
                or target not in stable_blocks
            ):
                raise ValueError("SemModelV4 structural relation has an unknown identity")
            structural_edges.append((source, target))
        else:
            raise ValueError(f"bounded PLS harness found unexpected relation kind: {kind}")
    if measured_observed_ids != set(observed_sources):
        raise ValueError("SemModelV4 measurement relations do not exactly cover observations")

    expected_blocks: dict[str, frozenset[str]] = {}
    for logical_id, indicators in expected_constructs.items():
        if not logical_id or not indicators or len(indicators) != len(set(indicators)):
            raise ValueError("oracle construct identity or indicator block is invalid")
        expected_blocks[logical_id] = frozenset(indicators)
    stable_block_sets = {
        stable_id: frozenset(indicators)
        for stable_id, indicators in stable_blocks.items()
    }
    identity_map: dict[str, str] = {}
    for logical_id, expected_block in expected_blocks.items():
        matches = [
            stable_id
            for stable_id, stable_block in stable_block_sets.items()
            if stable_block == expected_block
            and len(stable_blocks[stable_id]) == len(expected_constructs[logical_id])
        ]
        if not matches:
            raise ValueError(
                f"SemModelV4 stable identity mapping is missing for oracle construct {logical_id}"
            )
        if len(matches) != 1:
            raise ValueError(
                f"SemModelV4 stable identity mapping is ambiguous for oracle construct {logical_id}"
            )
        identity_map[logical_id] = matches[0]
    if len(identity_map) != len(set(identity_map.values())) or set(
        identity_map.values()
    ) != set(composite_ids):
        raise ValueError("SemModelV4 construct identity mapping is non-bijective")

    if len(structural_edges) != len(set(structural_edges)):
        raise ValueError("SemModelV4 structural relation is duplicated")
    try:
        expected_stable_edges = {
            (identity_map[source], identity_map[target])
            for source, target in expected_paths
        }
    except KeyError as error:
        raise ValueError("oracle structural path uses an unknown construct") from error
    if set(structural_edges) != expected_stable_edges or len(structural_edges) != len(
        expected_paths
    ):
        raise ValueError("SemModelV4 structural graph differs from the oracle graph")
    return identity_map


def _rekey_oracle_construct_identities(
    values: Mapping[tuple[str, str, str, str], float],
    identity_map: Mapping[str, str],
) -> dict[tuple[str, str, str, str], float]:
    rekeyed: dict[tuple[str, str, str, str], float] = {}
    for (kind, source, target, indicator), value in values.items():
        if kind == "path":
            if source not in identity_map or target not in identity_map or indicator:
                raise ValueError("oracle path key cannot be bound to stable identities")
            key = (kind, identity_map[source], identity_map[target], "")
        elif kind in {"loading", "weight"}:
            if source not in identity_map or target or not indicator:
                raise ValueError("oracle outer-estimate key cannot be bound to stable identities")
            key = (kind, identity_map[source], "", indicator)
        else:
            raise ValueError(f"unexpected oracle quantity kind: {kind}")
        if key in rekeyed:
            raise ValueError("oracle stable-identity rekeying produced a duplicate quantity")
        rekeyed[key] = value
    return rekeyed


def _comparison(
    label: str,
    product: Mapping[tuple[str, str, str, str], float],
    expected: Mapping[tuple[str, str, str, str], float],
) -> dict[str, Any]:
    shared = set(product) & set(expected)
    maximum = max((abs(product[key] - expected[key]) for key in shared), default=None)
    exact = set(product) == set(expected)
    return {
        "variant": label,
        "passed": bool(exact and shared and maximum is not None and maximum <= 1.0e-6),
        "exact_key_membership": exact,
        "compared_quantity_count": len(shared),
        "max_abs_difference": maximum,
        "tolerance": 1.0e-6,
        "missing_from_product": [list(key) for key in sorted(set(expected) - set(product))],
        "unexpected_from_product": [list(key) for key in sorted(set(product) - set(expected))],
    }


def _resolve_artifact(root: Path, descriptor: Mapping[str, Any]) -> Path:
    relative = descriptor.get("path")
    if not isinstance(relative, str) or not relative:
        raise ValueError("harness artifact descriptor path is missing")
    candidate = (root / relative).resolve(strict=True)
    try:
        candidate.relative_to(root.resolve(strict=True))
    except ValueError as error:
        raise ValueError("harness artifact escapes its output root") from error
    if candidate.stat().st_size != descriptor.get("size_bytes"):
        raise ValueError(f"harness artifact size mismatch: {relative}")
    if factory.sha256_file(candidate) != descriptor.get("sha256"):
        raise ValueError(f"harness artifact digest mismatch: {relative}")
    return candidate


def _validate_harness_report(
    report: Mapping[str, Any],
    *,
    executable: Path,
    executable_descriptor: Mapping[str, Any],
    source_set_sha256: str,
    scenario_set_sha256: str,
) -> list[Mapping[str, Any]]:
    exact_header = {
        "schema_version": report.get("schema_version"),
        "report_kind": report.get("report_kind"),
        "work_evidence_only": report.get("work_evidence_only"),
        "qualification_ready": report.get("qualification_ready"),
        "promotion_authority": report.get("promotion_authority"),
        "customer_cli_invoked": report.get("customer_cli_invoked"),
        "customer_registry_admission_invoked": report.get(
            "customer_registry_admission_invoked"
        ),
    }
    expected_header = {
        "schema_version": 1,
        "report_kind": "pls_algorithm_v1_current_product_harness_v1",
        "work_evidence_only": True,
        "qualification_ready": False,
        "promotion_authority": False,
        "customer_cli_invoked": False,
        "customer_registry_admission_invoked": False,
    }
    if exact_header != expected_header:
        raise ValueError("qualification harness report has a promotional or drifted header")
    if report.get("candidate_receipt_descriptors") != []:
        raise ValueError("qualification harness must not emit candidate receipts")
    compiled = report.get("compiled_identity")
    if not isinstance(compiled, Mapping):
        raise ValueError("qualification harness omitted compiled identity")
    if compiled.get("source_set_sha256") != source_set_sha256:
        raise ValueError("qualification harness source-set identity mismatch")
    if compiled.get("scenario_set_sha256") != scenario_set_sha256:
        raise ValueError("qualification harness scenario-set identity mismatch")
    observed_executable = compiled.get("executable")
    if not isinstance(observed_executable, Mapping):
        raise ValueError("qualification harness omitted self-executable identity")
    if (
        Path(str(observed_executable.get("path"))).resolve() != executable.resolve()
        or observed_executable.get("sha256") != executable_descriptor["sha256"]
        or observed_executable.get("size_bytes") != executable_descriptor["size_bytes"]
    ):
        raise ValueError("qualification harness self-executable identity mismatch")
    if report.get("capability_cell") != {
        "registry_schema_version": 2,
        "capability_id": factory.CAPABILITY_ID,
        "cell_id": factory.CELL_ID,
        "capability_version": factory.METHOD_VERSION,
    }:
        raise ValueError("qualification harness capability-cell identity mismatch")
    archive = report.get("archive")
    if not isinstance(archive, Mapping) or not all(
        [
            archive.get("schema_version") == 6,
            archive.get("canonical_result_document_count") == len(VARIANTS),
            archive.get("resident_dataset_verified") is True,
            archive.get("models_and_recipes_verified") is True,
            archive.get("atomic_append_verified") is True,
            archive.get("final_reopen_verified") is True,
        ]
    ):
        raise ValueError("qualification harness schema-6 archive contract did not pass")
    variants = report.get("variants")
    if not isinstance(variants, list):
        raise ValueError("qualification harness variants are missing")
    by_id = {row.get("variant"): row for row in variants if isinstance(row, Mapping)}
    if set(by_id) != set(VARIANTS) or len(by_id) != len(variants):
        raise ValueError("qualification harness variant set is not exact")
    for label, row in by_id.items():
        if not all(
            [
                row.get("recipe_v4_compiled") is True,
                row.get("real_runner_executed") is True,
                row.get("product_canonical_builder_executed") is True,
                row.get("schema6_append_reopened") is True,
                row.get("method_version") == factory.METHOD_VERSION,
                isinstance(row.get("request"), Mapping),
                isinstance(row.get("execution"), Mapping),
            ]
        ):
            raise ValueError(f"qualification harness variant did not pass: {label}")
    return [by_id[label] for label in VARIANTS]


def _preserve_failed_run(
    *,
    temporary: Path,
    build_root: Path,
    executable_descriptor: Mapping[str, Any],
    diagnostics: Mapping[str, Any],
) -> Path:
    """Publish create-new, explicitly non-promotional failure diagnostics."""

    sha256 = executable_descriptor.get("sha256")
    if not isinstance(sha256, str) or len(sha256) != 64:
        raise ValueError("failed-run diagnostics require the exact harness binary identity")
    required_header = {
        "report_kind": "pls_algorithm_v1_current_product_failure_diagnostics_v1",
        "passed": False,
        "work_evidence_only": True,
        "qualification_ready": False,
        "promotion_authority": False,
        "candidate_receipt_descriptors": [],
        "customer_cli_invoked": False,
        "customer_registry_admission_invoked": False,
    }
    if any(diagnostics.get(key) != value for key, value in required_header.items()):
        raise ValueError("failed-run diagnostics header is promotional or incomplete")
    destination = build_root / f"failed_harness_{sha256}"
    if destination.exists():
        raise FileExistsError(
            "failed-run diagnostics already exist for this harness binary: "
            f"{destination.resolve()}"
        )
    _write_json_new(temporary / "failed_current_product_diagnostics.json", diagnostics)
    temporary.rename(destination)
    # TemporaryDirectory still owns the original path. Recreate it empty so
    # cleanup cannot follow the published, create-new diagnostic directory.
    temporary.mkdir()
    return destination


def write_new_snapshot(
    *, cargo: str, source_set_sha256: str, scenario_set_sha256: str
) -> dict[str, Any]:
    identity = _assert_exact_identity(
        source_set_sha256=source_set_sha256,
        scenario_set_sha256=scenario_set_sha256,
    )
    source_root = factory.RESULT_ROOT / f"source_{source_set_sha256}"
    build_root = source_root / "product_builds"
    build_root.mkdir(parents=True, exist_ok=True)
    commands: list[dict[str, Any]] = []
    compile_env = {
        SOURCE_SET_ENV: source_set_sha256,
        SCENARIO_SET_ENV: scenario_set_sha256,
    }

    with tempfile.TemporaryDirectory(prefix="pls-current-product-", dir=build_root) as name:
        temporary = Path(name)
        built, record = _run(
            _cargo_build_command(cargo),
            transcript_root=temporary,
            label="build_internal_desktop_harness",
            extra_env=compile_env,
        )
        commands.append(record)
        if built.returncode != 0:
            raise RuntimeError("current-product PLS qualification harness build failed")
        executable = _extract_harness_executable(built.stdout)
        executable_descriptor = _bind_harness_binary(executable, identity)
        final = build_root / f"harness_{executable_descriptor['sha256']}"
        if final.exists():
            raise FileExistsError(f"current-product snapshot already exists: {final}")

        harness_root = temporary / "harness"
        runtime_env = {
            **compile_env,
            ACTIVATION_ENV: "1",
            OUTPUT_ENV: str(harness_root.resolve()),
            "RUST_TEST_THREADS": "1",
        }
        executed, record = _run(
            _harness_command(executable),
            transcript_root=temporary,
            label="run_internal_desktop_harness",
            extra_env=runtime_env,
        )
        commands.append(record)
        if executed.returncode != 0:
            raise RuntimeError("current-product PLS qualification harness execution failed")
        harness_report_path = harness_root / "harness_report.json"
        harness_report = strict_load_json(harness_report_path)
        variants = _validate_harness_report(
            harness_report,
            executable=executable,
            executable_descriptor=executable_descriptor,
            source_set_sha256=source_set_sha256,
            scenario_set_sha256=scenario_set_sha256,
        )

        fixture = VALIDATION / "fixtures" / "simple_reflective.csv"
        variables, rows = _read_csv(fixture)
        comparisons = []
        for variant in variants:
            label = str(variant["variant"])
            scheme, oracle_mode = VARIANTS[label]
            request_path = _resolve_artifact(harness_root, variant["request"])
            execution_path = _resolve_artifact(harness_root, variant["execution"])
            request = strict_load_json(request_path)
            identity_map = _derive_stable_construct_identity_map(
                request,
                ORACLE_CONSTRUCT_BLOCKS,
                ORACLE_STRUCTURAL_PATHS,
            )
            product = _product_values(strict_load_json(execution_path))
            expected = oracle.estimate_pls(
                rows,
                variables,
                [
                    oracle.OracleConstruct(
                        logical_id,
                        indicators,
                        oracle_mode,
                    )
                    for logical_id, indicators in ORACLE_CONSTRUCT_BLOCKS.items()
                ],
                [
                    oracle.OraclePath(source, target)
                    for source, target in ORACLE_STRUCTURAL_PATHS
                ],
                weighting_scheme=scheme,
            )
            stable_expected = _rekey_oracle_construct_identities(
                _oracle_values(expected), identity_map
            )
            comparison = _comparison(label, product, stable_expected)
            comparison["construct_identity_map"] = identity_map
            comparisons.append(comparison)

        archive_smoke = {
            "passed": True,
            "schema_version": 6,
            "scientific_result_appended_by_real_runner": True,
            "real_product_canonical_projection": True,
            "atomic_append_and_reopen_verified": True,
            "receipt_eligible": False,
        }
        passed = all(row["passed"] for row in comparisons)
        report = {
            "schema_version": 2,
            "report_kind": "pls_algorithm_v1_current_product_work_envelope_v2",
            "qualification_id": factory.QUALIFICATION_ID,
            "capability_id": factory.CAPABILITY_ID,
            "cell_id": factory.CELL_ID,
            "method_version": factory.METHOD_VERSION,
            "generated_at_utc": _now(),
            **identity,
            "harness_binary": executable_descriptor,
            "build_fingerprint": (
                "quickpls-desktop-test-sha256:"
                f"{executable_descriptor['sha256']};source-set-sha256:{source_set_sha256}"
            ),
            "passed": passed,
            "work_evidence_only": True,
            "qualification_ready": False,
            "promotion_authority": False,
            "candidate_receipt_descriptors": [],
            "customer_cli_invoked": False,
            "customer_registry_admission_invoked": False,
            "commands": commands,
            "comparisons": comparisons,
            "archive_smoke": archive_smoke,
            "harness_report": _descriptor(harness_report_path, temporary),
            "remaining_blockers": [
                "The bounded four-variant current-product matrix is not the complete QualificationSpec scenario contract.",
                "Qualification-sized generative, adversarial, export, native packaged, performance, soak, and scientific-review roles remain absent.",
                "This work envelope has no authority to attach candidate receipts or change Registry, manifest, Standard, beta, or release state.",
            ],
        }
        if not passed:
            failure = _preserve_failed_run(
                temporary=temporary,
                build_root=build_root,
                executable_descriptor=executable_descriptor,
                diagnostics={
                    "schema_version": 1,
                    "report_kind": (
                        "pls_algorithm_v1_current_product_failure_diagnostics_v1"
                    ),
                    "failure_stage": "stable_identity_product_oracle_comparison",
                    "passed": False,
                    "work_evidence_only": True,
                    "qualification_ready": False,
                    "promotion_authority": False,
                    "candidate_receipt_descriptors": [],
                    "customer_cli_invoked": False,
                    "customer_registry_admission_invoked": False,
                    "source_set_sha256": source_set_sha256,
                    "scenario_set_sha256": scenario_set_sha256,
                    "harness_binary": executable_descriptor,
                    "commands": commands,
                    "comparisons": comparisons,
                    "harness_report": _descriptor(harness_report_path, temporary),
                },
            )
            raise RuntimeError(
                "current-product PLS oracle comparison did not pass; diagnostics preserved "
                f"at {factory.repository_path(failure)}"
            )
        _write_json_new(temporary / "current_product_work_envelope.json", report)
        Path(name).rename(final)
        # TemporaryDirectory still owns its original path. Recreate an empty
        # directory so cleanup cannot follow the renamed evidence snapshot.
        Path(name).mkdir()

    return {
        "passed": True,
        "snapshot": factory.repository_path(final),
        "work_evidence_only": True,
        "candidate_receipt_descriptors": [],
        "qualification_ready": False,
        "promotion_allowed": False,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cargo", default="cargo")
    parser.add_argument("--source-set-sha256", required=True)
    parser.add_argument("--scenario-set-sha256", required=True)
    parser.add_argument("--write-new-snapshot", action="store_true")
    args = parser.parse_args()
    if not args.write_new_snapshot:
        parser.error("--write-new-snapshot is required")
    try:
        result = write_new_snapshot(
            cargo=args.cargo,
            source_set_sha256=args.source_set_sha256,
            scenario_set_sha256=args.scenario_set_sha256,
        )
    except Exception as error:  # noqa: BLE001 - fail-closed validation boundary
        result = {
            "passed": False,
            "error_type": type(error).__name__,
            "error": str(error),
            "qualification_ready": False,
            "promotion_allowed": False,
        }
    print(json.dumps(result, indent=2, sort_keys=True, allow_nan=False))
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
