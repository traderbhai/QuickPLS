#!/usr/bin/env python3
"""Generate a source-bound, fail-closed audit of the Phase-5 CB-SEM program.

This audit deliberately distinguishes a truthful support-boundary PASS from a
method-promotion PASS.  Current preview payloads, typed enum variants, and
individual fit fields are inspected but never treated as method evidence.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
CONTRACT_PATH = VALIDATION / "phase5_cbsem_expansion_contract.json"
DEFAULT_OUTPUT = VALIDATION / "results" / "phase5_cbsem_expansion"

if str(VALIDATION) not in sys.path:
    sys.path.insert(0, str(VALIDATION))

from method_promotion_manifest import validate_manifest  # noqa: E402


PROGRAM_ID = "qpls3.program.phase5_cbsem_expansion"
PROGRAM_VERSION = "phase5_cbsem_expansion_v1"
CATALOGUE_DATE = "2026-08-12"
EXPECTED_TRACK_IDS = (
    "cbsem_ml_baseline",
    "cbsem_bootstrap",
    "cbsem_measurement_invariance",
    "cbsem_multigroup",
    "cbsem_model_comparison",
    "cbsem_moderator",
    "cbsem_robust_ml",
    "cbsem_ordinal_wlsmv",
    "cbsem_fiml",
    "cbsem_equality_constraints",
)
ALLOWED_CURRENT_STATES = frozenset(
    {"release_qualified", "native_qualified", "absent"}
)
COMMON_SOURCES = (
    "validation/phase5_cbsem_expansion_contract.json",
    "validation/phase5_cbsem_expansion_audit.py",
    "validation/test_phase5_cbsem_expansion_audit.py",
    "docs/PHASE5_CBSEM_EXPANSION_STATUS.md",
    "validation/method_promotion_manifest.py",
    "validation/methods/method_promotion_manifest.schema.json",
    "validation/test_method_promotion_manifest.py",
)


class DuplicateKeyError(ValueError):
    """Raised when JSON contains an ambiguous duplicate object key."""


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


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def repository_path(path: Path, root: Path = ROOT) -> str:
    return path.resolve().relative_to(root.resolve()).as_posix()


def source_descriptors(root: Path, paths: Iterable[str]) -> list[dict[str, Any]]:
    descriptors: list[dict[str, Any]] = []
    for relative in sorted(set(paths)):
        candidate = Path(relative)
        if candidate.is_absolute() or "\\" in relative or ".." in candidate.parts:
            raise ValueError(f"unsafe Phase-5 source path: {relative!r}")
        path = root / candidate
        if not path.is_file():
            raise FileNotFoundError(f"required Phase-5 source is missing: {relative}")
        descriptors.append(
            {
                "path": relative,
                "size": path.stat().st_size,
                "sha256": sha256_file(path),
            }
        )
    return descriptors


def _cycle_errors(tracks: list[dict[str, Any]]) -> list[str]:
    dependencies = {track["id"]: tuple(track["dependencies"]) for track in tracks}
    visiting: set[str] = set()
    visited: set[str] = set()
    errors: list[str] = []

    def visit(track_id: str, chain: tuple[str, ...]) -> None:
        if track_id in visiting:
            errors.append("dependency cycle: " + " -> ".join((*chain, track_id)))
            return
        if track_id in visited:
            return
        visiting.add(track_id)
        for dependency in dependencies.get(track_id, ()):
            visit(dependency, (*chain, track_id))
        visiting.remove(track_id)
        visited.add(track_id)

    for track_id in dependencies:
        visit(track_id, ())
    return errors


def validate_contract(document: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    if document.get("schema_version") != 1:
        errors.append("schema_version must equal 1")
    if document.get("program_id") != PROGRAM_ID:
        errors.append(f"program_id must equal {PROGRAM_ID}")
    if document.get("program_version") != PROGRAM_VERSION:
        errors.append(f"program_version must equal {PROGRAM_VERSION}")
    if document.get("catalogue_snapshot_date") != CATALOGUE_DATE:
        errors.append(f"catalogue_snapshot_date must equal {CATALOGUE_DATE}")

    tracks = document.get("tracks")
    if not isinstance(tracks, list):
        return [*errors, "tracks must be an array"]
    ids = [track.get("id") for track in tracks if isinstance(track, dict)]
    if tuple(ids) != EXPECTED_TRACK_IDS:
        errors.append(
            "track inventory/order must equal the closed Phase-5 inventory: "
            + ", ".join(EXPECTED_TRACK_IDS)
        )
    feature_ids = [track.get("feature_id") for track in tracks if isinstance(track, dict)]
    method_versions = [track.get("method_version") for track in tracks if isinstance(track, dict)]
    for label, values in (("feature_id", feature_ids), ("method_version", method_versions)):
        duplicates = sorted(value for value, count in Counter(values).items() if count > 1)
        if duplicates:
            errors.append(f"duplicate {label} values: {', '.join(duplicates)}")

    id_set = set(ids)
    for track in tracks:
        if not isinstance(track, dict):
            errors.append("every track must be an object")
            continue
        track_id = track.get("id", "<missing>")
        if track.get("current_state") not in ALLOWED_CURRENT_STATES:
            errors.append(f"{track_id}: unsupported current_state")
        candidate_implementation = track.get("candidate_implementation", False)
        if not isinstance(candidate_implementation, bool):
            errors.append(f"{track_id}: candidate_implementation must be boolean when present")
        elif candidate_implementation and track.get("current_state") != "absent":
            errors.append(f"{track_id}: a candidate implementation must remain absent until evidence promotes it")
        if track.get("target_state") != "release_qualified":
            errors.append(f"{track_id}: target_state must be release_qualified")
        dependencies = track.get("dependencies")
        if not isinstance(dependencies, list) or len(dependencies) != len(set(dependencies)):
            errors.append(f"{track_id}: dependencies must be a unique array")
        elif track_id in dependencies or any(item not in id_set for item in dependencies):
            errors.append(f"{track_id}: dependency is self-referential or unknown")
        probes = track.get("source_probes")
        if not isinstance(probes, list) or not probes:
            errors.append(f"{track_id}: at least one source probe is required")
        elif any(
            not isinstance(probe, dict)
            or not isinstance(probe.get("path"), str)
            or not isinstance(probe.get("contains"), str)
            or not probe["contains"]
            for probe in probes
        ):
            errors.append(f"{track_id}: every source probe needs path and contains")
        blockers = track.get("blockers")
        if not isinstance(blockers, list) or not blockers or any(not item for item in blockers):
            errors.append(f"{track_id}: exact blockers are required")
        previews = track.get("ineligible_preview_versions")
        if not isinstance(previews, list) or len(previews) != len(set(previews)):
            errors.append(f"{track_id}: ineligible_preview_versions must be unique")

    completion = document.get("completion_rule", {})
    if completion.get("required_track_state") != "release_qualified":
        errors.append("completion_rule must require release_qualified")
    if completion.get("preview_relabeling_forbidden") is not True:
        errors.append("completion_rule must forbid preview relabeling")
    if tuple(completion.get("required_tracks", ())) != EXPECTED_TRACK_IDS:
        errors.append("completion_rule required_tracks must equal the closed inventory")
    errors.extend(_cycle_errors(tracks))
    return errors


def _manifest_sources(document: dict[str, Any]) -> list[str]:
    paths: set[str] = set()
    for stage in document.get("qualification", {}).get("evidence", {}).values():
        for evidence in stage:
            path = evidence.get("path")
            if isinstance(path, str):
                paths.add(path)
    return sorted(paths)


def evaluate_track(
    track: dict[str, Any],
    *,
    root: Path,
    contract_errors: list[str],
) -> dict[str, Any]:
    probe_rows: list[dict[str, Any]] = []
    probed_paths: set[str] = set()
    source_text: dict[str, str] = {}
    for probe in track["source_probes"]:
        relative = probe["path"]
        probed_paths.add(relative)
        path = root / relative
        try:
            text = source_text.setdefault(relative, path.read_text(encoding="utf-8-sig"))
            found = probe["contains"] in text
            error = None
        except (OSError, UnicodeError) as exception:
            found = False
            error = f"{type(exception).__name__}: {exception}"
        probe_rows.append(
            {
                "path": relative,
                "needle": probe["contains"],
                "found": found,
                "error": error,
            }
        )

    engine_paths = {
        "crates/qpls-estimation/src/pls.rs",
        "crates/qpls-core/src/contract.rs",
        "crates/qpls-core/src/validation.rs",
    }
    forbidden_rows: list[dict[str, Any]] = []
    for marker in track.get("forbidden_engine_markers", []):
        locations: list[str] = []
        for relative in sorted(engine_paths):
            path = root / relative
            try:
                text = source_text.setdefault(relative, path.read_text(encoding="utf-8-sig"))
            except (OSError, UnicodeError):
                continue
            if marker in text:
                locations.append(relative)
        forbidden_rows.append(
            {"marker": marker, "absent": not locations, "locations": locations}
        )
        probed_paths.update(engine_paths)

    manifest_result: dict[str, Any] | None = None
    manifest_document: dict[str, Any] | None = None
    unexpected_manifests: list[str] = []
    manifest_relative = track.get("promotion_manifest")
    if isinstance(manifest_relative, str):
        manifest_path = root / manifest_relative
        manifest_result = validate_manifest(manifest_path, root)
        if manifest_path.is_file():
            manifest_document = strict_load(manifest_path)
    else:
        for candidate in sorted((root / "validation/methods").glob("*.manifest.json")):
            try:
                candidate_document = strict_load(candidate)
            except (OSError, UnicodeError, json.JSONDecodeError, DuplicateKeyError, ValueError):
                continue
            if candidate_document.get("feature", {}).get("id") == track["feature_id"]:
                unexpected_manifests.append(repository_path(candidate, root))

    derived_state = (
        manifest_result.get("derived_state")
        if manifest_result is not None
        else "absent"
    )
    state_matches = derived_state == track["current_state"]
    identity_matches = True
    if manifest_result is not None:
        identity_matches = (
            manifest_result.get("feature_id") == track["feature_id"]
            and manifest_result.get("method_version") == track["method_version"]
            and manifest_result.get("catalogue_snapshot_date") == CATALOGUE_DATE
        )
    probes_pass = all(row["found"] and row["error"] is None for row in probe_rows)
    forbidden_pass = all(row["absent"] for row in forbidden_rows)
    manifest_pass = (
        (manifest_result is None or manifest_result.get("passed") is True)
        and not unexpected_manifests
    )
    passed = (
        not contract_errors
        and probes_pass
        and forbidden_pass
        and manifest_pass
        and identity_matches
        and state_matches
    )

    sources = set(COMMON_SOURCES)
    sources.update(probed_paths)
    if manifest_relative:
        sources.add(manifest_relative)
    if manifest_document is not None:
        sources.update(_manifest_sources(manifest_document))
    source_error: str | None = None
    try:
        descriptors = source_descriptors(root, sources)
    except (OSError, ValueError) as exception:
        descriptors = []
        source_error = f"{type(exception).__name__}: {exception}"
        passed = False

    if track["current_state"] == "release_qualified":
        classification = "bounded_release_foundation"
    elif track["current_state"] == "native_qualified":
        classification = "bounded_native_foundation"
    elif track.get("candidate_implementation") is True:
        classification = "implemented_candidate_unqualified"
    elif track["ineligible_preview_versions"]:
        classification = "ineligible_preview_only"
    else:
        classification = "absent_contract_scaffold_only"

    return {
        "schema_version": 1,
        "report_kind": "quickpls_phase5_support_boundary_report",
        "passed": passed,
        "feature_id": track["feature_id"],
        "method_version": track["method_version"],
        "catalogue_snapshot_date": CATALOGUE_DATE,
        "track_id": track["id"],
        "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "support_classification": classification,
        "declared_current_state": track["current_state"],
        "derived_current_state": derived_state,
        "target_state": track["target_state"],
        "phase5_track_complete": derived_state == "release_qualified",
        "ineligible_preview_versions": track["ineligible_preview_versions"],
        "dependencies": track["dependencies"],
        "checks": {
            "contract_valid": not contract_errors,
            "source_probes_present": probes_pass,
            "forbidden_engine_markers_absent": forbidden_pass,
            "promotion_manifest_valid_or_explicitly_uncreated": manifest_pass,
            "untracked_promotion_manifests_absent": not unexpected_manifests,
            "promotion_identity_matches": identity_matches,
            "derived_state_matches_contract": state_matches,
            "preview_relabeling_forbidden": True,
            "source_binding_complete": source_error is None,
        },
        "source_probes": probe_rows,
        "forbidden_engine_markers": forbidden_rows,
        "promotion_manifest_result": manifest_result,
        "unexpected_promotion_manifests": unexpected_manifests,
        "blockers": track["blockers"],
        "source_binding_error": source_error,
        "source_artifacts": descriptors,
    }


def audit(
    *,
    root: Path = ROOT,
    contract_path: Path | None = None,
) -> dict[str, Any]:
    selected_contract = contract_path or (root / "validation/phase5_cbsem_expansion_contract.json")
    try:
        contract = strict_load(selected_contract)
    except (OSError, UnicodeError, json.JSONDecodeError, DuplicateKeyError, ValueError) as error:
        return {
            "schema_version": 1,
            "report_kind": "quickpls_phase5_cbsem_expansion_audit",
            "passed": False,
            "phase5_complete": False,
            "errors": [f"cannot load contract: {type(error).__name__}: {error}"],
            "tracks": [],
        }
    contract_errors = validate_contract(contract)
    tracks = [
        evaluate_track(track, root=root, contract_errors=contract_errors)
        for track in contract.get("tracks", [])
        if isinstance(track, dict)
    ]
    state_counts = Counter(row["derived_current_state"] for row in tracks)
    phase5_complete = (
        len(tracks) == len(EXPECTED_TRACK_IDS)
        and all(row["phase5_track_complete"] for row in tracks)
    )
    passed = not contract_errors and all(row["passed"] for row in tracks)
    return {
        "schema_version": 1,
        "report_kind": "quickpls_phase5_cbsem_expansion_audit",
        "passed": passed,
        "audit_pass_meaning": (
            "Current source bytes reproduce the declared support boundary; this is not "
            "a Phase-5 completion or method-promotion claim."
        ),
        "program_id": PROGRAM_ID,
        "program_version": PROGRAM_VERSION,
        "catalogue_snapshot_date": CATALOGUE_DATE,
        "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "phase5_complete": phase5_complete,
        "competitor_claim_for_expanded_cbsem_admissible": phase5_complete,
        "safe_current_cbsem_claim": (
            "QuickPLS has a release-qualified bounded single-group continuous reflective "
            "raw-data ordinary-ML CB-SEM/CFA foundation. Phase-5 expansion methods remain "
            "absent unless separately promoted by their method-factory evidence."
        ),
        "state_counts": dict(sorted(state_counts.items())),
        "release_qualified_track_count": sum(
            row["derived_current_state"] == "release_qualified" for row in tracks
        ),
        "required_track_count": len(EXPECTED_TRACK_IDS),
        "errors": contract_errors,
        "tracks": tracks,
    }


def write_reports(report: dict[str, Any], output: Path) -> list[Path]:
    output.mkdir(parents=True, exist_ok=True)
    paths: list[Path] = []
    for track in report.get("tracks", []):
        path = output / f"{track['track_id']}.identity.json"
        path.write_text(
            json.dumps(track, indent=2, sort_keys=True, allow_nan=False) + "\n",
            encoding="utf-8",
        )
        paths.append(path)
    aggregate = output / "phase5_cbsem_expansion_audit.json"
    aggregate.write_text(
        json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    paths.append(aggregate)
    return paths


def _summary(report: dict[str, Any]) -> str:
    lines = [
        f"Phase-5 CB-SEM support-boundary audit: {'PASS' if report['passed'] else 'FAIL'}",
        f"Phase complete: {report.get('phase5_complete', False)}",
    ]
    for track in report.get("tracks", []):
        lines.append(
            f"- {track['track_id']}: {track['derived_current_state']} "
            f"({track['support_classification']})"
        )
    lines.extend(f"ERROR: {error}" for error in report.get("errors", []))
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository-root", type=Path, default=ROOT)
    parser.add_argument("--contract", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--check-only", action="store_true")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)
    root = args.repository_root.resolve()
    report = audit(root=root, contract_path=args.contract)
    if not args.check_only:
        output = args.output or (root / "validation/results/phase5_cbsem_expansion")
        write_reports(report, output)
    if args.json:
        json.dump(report, sys.stdout, indent=2, sort_keys=True, ensure_ascii=False)
        sys.stdout.write("\n")
    else:
        print(_summary(report))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
