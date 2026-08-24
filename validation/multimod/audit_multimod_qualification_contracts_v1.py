#!/usr/bin/env python3
"""Static and frozen-candidate audit for the MultiMod campaign bindings.

This audit distinguishes reviewed executable coverage from explicit pending
targets and proves that tracked qualification templates still make no live
evidence claim. It never promotes a capability and never treats reference-only
Python tests or retained diagnostic commands as system-under-test evidence.
"""

from __future__ import annotations

import argparse
import fnmatch
import hashlib
import json
import re
import subprocess
from pathlib import Path
from typing import Any

from materialize_multimod_live_manifests_v1 import profile_specific_gates


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
PLAN = HERE / "v256_multimod_qualification_plan_v1.json"
CATALOG = HERE / "multimod_gate_bindings_v1.json"
INDEX = HERE / "multimod_capability_index_v1.json"
SHA40 = re.compile(r"^[a-f0-9]{40}$")
SHA64 = re.compile(r"^[a-f0-9]{64}$")
ALLOWED_EXECUTABLES = {"cargo", "npm.cmd", "npx.cmd", "python", "pwsh"}
SCIENTIFIC_PREFIXES = ("mga.", "fimix.", "pos.", "heterogeneity.", "conditional.", "causal.")
PENDING_BINDING_STATES = {
    "pending_full_mga_profile_matrix",
    "pending_full_mga_inference_matrix",
    "pending_multiseed_fimix_simulation",
    "pending_fimix_failure_matrix",
    "pending_production_pos_profile_matrix",
    "pending_pos_common_metric_inferential_matrix",
    "pending_full_heterogeneity_bootstrap_pipeline",
    "pending_full_conditional_profile_matrix",
    "pending_conditional_bca_studentized_simulations",
    "pending_conditional_hoc_group_weight_end_to_end",
    "pending_continuous_and_long_path_causal_simulations",
    "pending_global_metamorphic_matrix",
    "pending_qualified_semantic_export_matrix",
    "pending_real_maximum_profile_harness",
    "pending_real_packaged_workflow_matrix",
    "pending_runtime_promotion_cycle",
}


def read_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git(*arguments: str, text: bool = True) -> str | bytes:
    result = subprocess.run(
        ["git", "-C", str(ROOT), *arguments],
        check=True,
        capture_output=True,
        text=text,
    )
    return result.stdout.strip() if text else result.stdout


def relative(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def argument_values(arguments: list[str], flag: str) -> list[str]:
    return [
        arguments[index + 1]
        for index, value in enumerate(arguments[:-1])
        if value == flag
    ]


def executable_source_corpus(binding: dict[str, Any], step: dict[str, Any]) -> str:
    """Return the exact source-bound text that can emit a producer identity."""
    paths = {str(item) for item in binding.get("input_artifacts", [])}
    arguments = [str(item) for item in step.get("arguments", [])]
    executable = step.get("executable")
    if executable == "python" and arguments:
        paths.add(arguments[0])
    elif executable == "pwsh" and "-File" in arguments:
        index = arguments.index("-File") + 1
        if index < len(arguments):
            paths.add(arguments[index])
    elif executable == "npx.cmd" and "vitest" in arguments:
        paths.update(item for item in arguments if ".test." in item)
    elif executable == "cargo" and "--test" in arguments:
        index = arguments.index("--test") + 1
        if index < len(arguments):
            target = arguments[index]
            candidates = list((ROOT / "crates").glob(f"*/tests/{target}.rs"))
            candidates.extend((ROOT / "src-tauri" / "tests").glob(f"{target}.rs"))
            candidates.extend((ROOT / "tests").glob(f"{target}.rs"))
            paths.update(relative(path) for path in candidates)
    chunks: list[str] = []
    for declared in sorted(paths):
        path = ROOT / declared
        if not path.is_file():
            continue
        chunks.append(declared)
        try:
            chunks.append(path.read_text(encoding="utf-8", errors="replace"))
        except OSError:
            continue
    return "\n".join(chunks)


def scientific_identity_is_source_bound(corpus: str, identity: str) -> bool:
    if identity in corpus:
        return True
    prefix, separator, leaf = identity.rpartition(".")
    if not separator:
        return False
    # Several comparator checks are deliberately generated from a frozen case
    # label or method enum. Require both the exact emitted prefix and the exact
    # final enum token in the same source-bound producer corpus.
    if f"{prefix}." in corpus and re.search(
        rf"[\"']{re.escape(leaf)}[\"']", corpus
    ) is not None:
        return True
    parts = identity.split(".")
    return (
        len(parts) >= 3
        and f"{parts[0]}.{{" in corpus
        and f"}}.{parts[-1]}" in corpus
        and all(
            re.search(rf"[\"']{re.escape(token)}[\"']", corpus) is not None
            for token in parts[1:-1]
        )
    )


def validate_declared_step_paths(
    gate_id: str,
    step: dict[str, Any],
    label: str,
    errors: list[str],
) -> None:
    """Validate static command and test-target paths without executing a step."""
    step_id = step.get("step_id")
    prefix = f"{gate_id}/{label}/{step_id}"
    if step.get("executable") not in ALLOWED_EXECUTABLES:
        errors.append(f"{prefix}: executable is not allowlisted")
    arguments = step.get("arguments")
    if not isinstance(arguments, list):
        errors.append(f"{prefix}: arguments must be an array")
        return
    arguments = [str(item) for item in arguments]
    if not isinstance(step.get("uses_cargo"), bool):
        errors.append(f"{prefix}: uses_cargo must be explicit")
    if not isinstance(step.get("maximum_seconds"), int) or step.get("maximum_seconds", 0) <= 0:
        errors.append(f"{prefix}: maximum_seconds must be positive")
    if not isinstance(step.get("expected_outputs"), list):
        errors.append(f"{prefix}: expected_outputs must be an array")
    command_file: str | None = None
    if step.get("executable") == "python" and arguments:
        command_file = arguments[0]
    elif step.get("executable") == "pwsh" and "-File" in arguments:
        index_of_file = arguments.index("-File") + 1
        if index_of_file < len(arguments):
            command_file = arguments[index_of_file]
    if command_file and "{" not in command_file and not (ROOT / command_file).is_file():
        errors.append(f"{prefix}: executable command file is missing: {command_file}")
    if step.get("executable") == "npx.cmd" and "vitest" in arguments:
        for target in (item for item in arguments if ".test." in item):
            if not (ROOT / target).is_file():
                errors.append(f"{prefix}: Vitest target is missing: {target}")
    if step.get("executable") == "cargo" and "--test" in arguments:
        index_of_test = arguments.index("--test") + 1
        if index_of_test >= len(arguments):
            errors.append(f"{prefix}: Cargo --test has no target")
        else:
            target_name = arguments[index_of_test]
            candidates = list((ROOT / "crates").glob(f"*/tests/{target_name}.rs"))
            candidates.extend((ROOT / "src-tauri" / "tests").glob(f"{target_name}.rs"))
            candidates.extend((ROOT / "tests").glob(f"{target_name}.rs"))
            if not candidates:
                errors.append(f"{prefix}: Cargo integration-test target is missing: {target_name}")
    if (
        step.get("executable") == "cargo"
        and "test" in arguments
        and "--lib" in arguments
    ):
        lib_index = arguments.index("--lib") + 1
        test_filter = None
        if lib_index < len(arguments) and not arguments[lib_index].startswith("-"):
            test_filter = arguments[lib_index]
        if test_filter:
            package = None
            if "-p" in arguments:
                package_index = arguments.index("-p") + 1
                if package_index < len(arguments):
                    package = arguments[package_index]
            package_roots: list[Path] = []
            for manifest in [
                ROOT / "Cargo.toml",
                *(ROOT / "crates").glob("*/Cargo.toml"),
                ROOT / "src-tauri/Cargo.toml",
            ]:
                if not manifest.is_file():
                    continue
                contents = manifest.read_text(encoding="utf-8", errors="replace")
                if package and re.search(
                    rf"(?m)^name\s*=\s*[\"']{re.escape(package)}[\"']\s*$",
                    contents,
                ):
                    package_roots.append(manifest.parent)
            if package and not package_roots:
                errors.append(f"{prefix}: Cargo package is missing: {package}")
            source_corpus = "\n".join(
                source.read_text(encoding="utf-8", errors="replace")
                for package_root in package_roots
                for source in package_root.glob("src/**/*.rs")
            )
            missing_tokens = [
                token
                for token in test_filter.split("::")
                if token and token not in source_corpus
            ]
            if missing_tokens:
                errors.append(
                    f"{prefix}: Cargo lib-test filter is absent from package source: "
                    f"{test_filter} (missing {missing_tokens})"
                )


def audit(mode: str, candidate_commit: str | None) -> dict[str, Any]:
    errors: list[str] = []
    plan = read_json(PLAN)
    catalog = read_json(CATALOG)
    index = read_json(INDEX)
    plan_gates = plan.get("gates", [])
    bound_gates = catalog.get("gates", [])
    plan_by_id = {gate.get("gate_id"): gate for gate in plan_gates}
    binding_by_id = {gate.get("gate_id"): gate for gate in bound_gates}

    if len(plan_by_id) != 32 or len(plan_by_id) != len(plan_gates):
        errors.append("qualification plan must contain 32 unique gates")
    if set(plan_by_id) != set(binding_by_id) or len(binding_by_id) != len(bound_gates):
        errors.append("gate binding catalog must match the 32 unique plan gates exactly")
    if catalog.get("campaign_seed") != 42:
        errors.append("campaign seed must remain frozen at 42")
    if (
        catalog.get("binding_kind") != "reviewed_executable_coverage_v1"
        or catalog.get("placeholder_bindings_permitted") is not False
    ):
        errors.append("gate catalog must explicitly reject placeholder coverage")

    scientific_bindings_path = HERE / "scientific_slice_command_bindings_v1.json"
    scientific_bindings = read_json(scientific_bindings_path)
    if (
        scientific_bindings.get("qualification_claim") != "none"
        or scientific_bindings.get("review_status")
        != "reviewed_partial_diagnostic_programs_unexecuted"
        or scientific_bindings.get("campaign_gate_ready") is not False
    ):
        errors.append("bounded scientific slice must remain explicitly non-qualifying")
    scientific_runner_arguments = scientific_bindings.get("runner", {}).get(
        "common_arguments", []
    )
    if "-File" not in scientific_runner_arguments:
        errors.append("bounded scientific slice runner has no -File binding")
    else:
        file_index = scientific_runner_arguments.index("-File") + 1
        if file_index >= len(scientific_runner_arguments) or not (
            ROOT / scientific_runner_arguments[file_index]
        ).is_file():
            errors.append("bounded scientific slice runner path is missing")
    if {
        binding.get("gate_argument")
        for binding in scientific_bindings.get("bindings", [])
    } != {"mga", "fimix", "pos", "conditional", "causal"}:
        errors.append("bounded scientific slice gate arguments are incomplete")

    runner_promotion = (
        ROOT / "crates/qpls-runner/src/multimod_execution_v1.rs"
    ).read_text(encoding="utf-8", errors="replace")
    native_publication = (
        ROOT / "src-tauri/src/canonical_result_export_publication_v2.rs"
    ).read_text(encoding="utf-8", errors="replace")
    runner_is_protected_labs_boundary = re.search(
        r"qualification\s*:\s*MultimodQualificationStateV1::UnqualifiedLabs\s*,",
        runner_promotion,
    ) is not None
    native_authority = (
        ROOT / "src-tauri/src/multimod_candidate_authority_v1.rs"
    ).read_text(encoding="utf-8", errors="replace")
    desktop_build = (ROOT / "src-tauri/build.rs").read_text(
        encoding="utf-8", errors="replace"
    )
    promotion_mechanism_available = (
        runner_is_protected_labs_boundary
        and "promote_completed_multimod_result_v1" in native_authority
        and "required_multimod_candidate_profile_cells_v1" in native_authority
        and "verify_multimod_candidate_receipt_against_embedded_v1"
        in native_publication
        and "QPLS_MULTIMOD_BUILD_CANDIDATE_AUTHORITY_V1" in desktop_build
        and "QPLS_MULTIMOD_BUILD_PREPACKAGE_MANIFEST_SET_V1" in desktop_build
    )
    if not promotion_mechanism_available:
        for required_pending in (
            "exports.semantic.readback",
            "installed.offline.smoke",
            "portable.offline.smoke",
            "manifests.prepackage.authority",
            "manifests.live.derivation",
        ):
            if plan_by_id.get(required_pending, {}).get("implementation_status") != "pending":
                errors.append(
                    f"{required_pending}: must remain pending while runtime promotion/export is unavailable"
                )

    wrapper = "validation/multimod/invoke_multimod_gate_v1.ps1"
    executable_signatures: dict[str, list[str]] = {}
    pending_gates: list[dict[str, str]] = []
    plan_order = {gate.get("gate_id"): index for index, gate in enumerate(plan_gates)}
    for gate_id, gate in plan_by_id.items():
        invalidated = gate.get("invalidates_on_failure")
        if not isinstance(invalidated, list) or len(invalidated) != len(set(invalidated or [])):
            errors.append(f"{gate_id}: invalidated downstream gate list is missing or duplicated")
            invalidated = []
        for downstream in invalidated:
            if downstream not in plan_order:
                errors.append(f"{gate_id}: invalidation target is unknown: {downstream}")
            elif plan_order[downstream] <= plan_order[gate_id]:
                errors.append(f"{gate_id}: invalidation target is not downstream: {downstream}")
        implementation_status = gate.get("implementation_status")
        if implementation_status not in {"ready", "pending"}:
            errors.append(f"{gate_id}: implementation status is invalid")
        command = gate.get("command")
        expected_arguments = ["-NoProfile", "-File", wrapper, "-GateId", gate_id]
        if implementation_status == "ready":
            command = command or {}
            if command.get("executable") != "pwsh" or command.get("arguments") != expected_arguments:
                errors.append(f"{gate_id}: ready plan command is not the exact gate wrapper binding")
        elif command is not None:
            errors.append(f"{gate_id}: pending gate must not expose an executable plan command")

        binding = binding_by_id.get(gate_id, {})
        if binding.get("probable_root_component") != gate.get("probable_root_component"):
            errors.append(f"{gate_id}: probable root component differs between plan and binding")
        if binding.get("profiles") != gate.get("profiles"):
            errors.append(f"{gate_id}: profile list differs between plan and binding")
        if not binding.get("input_artifacts"):
            errors.append(f"{gate_id}: no bound input artifacts")
        for declared in binding.get("input_artifacts", []):
            path = (ROOT / declared).resolve()
            try:
                path.relative_to(ROOT)
            except ValueError:
                errors.append(f"{gate_id}: input escapes repository: {declared}")
                continue
            if not path.is_file():
                errors.append(f"{gate_id}: input artifact is missing: {declared}")
        steps = binding.get("steps", [])
        if implementation_status == "pending":
            if binding.get("binding_state") not in PENDING_BINDING_STATES:
                errors.append(f"{gate_id}: pending binding state is not explicit")
            if binding.get("covered_evidence_cells") != []:
                errors.append(f"{gate_id}: pending binding must claim no covered evidence cells")
            if not binding.get("target_evidence_cells"):
                errors.append(f"{gate_id}: pending binding has no target evidence declaration")
            if not isinstance(binding.get("pending_reason"), str) or not binding["pending_reason"].strip():
                errors.append(f"{gate_id}: pending binding has no concrete reason")
            if steps:
                errors.append(f"{gate_id}: pending binding must expose no executable steps")
            diagnostic_steps = binding.get("diagnostic_steps", [])
            if not isinstance(diagnostic_steps, list):
                errors.append(f"{gate_id}: diagnostic_steps must be an array when present")
                diagnostic_steps = []
            seen_diagnostics: set[str] = set()
            for diagnostic_step in diagnostic_steps:
                diagnostic_id = diagnostic_step.get("step_id")
                if (
                    not isinstance(diagnostic_id, str)
                    or not diagnostic_id
                    or diagnostic_id in seen_diagnostics
                ):
                    errors.append(f"{gate_id}: diagnostic step identity is empty or duplicated")
                seen_diagnostics.add(str(diagnostic_id))
                validate_declared_step_paths(
                    gate_id, diagnostic_step, "diagnostic", errors
                )
            pending_gates.append({"gate_id": gate_id, "reason": str(binding.get("pending_reason", ""))})
            continue
        if not binding.get("covered_evidence_cells"):
            errors.append(f"{gate_id}: ready binding has no evidence-cell declaration")
        if binding.get("binding_state", "reviewed_executable") != "reviewed_executable":
            errors.append(f"{gate_id}: ready binding state is not executable")
        if not steps:
            errors.append(f"{gate_id}: ready binding has no executable steps")
        seen_steps: set[str] = set()
        for step in steps:
            step_id = step.get("step_id")
            if not isinstance(step_id, str) or not step_id or step_id in seen_steps:
                errors.append(f"{gate_id}: step identity is empty or duplicated")
            seen_steps.add(step_id)
            if step.get("executable") not in ALLOWED_EXECUTABLES:
                errors.append(f"{gate_id}/{step_id}: executable is not allowlisted")
            if not isinstance(step.get("arguments"), list):
                errors.append(f"{gate_id}/{step_id}: arguments must be an array")
            if not isinstance(step.get("uses_cargo"), bool):
                errors.append(f"{gate_id}/{step_id}: uses_cargo must be explicit")
            if not isinstance(step.get("maximum_seconds"), int) or step["maximum_seconds"] <= 0:
                errors.append(f"{gate_id}/{step_id}: maximum_seconds must be positive")
            if not isinstance(step.get("expected_outputs"), list):
                errors.append(f"{gate_id}/{step_id}: expected_outputs must be an array")
            validate_declared_step_paths(gate_id, step, "ready", errors)
            arguments = [str(item) for item in step.get("arguments", [])]
            signature = json.dumps(
                [step.get("executable"), arguments],
                separators=(",", ":"),
            )
            executable_signatures.setdefault(signature, []).append(f"{gate_id}/{step_id}")
            executable = step.get("executable")
            command_file: str | None = None
            if executable == "python" and arguments:
                command_file = arguments[0]
            elif executable == "pwsh" and "-File" in arguments:
                index_of_file = arguments.index("-File") + 1
                if index_of_file < len(arguments):
                    command_file = arguments[index_of_file]
            if command_file and "{" not in command_file and not (ROOT / command_file).is_file():
                errors.append(f"{gate_id}/{step_id}: executable command file is missing: {command_file}")
            if executable == "npx.cmd" and "vitest" in arguments:
                for target in (item for item in arguments if ".test." in item):
                    if not (ROOT / target).is_file():
                        errors.append(f"{gate_id}/{step_id}: Vitest target is missing: {target}")
            if executable == "cargo" and "--test" in arguments:
                index_of_test = arguments.index("--test") + 1
                if index_of_test >= len(arguments):
                    errors.append(f"{gate_id}/{step_id}: Cargo --test has no target")
                else:
                    target_name = arguments[index_of_test]
                    candidates = list((ROOT / "crates").glob(f"*/tests/{target_name}.rs"))
                    candidates.extend((ROOT / "src-tauri" / "tests").glob(f"{target_name}.rs"))
                    candidates.extend((ROOT / "tests").glob(f"{target_name}.rs"))
                    if not candidates:
                        errors.append(f"{gate_id}/{step_id}: Cargo integration-test target is missing: {target_name}")
            if "run_scientific_sut_slice_v1.ps1" in arguments and "all" in arguments:
                errors.append(f"{gate_id}/{step_id}: aggregate scientific rerun is forbidden")
            invokes_dependency_verifier = any(
                item.replace("\\", "/").endswith("/verify_multimod_gate_dependency_v1.py")
                or item == "verify_multimod_gate_dependency_v1.py"
                for item in arguments
            )
            if invokes_dependency_verifier and not any(
                item in arguments
                for item in ("--required-log-contains", "--required-scientific-check")
            ):
                errors.append(f"{gate_id}/{step_id}: dependency binding names no executable evidence identity")
            if invokes_dependency_verifier:
                producer_gate_values = argument_values(arguments, "--producer-gate")
                producer_step_values = argument_values(arguments, "--producer-step")
                if len(producer_gate_values) != 1 or len(producer_step_values) != 1:
                    errors.append(f"{gate_id}/{step_id}: dependency producer identity is not exact")
                else:
                    producer_gate_id = producer_gate_values[0]
                    producer_step_id = producer_step_values[0]
                    producer_binding = binding_by_id.get(producer_gate_id, {})
                    producer_steps = [
                        item
                        for item in producer_binding.get("steps", [])
                        if item.get("step_id") == producer_step_id
                    ]
                    if len(producer_steps) != 1:
                        errors.append(
                            f"{gate_id}/{step_id}: producer step is missing or duplicated: "
                            f"{producer_gate_id}/{producer_step_id}"
                        )
                    elif plan_order.get(producer_gate_id, 10**9) >= plan_order.get(gate_id, -1):
                        errors.append(
                            f"{gate_id}/{step_id}: producer does not precede consumer: {producer_gate_id}"
                        )
                    else:
                        corpus = executable_source_corpus(producer_binding, producer_steps[0])
                        for identity in argument_values(arguments, "--required-log-contains"):
                            if identity not in corpus:
                                errors.append(
                                    f"{gate_id}/{step_id}: required producer identity is absent from "
                                    f"source-bound inputs: {identity}"
                                )
                        for identity in argument_values(arguments, "--required-scientific-check"):
                            if not scientific_identity_is_source_bound(corpus, identity):
                                errors.append(
                                    f"{gate_id}/{step_id}: required scientific identity is absent from "
                                    f"source-bound producer logic: {identity}"
                                )

        if gate_id.startswith(SCIENTIFIC_PREFIXES):
            invokes_sut = any(
                step.get("uses_cargo") is True
                or "run_scientific_sut_slice_v1.ps1" in step.get("arguments", [])
                for step in steps
            )
            if not invokes_sut:
                invokes_bound_sut = any(
                    any(
                        str(argument).replace("\\", "/").endswith(
                            "/verify_multimod_gate_dependency_v1.py"
                        )
                        or argument == "verify_multimod_gate_dependency_v1.py"
                        for argument in step.get("arguments", [])
                    )
                    for step in steps
                )
                if not invokes_bound_sut:
                    errors.append(f"{gate_id}: scientific gate has reference-only evidence")

    for signature, owners in executable_signatures.items():
        if len(owners) > 1:
            errors.append(f"duplicate executable workload binding: {', '.join(owners)}")

    manifests = []
    pending_profile_cells: list[dict[str, Any]] = []
    indexed_manifests = {family.get("manifest") for family in index.get("families", [])}
    if set(plan.get("manifests", [])) != indexed_manifests:
        errors.append("plan and capability index manifest sets differ")
    for declared in plan.get("manifests", []):
        path = ROOT / declared
        if not path.is_file():
            errors.append(f"manifest template is missing: {declared}")
            continue
        manifest = read_json(path)
        manifests.append(manifest)
        if (
            manifest.get("surface") != "labs"
            or manifest.get("declared_evidence_state") != "absent"
            or manifest.get("promotion_allowed") is not False
        ):
            errors.append(f"{declared}: tracked template must remain Labs/absent/nonpromoting")
        source = manifest.get("source_binding", {})
        if source.get("status") != "pending" or source.get("candidate_commit_sha") is not None:
            errors.append(f"{declared}: tracked source binding must remain pending")
        for artifact in source.get("source_artifacts", []):
            source_path = ROOT / artifact.get("path", "")
            if not source_path.is_file():
                errors.append(f"{declared}: source artifact is missing: {artifact.get('path')}")
            if artifact.get("sha256") is not None:
                errors.append(f"{declared}: tracked source hash must remain null")
        for profile in manifest.get("profile_matrix", []):
            profile_id = profile.get("profile_id", "")
            if (
                profile.get("surface") != "labs"
                or profile.get("coverage_state") != "absent"
                or profile.get("evidence_state") != "absent"
            ):
                errors.append(f"{profile.get('profile_id')}: tracked profile overstates evidence")
            for cell in profile.get("procedure_cells", []):
                identity = f"{profile_id}::{cell.get('procedure_id', '')}"
                if (
                    cell.get("evidence_state") != "absent"
                    or cell.get("gate_state") != "pending"
                    or cell.get("report_path") is not None
                    or cell.get("report_sha256") is not None
                ):
                    errors.append(
                        f"{identity}: tracked cell overstates evidence"
                    )
                required_profile_gates = profile_specific_gates(manifest.get("family_id", ""), profile_id)
                matching_bindings = []
                pending_target_bindings = []
                for gate_id in required_profile_gates:
                    binding = binding_by_id.get(gate_id, {})
                    profile_declared = any(
                        fnmatch.fnmatchcase(profile_id, pattern)
                        for pattern in binding.get("profiles", [])
                    )
                    cell_declared = any(
                        fnmatch.fnmatchcase(identity, pattern)
                        for pattern in binding.get("covered_evidence_cells", [])
                    )
                    if profile_declared and cell_declared:
                        matching_bindings.append(gate_id)
                    target_declared = any(
                        fnmatch.fnmatchcase(identity, pattern)
                        for pattern in binding.get("target_evidence_cells", [])
                    )
                    if (
                        plan_by_id.get(gate_id, {}).get("implementation_status") == "pending"
                        and profile_declared
                        and target_declared
                    ):
                        pending_target_bindings.append(gate_id)
                if not matching_bindings:
                    if pending_target_bindings:
                        pending_profile_cells.append(
                            {
                                "identity": identity,
                                "pending_gate_ids": pending_target_bindings,
                            }
                        )
                    else:
                        errors.append(
                            f"{identity}: neither reviewed executable coverage nor an explicit pending target binding exists"
                        )

    if mode == "frozen" and pending_gates:
        errors.append(
            "frozen campaign is blocked by pending gates: "
            + ", ".join(item["gate_id"] for item in pending_gates)
        )

    wrapper_path = ROOT / wrapper
    for required in (
        wrapper_path,
        HERE / "materialize_multimod_live_manifests_v1.py",
        HERE / "verify_multimod_release_acceptance_v1.py",
        HERE / "package_multimod_candidate_v1.ps1",
        HERE / "run_multimod_packaged_offline_smoke_v1.ps1",
        HERE / "multimod_packaged_smoke_driver_v1.mjs",
        HERE / "run_multimod_performance_profiles_v1.ps1",
        HERE / "verify_multimod_gate_dependency_v1.py",
        HERE / "scientific_slice_command_bindings_v1.json",
        HERE / "multimod_runtime_promotion_smoke_v1.schema.json",
    ):
        if not required.is_file():
            errors.append(f"required gate program is missing: {relative(required)}")

    observed_commit = str(git("rev-parse", "HEAD"))
    if mode == "frozen":
        if not candidate_commit or not SHA40.fullmatch(candidate_commit):
            errors.append("frozen audit requires an exact lowercase candidate commit")
        elif candidate_commit != observed_commit:
            errors.append("candidate commit differs from current HEAD")
        if str(git("status", "--porcelain=v1", "--untracked-files=all")):
            errors.append("frozen candidate worktree is dirty")
        versions = {
            "cargo": read_json(ROOT / "package.json").get("version"),
            "tauri": read_json(ROOT / "src-tauri/tauri.conf.json").get("version"),
        }
        cargo_version = None
        for line in (ROOT / "Cargo.toml").read_text(encoding="utf-8").splitlines():
            if line.strip().startswith("version ="):
                cargo_version = line.split("=", 1)[1].strip().strip('"')
                break
        versions["workspace"] = cargo_version
        if set(versions.values()) != {plan["candidate"]["final_version"]}:
            errors.append(f"candidate versions are not all final: {versions}")

    return {
        "schema_version": 1,
        "report_id": "qpls.multimod.qualification_contract_audit.v1",
        "mode": mode,
        "passed": not errors,
        "candidate_commit_sha": observed_commit,
        "plan_sha256": sha256(PLAN),
        "binding_sha256": sha256(CATALOG),
        "gate_count": len(plan_gates),
        "manifest_count": len(manifests),
        "ready_gate_count": len(plan_gates) - len(pending_gates),
        "pending_gate_count": len(pending_gates),
        "pending_gates": pending_gates,
        "pending_profile_cell_count": len(pending_profile_cells),
        "pending_profile_cells": pending_profile_cells,
        "tracked_templates_remain_absent": True,
        "promotion_evidence": False,
        "errors": errors,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=("development", "frozen"), default="development")
    parser.add_argument("--candidate-commit")
    parser.add_argument("--output", type=Path)
    arguments = parser.parse_args()
    try:
        report = audit(arguments.mode, arguments.candidate_commit)
    except Exception as error:  # fail closed with a structured report
        report = {
            "schema_version": 1,
            "report_id": "qpls.multimod.qualification_contract_audit.v1",
            "mode": arguments.mode,
            "passed": False,
            "promotion_evidence": False,
            "errors": [f"harness_error:{type(error).__name__}:{error}"],
        }
    payload = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if arguments.output:
        arguments.output.parent.mkdir(parents=True, exist_ok=True)
        arguments.output.write_text(payload, encoding="utf-8")
    else:
        print(payload, end="")
    return 0 if report.get("passed") is True else 1


if __name__ == "__main__":
    raise SystemExit(main())
