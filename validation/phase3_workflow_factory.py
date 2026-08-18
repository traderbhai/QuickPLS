#!/usr/bin/env python3
"""Current, fail-closed evidence factory for bounded Phase-3 PLS workflows.

Historical validation fixtures remain preserved as schema-v2 sources.  This
factory upgrades an in-memory copy to the explicit schema-v3 method contract,
runs the already-built CLI, and records the migration in every report.  It
never edits product sources and never builds an executable.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
import subprocess
import sys
import tempfile
import time
import zipfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable, Sequence


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
RESULTS = VALIDATION / "results"
CLI = ROOT / "target" / "debug" / "qpls.exe"
COMMON_SOURCE = "validation/phase3_workflow_factory.py"
FOCUSED_TEST = "validation/test_phase3_workflow_factory.py"

IDENTITY_VERIFICATION = {
    "kind": "identity_report",
    "identity_pointers": {
        "passed": "/passed",
        "feature_id": "/feature_id",
        "method_version": "/method_version",
        "catalogue_snapshot_date": "/catalogue_snapshot_date",
    },
    "source_artifacts_pointer": "/source_artifacts",
    "generated_at_pointer": "/generated_at_utc",
}

CONFIGS: dict[str, dict[str, Any]] = {
    "nonlinear_effects_v1": {
        "manifest": "validation/methods/nonlinear_effects_v1.manifest.json",
        "native": False,
        "simulation_wrapper": "validation/nonlinear_effects_simulation.py",
        "boundary_wrapper": "validation/nonlinear_effects_boundary_gate.py",
        "persistence_wrapper": "validation/nonlinear_effects_persistence_gate.py",
    },
    "moderated_mediation_v1": {
        "manifest": "validation/methods/moderated_mediation_v1.manifest.json",
        "native": False,
        "simulation_wrapper": "validation/moderated_mediation_simulation.py",
        "boundary_wrapper": "validation/moderated_mediation_boundary_gate.py",
        "persistence_wrapper": "validation/moderated_mediation_persistence_gate.py",
    },
    "higher_order_v1": {
        "manifest": "validation/methods/higher_order_v1.manifest.json",
        "native": True,
        "simulation_wrapper": "validation/higher_order_two_stage_simulation.py",
        "boundary_wrapper": "validation/higher_order_two_stage_boundary_gate.py",
        "persistence_wrapper": "validation/higher_order_two_stage_persistence_gate.py",
        "archive_glob": "v247-native-hoc-*.qpls",
        "archive_kind": "higher_order",
    },
    "mediation_v1": {
        "manifest": "validation/methods/mediation_v1.manifest.json",
        "evidence_generation": "evidence_truth_reconciliation_v2",
        "native": True,
        "simulation_wrapper": "validation/mediation_coverage_qualification.py",
        "boundary_wrapper": "validation/mediation_boundary_gate.py",
        "persistence_wrapper": "validation/mediation_persistence_gate.py",
        "archive_glob": "v247-native-mediation-*.qpls",
        "archive_kind": "mediation",
    },
    "moderation_v1": {
        "manifest": "validation/methods/moderation_v1.manifest.json",
        "evidence_generation": "evidence_truth_reconciliation_v2",
        "native": True,
        "simulation_wrapper": "validation/moderation_simulation.py",
        "boundary_wrapper": "validation/moderation_boundary_gate.py",
        "persistence_wrapper": "validation/moderation_persistence_gate.py",
        "archive_glob": "v247-native-moderation-*.qpls",
        "archive_kind": "moderation",
    },
}


class DuplicateKeyError(ValueError):
    pass


def _strict_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise DuplicateKeyError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def strict_load(path: Path) -> Any:
    with path.open("r", encoding="utf-8-sig") as handle:
        return json.load(
            handle,
            object_pairs_hook=_strict_pairs,
            parse_constant=lambda value: (_ for _ in ()).throw(
                ValueError(f"non-finite JSON number: {value}")
            ),
        )


def write_json(path: Path, document: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(document, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )


def sha256_bytes(content: bytes) -> str:
    return hashlib.sha256(content).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def relative(path: Path) -> str:
    return path.resolve().relative_to(ROOT.resolve()).as_posix()


def source_descriptors(paths: Iterable[str]) -> list[dict[str, Any]]:
    descriptors: list[dict[str, Any]] = []
    for item in sorted(set(paths)):
        candidate = Path(item)
        if candidate.is_absolute() or "\\" in item or ".." in candidate.parts:
            raise ValueError(f"unsafe evidence source path: {item!r}")
        path = ROOT / candidate
        if not path.is_file():
            raise FileNotFoundError(f"required Phase-3 evidence source is missing: {item}")
        descriptors.append(
            {"path": item, "size": path.stat().st_size, "sha256": sha256_file(path)}
        )
    return descriptors


def manifest(method: str) -> dict[str, Any]:
    return strict_load(ROOT / CONFIGS[method]["manifest"])


def report_root(method: str) -> Path:
    root = RESULTS / "method_factory" / method
    generation = CONFIGS[method].get("evidence_generation")
    return root / generation if isinstance(generation, str) else root


def work_root(method: str) -> Path:
    return report_root(method) / "work"


def run_command(command: Sequence[str], timeout: int = 300) -> dict[str, Any]:
    started = time.monotonic()
    completed = subprocess.run(
        list(command), cwd=ROOT, capture_output=True, text=True, timeout=timeout
    )
    return {
        "command": list(command),
        "returncode": completed.returncode,
        "duration_seconds": round(time.monotonic() - started, 3),
        "stdout_tail": completed.stdout[-4000:],
        "stderr_tail": completed.stderr[-4000:],
    }


def require_prebuilt_cli() -> dict[str, Any]:
    if not CLI.is_file():
        raise FileNotFoundError(
            "the Phase-3 factory requires target/debug/qpls.exe and does not build it"
        )
    return {"path": relative(CLI), "size": CLI.stat().st_size, "sha256": sha256_file(CLI)}


def upgrade_recipe_v3(recipe: dict[str, Any], kind: str) -> dict[str, Any]:
    upgraded = copy.deepcopy(recipe)
    upgraded["schema_version"] = 3
    model = upgraded["model"]
    model.setdefault("controls", [])
    model.setdefault("higher_order_constructs", [])
    model.setdefault("interactions", [])
    settings = upgraded["settings"]
    settings.setdefault("studentized_inner_samples", 0)
    settings.setdefault("permutation_samples", 0)
    settings.setdefault("workers", 1)
    settings.setdefault("confidence_level", 0.95)
    settings.setdefault("case_weight_column", None)
    upgraded["method_config"] = {"kind": kind}
    upgraded.setdefault("metadata", {})["validation_recipe_migration"] = (
        "schema_v2_fixture_to_explicit_schema_v3_copy"
    )
    return upgraded


def _patch_writer(module: Any, kind: str) -> None:
    original = module.write_recipe

    def current_writer(path: Path, *args: Any, **kwargs: Any) -> Any:
        result = original(path, *args, **kwargs)
        recipe = upgrade_recipe_v3(strict_load(path), kind)
        write_json(path, recipe)
        return recipe if isinstance(result, dict) else result

    module.write_recipe = current_writer


def _scientific_checks(
    report: dict[str, Any],
    ignored: set[str],
    *,
    require_report_passed: bool = False,
) -> tuple[bool, dict[str, Any]]:
    checks = report.get("checks", {})
    failures: dict[str, Any] = {}
    if require_report_passed and report.get("passed") is not True:
        failures["report_passed"] = report.get("passed")
    for key, value in checks.items():
        if key in ignored:
            continue
        if isinstance(value, bool) and not value:
            failures[key] = value
        if isinstance(value, (int, float)) and key.endswith(("_delta", "_max_delta")):
            tolerance = float(report.get("tolerance", 1e-6))
            if float(value) > tolerance:
                failures[key] = value
    return not failures, failures


def _run_nonlinear(method: str) -> dict[str, Any]:
    sys.path.insert(0, str(VALIDATION))
    import nonlinear_effects_reference as ref

    require_prebuilt_cli()
    original_recipe = ref.recipe_payload
    original_rows = ref.generated_rows
    scenarios: list[dict[str, Any]] = []
    for index, (seed, sample_size) in enumerate(((20260719, 140), (20260814, 220))):
        work = work_root(method) / f"scenario_{index:02d}"
        work.mkdir(parents=True, exist_ok=True)
        ref.RESULTS = work
        ref.DATA = work / "data.csv"
        ref.RECIPE = work / "recipe.json"
        ref.QUICKPLS = work / "quickpls.json"
        ref.OUTPUT = work / "reference_report.json"
        ref.recipe_payload = lambda fingerprint, _orig=original_recipe: upgrade_recipe_v3(
            _orig(fingerprint), "nonlinear_effects"
        )
        ref.generated_rows = lambda _seed=seed, _n=sample_size: original_rows(seed, sample_size)
        try:
            ref.main()
        except SystemExit as error:
            if error.code not in (None, 0, 1):
                raise
        report = strict_load(ref.OUTPUT)
        passed, failures = _scientific_checks(report, {"has_experimental_warning"})
        scenarios.append(
            {
                "seed": seed,
                "sample_size": sample_size,
                "passed": passed,
                "scientific_failures": failures,
                "warning_present": report["checks"].get("has_experimental_warning"),
                "report": relative(ref.OUTPUT),
                "report_sha256": sha256_file(ref.OUTPUT),
            }
        )
    return {
        "passed": all(row["passed"] for row in scenarios),
        "scenarios": scenarios,
        "scope_warning_gap": "the former generic experimental warning is absent from the engine payload",
        "claim_limited_to": "fixed-score quadratic point diagnostics",
    }


def _run_moderated_mediation(method: str) -> dict[str, Any]:
    sys.path.insert(0, str(VALIDATION))
    import moderated_mediation_reference as ref

    require_prebuilt_cli()
    original_recipe = ref.recipe_payload
    original_rows = ref.generated_rows
    scenarios: list[dict[str, Any]] = []
    for index, (seed, sample_size) in enumerate(((20260719, 128), (20260814, 192))):
        work = work_root(method) / f"scenario_{index:02d}"
        work.mkdir(parents=True, exist_ok=True)
        ref.RESULTS = work
        ref.DATA = work / "data.csv"
        ref.RECIPE = work / "recipe.json"
        ref.QUICKPLS = work / "quickpls.json"
        ref.OUTPUT = work / "reference_report.json"
        ref.GUARD_RECIPE = work / "invalid.recipe.json"
        ref.GUARD_RESULT = work / "invalid.quickpls.json"
        ref.recipe_payload = (
            lambda fingerprint, include_interaction=True, _orig=original_recipe: upgrade_recipe_v3(
                _orig(fingerprint, include_interaction), "moderated_mediation"
            )
        )
        ref.generated_rows = lambda _seed=seed, _n=sample_size: original_rows(seed, sample_size)
        try:
            ref.main()
        except SystemExit as error:
            if error.code not in (None, 0, 1):
                raise
        report = strict_load(ref.OUTPUT)
        passed, failures = _scientific_checks(report, {"has_experimental_warning"})
        scenarios.append(
            {
                "seed": seed,
                "sample_size": sample_size,
                "passed": passed,
                "scientific_failures": failures,
                "invalid_guard": report["checks"].get("invalid_guard"),
                "warning_present": report["checks"].get("has_experimental_warning"),
                "report": relative(ref.OUTPUT),
                "report_sha256": sha256_file(ref.OUTPUT),
            }
        )
    return {
        "passed": all(row["passed"] for row in scenarios),
        "scenarios": scenarios,
        "scope_warning_gap": "the former generic experimental warning is absent from the engine payload",
        "claim_limited_to": "one first- or second-stage bounded conditional indirect-effect diagnostic",
    }


def _run_higher_order(method: str) -> dict[str, Any]:
    sys.path.insert(0, str(VALIDATION))
    import higher_order_two_stage_reference as ref

    require_prebuilt_cli()
    work = work_root(method)
    work.mkdir(parents=True, exist_ok=True)
    ref.RESULTS = work
    ref.OUTPUT = work / "two_stage_reference_report.json"
    _patch_writer(ref, "pls_algorithm")
    try:
        ref.main()
    except SystemExit as error:
        if error.code not in (None, 0, 1):
            raise
    report = strict_load(ref.OUTPUT)
    passed, failures = _scientific_checks(report, set(), require_report_passed=True)
    return {
        "passed": passed,
        "raw_report_passed": report.get("passed") is True,
        "scientific_failures": failures,
        "scenario_count": len(report.get("scenarios", [])),
        "checks": report.get("checks", {}),
        "scope_warning_contract": "the point-only/no-HOC-resampling warning is present and the former generic experimental label is absent",
        "report": relative(ref.OUTPUT),
        "report_sha256": sha256_file(ref.OUTPUT),
        "claim_limited_to": "reflective-reflective disjoint two-stage HOC point estimates",
    }


def _replace_cargo_cli(args: Sequence[str]) -> list[str]:
    command = list(args)
    if command[:4] == ["cargo", "run", "-p", "qpls-cli"] and "--" in command:
        return [str(CLI), *command[command.index("--") + 1 :]]
    return command


def _run_mediation(method: str) -> dict[str, Any]:
    sys.path.insert(0, str(VALIDATION))
    import mediation_metamorphic as ref

    require_prebuilt_cli()
    work = work_root(method)
    work.mkdir(parents=True, exist_ok=True)
    ref.RESULTS = work
    ref.OUTPUT = work / "mediation_coverage_report.json"
    _patch_writer(ref, "pls_algorithm")
    original_run = ref.subprocess.run

    def current_run(args: Sequence[str], *rest: Any, **kwargs: Any) -> Any:
        return original_run(_replace_cargo_cli(args), *rest, **kwargs)

    ref.subprocess.run = current_run
    ref.main()
    report = strict_load(ref.OUTPUT)
    return {
        "passed": report.get("passed") is True,
        "scenario_count": len(report.get("scenarios", [])),
        "checks": report.get("checks", {}),
        "report": relative(ref.OUTPUT),
        "report_sha256": sha256_file(ref.OUTPUT),
        "claim_limited_to": "single-mediator direct, indirect, total, VAF, and descriptive classification",
    }


def _run_moderation(method: str) -> dict[str, Any]:
    sys.path.insert(0, str(VALIDATION))
    import moderation_reference as base
    import moderation_simulation as ref

    require_prebuilt_cli()
    work = work_root(method)
    work.mkdir(parents=True, exist_ok=True)
    base.RESULTS = work
    ref.ref.RESULTS = work
    ref.OUTPUT = work / "moderation_simulation_report.json"
    _patch_writer(base, "pls_algorithm")
    ref.main()
    report = strict_load(ref.OUTPUT)
    return {
        "passed": report.get("passed") is True,
        "signal_replicates": report.get("signal_replicates"),
        "null_replicates": report.get("null_replicates"),
        "checks": report.get("checks", {}),
        "report": relative(ref.OUTPUT),
        "report_sha256": sha256_file(ref.OUTPUT),
        "claim_limited_to": "one two-stage product-score interaction with probes at -1, 0, and +1",
    }


ENGINE_RUNNERS = {
    "nonlinear_effects_v1": _run_nonlinear,
    "moderated_mediation_v1": _run_moderated_mediation,
    "higher_order_v1": _run_higher_order,
    "mediation_v1": _run_mediation,
    "moderation_v1": _run_moderation,
}


def engine_gate(method: str) -> dict[str, Any]:
    started = datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")
    checks = ENGINE_RUNNERS[method](method)
    document = {
        "schema_version": 1,
        "kind": "quickpls_phase3_current_engine_gate",
        "method": method,
        "passed": checks["passed"],
        "started_at_utc": started,
        "completed_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "prebuilt_cli": require_prebuilt_cli(),
        "recipe_policy": "historical fixtures copied to explicit schema v3; source fixtures preserved",
        "checks": checks,
    }
    path = report_root(method) / "engine_gate.json"
    write_json(path, document)
    return {**document, "path": relative(path)}


def boundary_gate(method: str) -> dict[str, Any]:
    work = work_root(method)
    recipes = sorted(work.rglob("*.json"))
    recipe = next((path for path in recipes if path.name in {"recipe.json", "higher_order_two_stage_base.recipe.json"}), None)
    if recipe is None and method in {"mediation_v1", "moderation_v1"}:
        recipe = next((path for path in recipes if path.name.endswith("base.recipe.json")), None)
    if recipe is None:
        recipe = next(
            (
                path
                for path in recipes
                if path.name.endswith(".recipe.json")
                and path.name != "boundary_invalid.recipe.json"
            ),
            None,
        )
    checks: dict[str, Any] = {
        "prebuilt_cli_only": CLI.is_file(),
        "metamorphic_and_determinism_covered_by_engine_gate": True,
        "unsupported_scope_rejected": False,
        "dataset_identity_tamper_rejected": False,
        "no_scope_result_written": False,
        "no_tampered_result_written": False,
    }
    executions: list[dict[str, Any]] = []
    if recipe is not None:
        original = strict_load(recipe)
        invalid = copy.deepcopy(original)
        if method == "nonlinear_effects_v1":
            invalid["settings"]["weighting_scheme"] = "pca"
        elif method == "moderated_mediation_v1":
            invalid["model"]["interactions"] = []
        elif method == "higher_order_v1":
            invalid["model"]["higher_order_constructs"][0]["components"] = [
                invalid["model"]["higher_order_constructs"][0]["components"][0]
            ]
        elif method == "mediation_v1":
            constructs = [row["id"] for row in invalid["model"]["constructs"]]
            invalid["model"]["paths"].append(
                {"source": constructs[-1], "target": constructs[0]}
            )
        elif method == "moderation_v1":
            invalid["model"]["interactions"] = []
        invalid_path = work / "boundary_invalid.recipe.json"
        output_path = work / "boundary_scope_invalid.quickpls.json"
        tampered_path = work / "boundary_identity_tampered.recipe.json"
        tampered_output = work / "boundary_identity_tampered.quickpls.json"
        output_path.unlink(missing_ok=True)
        tampered_output.unlink(missing_ok=True)
        write_json(invalid_path, invalid)
        validation = run_command([str(CLI), "validate", relative(invalid_path), "--json"])
        executions.append(validation)
        data_path = recipe.with_name(recipe.name.removesuffix(".recipe.json") + ".csv")
        if not data_path.is_file():
            data_path = next(iter(sorted(recipe.parent.glob("*.csv"))), None)
        if data_path is not None:
            execution = run_command(
                [
                    str(CLI), "run", relative(invalid_path), "--data", relative(data_path),
                    "--output", relative(output_path), "--allow-experimental",
                ]
            )
            executions.append(execution)
            checks["unsupported_scope_rejected"] = execution["returncode"] != 0
            checks["no_scope_result_written"] = not output_path.exists()
            tampered = copy.deepcopy(original)
            tampered["dataset_fingerprint"] = "v2:" + "0" * 64
            write_json(tampered_path, tampered)
            tamper_execution = run_command(
                [
                    str(CLI), "run", relative(tampered_path), "--data", relative(data_path),
                    "--output", relative(tampered_output), "--allow-experimental",
                ]
            )
            executions.append(tamper_execution)
            checks["dataset_identity_tamper_rejected"] = tamper_execution["returncode"] != 0
            checks["no_tampered_result_written"] = not tampered_output.exists()
    passed = all(checks.values())
    document = {
        "schema_version": 1,
        "kind": "quickpls_phase3_boundary_gate",
        "method": method,
        "passed": passed,
        "checks": checks,
        "executions": executions,
    }
    path = report_root(method) / "boundary_gate.json"
    write_json(path, document)
    return {**document, "path": relative(path)}


def _strict_zip_json(archive: zipfile.ZipFile, member: str) -> Any:
    return json.loads(
        archive.read(member).decode("utf-8-sig"),
        object_pairs_hook=_strict_pairs,
        parse_constant=lambda value: (_ for _ in ()).throw(ValueError(value)),
    )


def verify_archive(path: Path, kind: str) -> dict[str, Any]:
    checks: dict[str, Any] = {}
    with zipfile.ZipFile(path) as archive:
        names = archive.namelist()
        checks["unique_safe_members"] = len(names) == len(set(names)) and all(
            not Path(name).is_absolute() and ".." not in Path(name).parts for name in names
        )
        manifest_doc = _strict_zip_json(archive, "manifest.json")
        project = _strict_zip_json(archive, "project.json")
        checks["checksums_exact"] = all(
            sha256_bytes(archive.read(name)) == expected
            for name, expected in manifest_doc.get("checksums", {}).items()
        )
        checks["completed_results"] = bool(project.get("results")) and all(
            row.get("status") == "completed" for row in project["results"]
        )
        if kind == "higher_order":
            checks["method_payload"] = any(
                recipe.get("model", {}).get("higher_order_constructs")
                for recipe in project.get("recipes", [])
            )
        elif kind == "mediation":
            checks["method_payload"] = any(
                row.get("payload", {}).get("estimation", {}).get("mediation", {}).get("method_version")
                == "pls_mediation_v1"
                for row in project.get("results", [])
            )
        else:
            checks["method_payload"] = any(
                row.get("payload", {}).get("estimation", {}).get("moderation", {}).get("method_version")
                == "pls_two_stage_moderation_v1"
                for row in project.get("results", [])
            )
    validation = run_command([str(CLI), "validate", relative(path), "--json"])
    checks["current_cli_accepts_archive"] = validation["returncode"] == 0
    checks["passed"] = all(checks.values())
    return {"checks": checks, "validation": validation}


def persistence_gate(method: str) -> dict[str, Any]:
    config = CONFIGS[method]
    if not config.get("native"):
        document = {
            "schema_version": 1,
            "kind": "quickpls_phase3_persistence_gate",
            "method": method,
            "passed": False,
            "blocker": "no method-specific saved native archive exists for this workflow",
        }
        path = report_root(method) / "persistence_gate.json"
        write_json(path, document)
        return {**document, "path": relative(path)}
    archives = sorted(RESULTS.glob(config["archive_glob"]), key=lambda p: p.stat().st_mtime_ns)
    if not archives:
        raise FileNotFoundError(f"no archive matches {config['archive_glob']}")
    archive_path: Path | None = None
    verified: dict[str, Any] | None = None
    rejected_candidates: list[dict[str, Any]] = []
    for candidate in reversed(archives):
        candidate_verification = verify_archive(candidate, config["archive_kind"])
        if candidate_verification["checks"]["passed"]:
            archive_path = candidate
            verified = candidate_verification
            break
        rejected_candidates.append(
            {"archive": relative(candidate), "checks": candidate_verification["checks"]}
        )
    if archive_path is None or verified is None:
        raise RuntimeError(
            f"no {config['archive_kind']} archive satisfies the frozen persistence contract"
        )
    document = {
        "schema_version": 1,
        "kind": "quickpls_phase3_persistence_gate",
        "method": method,
        "passed": verified["checks"]["passed"],
        "archive": relative(archive_path),
        "archive_size": archive_path.stat().st_size,
        "archive_sha256": sha256_file(archive_path),
        "rejected_newer_candidates": rejected_candidates,
        **verified,
    }
    path = report_root(method) / "persistence_gate.json"
    write_json(path, document)
    return {**document, "path": relative(path)}


def native_gate(method: str) -> dict[str, Any]:
    document = manifest(method)
    requirements = document["qualification"]["source_requirements"]
    tests = sorted(
        {
            path
            for role in ("frontend_report", "export_report")
            for path in requirements[role]
            if path.endswith((".test.ts", ".test.tsx"))
        }
    )
    execution = run_command(["npx.cmd", "vitest", "run", *tests], timeout=600)
    checks = {
        "all_declared_test_sources_exist": all((ROOT / path).is_file() for path in tests),
        "targeted_vitest_passed": execution["returncode"] == 0,
        "test_file_count": len(tests),
        "prebuilt_engine_identity": require_prebuilt_cli(),
    }
    passed = checks["all_declared_test_sources_exist"] and checks["targeted_vitest_passed"]
    report = {
        "schema_version": 1,
        "kind": "quickpls_phase3_native_source_gate",
        "method": method,
        "passed": passed,
        "tests": tests,
        "checks": checks,
        "execution": execution,
        "packaged_workflow_not_claimed": True,
    }
    path = report_root(method) / "native_gate.json"
    write_json(path, report)
    return {**report, "path": relative(path)}


def role_sources(method: str, roles: Iterable[str], extras: Iterable[str]) -> list[str]:
    document = manifest(method)
    governance = document["governance"]
    required = {
        governance["manifest_path"], governance["schema_path"], governance["validator_path"],
        governance["focused_test_path"], COMMON_SOURCE, FOCUSED_TEST, *extras,
    }
    for role in roles:
        required.update(document["qualification"]["source_requirements"][role])
    return sorted(required)


def write_identity(
    method: str,
    name: str,
    roles: list[str],
    checks: dict[str, Any],
    extras: Iterable[str],
) -> Path:
    document = manifest(method)
    feature = document["feature"]
    report = {
        "schema_version": 1,
        "report_kind": "quickpls_method_factory_identity_report",
        "role": "+".join(roles),
        "passed": checks.get("passed") is True,
        "feature_id": feature["id"],
        "method_version": feature["method_version"],
        "catalogue_snapshot_date": feature["catalogue_snapshot_date"],
        "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "source_artifacts": source_descriptors(role_sources(method, roles, extras)),
        "checks": checks,
    }
    path = report_root(method) / f"{name}.identity.json"
    write_json(path, report)
    return path


def run_factory(method: str) -> dict[str, Any]:
    engine = engine_gate(method)
    boundary = boundary_gate(method)
    engine_passed = engine["passed"] and boundary["passed"]
    engine_identity = write_identity(
        method,
        "engine_evidence",
        ["method_spec", "independent_reference", "simulation_report", "boundary_report"],
        {
            "passed": engine_passed,
            "engine_gate": engine,
            "boundary_gate": boundary,
        },
        [engine["path"], boundary["path"]],
    )
    outcome: dict[str, Any] = {
        "method": method,
        "engine_passed": engine_passed,
        "engine_identity": relative(engine_identity),
        "archive_passed": False,
        "native_passed": False,
    }
    if not engine_passed or not CONFIGS[method].get("native"):
        return outcome
    persistence = persistence_gate(method)
    persistence_identity = write_identity(
        method,
        "persistence_report",
        ["persistence_report"],
        persistence,
        [persistence["path"], persistence["archive"]],
    )
    outcome.update(
        archive_passed=persistence["passed"],
        persistence_identity=relative(persistence_identity),
    )
    if not persistence["passed"]:
        return outcome
    native = native_gate(method)
    native_identity = write_identity(
        method,
        "native_evidence",
        ["frontend_report", "export_report"],
        native,
        [native["path"]],
    )
    outcome.update(native_passed=native["passed"], native_identity=relative(native_identity))
    return outcome


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--method", choices=sorted(CONFIGS))
    parser.add_argument("--gate", choices=("engine", "boundary", "persistence", "native", "all"), default="all")
    args = parser.parse_args(argv)
    methods = [args.method] if args.method else list(CONFIGS)
    rows = []
    for method in methods:
        if args.gate == "all":
            rows.append(run_factory(method))
        else:
            result = {
                "engine": engine_gate,
                "boundary": boundary_gate,
                "persistence": persistence_gate,
                "native": native_gate,
            }[args.gate](method)
            rows.append({"method": method, "gate": args.gate, "passed": result["passed"]})
    print(json.dumps(rows, indent=2, sort_keys=True))
    return 0 if all(
        row.get("engine_passed", row.get("passed", False))
        and (not CONFIGS[row["method"]].get("native") or row.get("native_passed", True))
        for row in rows
    ) else 1


if __name__ == "__main__":
    raise SystemExit(main())
