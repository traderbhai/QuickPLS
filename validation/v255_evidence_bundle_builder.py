"""Fail-closed deterministic QuickPLS 2.55 evidence-bundle assembler.

This tool does not collect evidence.  It accepts only already passing named and
frozen crawler outputs, derives the frozen index from parsed receipts, verifies
every staged byte, and writes new proposal files plus a new ZIP.  It never
overwrites an input or output.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import stat
import zipfile
from pathlib import Path, PurePosixPath
from typing import Any

from v255_release_waiver import (
    DPI_WAIVER_MANIFEST_DECLARATION,
    exact_case_waiver_receipt_matches_observation,
    exact_cross_report_waiver_binding,
    exact_population_status,
    exact_release_waiver_matches_observation,
)

ROOT = Path(__file__).resolve().parents[1]
TARGET = "2.55.0"
FROZEN_SUITE = "quickpls_v255_frozen_archive_reopen_crawler_v1"
COLLECTOR_SUITE = "quickpls_v255_named_evidence_collector_v1"


def fail(message: str) -> None:
    raise ValueError(message)


def load(path: Path) -> dict[str, Any]:
    if not path.is_file() or path.stat().st_size > 32 * 1024 * 1024:
        fail(f"missing or oversized JSON: {path}")
    value = json.loads(path.read_text(encoding="utf-8-sig"))
    if not isinstance(value, dict):
        fail(f"JSON root is not an object: {path}")
    return value


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def canonical_digest(value: Any) -> str:
    payload = json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def pointer_value(value: Any, pointer: object) -> Any:
    if not isinstance(pointer, str) or not pointer.startswith("/"):
        fail(f"invalid JSON pointer: {pointer!r}")
    current = value
    for raw_token in pointer.split("/")[1:]:
        token = raw_token.replace("~1", "/").replace("~0", "~")
        if isinstance(current, dict):
            if token not in current:
                fail(f"JSON pointer does not resolve: {pointer}")
            current = current[token]
        elif isinstance(current, list):
            if not token.isdigit() or int(token) >= len(current):
                fail(f"JSON pointer does not resolve: {pointer}")
            current = current[int(token)]
        else:
            fail(f"JSON pointer does not resolve: {pointer}")
    return current


def safe(name: object) -> bool:
    if not isinstance(name, str) or not name or "\\" in name or "\x00" in name or ":" in name:
        return False
    value = PurePosixPath(name)
    return not value.is_absolute() and value.as_posix() == name and all(part not in {"", ".", ".."} for part in value.parts)


def member_path(root: Path, member: object) -> Path:
    if not safe(member):
        fail(f"unsafe member: {member!r}")
    result = root.joinpath(*PurePosixPath(str(member)).parts).resolve()
    try:
        result.relative_to(root.resolve())
    except ValueError:
        fail(f"member escapes staging: {member}")
    if not result.is_file() or result.is_symlink():
        fail(f"member is missing, non-file, or symlink: {member}")
    return result


def files_below(root: Path) -> set[str]:
    if not root.is_dir():
        fail(f"staging directory is missing: {root}")
    result: set[str] = set()
    for item in root.rglob("*"):
        if item.is_symlink():
            fail(f"staging contains a symlink: {item}")
        if item.is_file():
            result.add(item.relative_to(root).as_posix())
    return result


def write_json_new(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("x", encoding="utf-8", newline="\n") as stream:
        json.dump(value, stream, ensure_ascii=False, indent=2)
        stream.write("\n")


def assert_sha(root: Path, ref: dict[str, Any], label: str) -> str:
    member = ref.get("member")
    declared = ref.get("sha256")
    path = member_path(root, member)
    if not isinstance(declared, str) or len(declared) != 64 or digest(path) != declared:
        fail(f"{label} SHA-256 mismatch: {member}")
    return str(member)


def derive_frozen(template: dict[str, Any], staging: Path, aggregate: dict[str, Any]) -> dict[str, Any]:
    if not (aggregate.get("schema_version") == 1 and aggregate.get("suite_id") == FROZEN_SUITE and aggregate.get("target_release") == TARGET and aggregate.get("status") == "passed" and not aggregate.get("failures")):
        fail("frozen aggregate is not a passing exact 2.55 crawler report")
    artifacts = aggregate.get("method_receipts")
    methods = template.get("methods")
    if not isinstance(artifacts, list) or len(artifacts) != 18 or not isinstance(methods, list) or len(methods) != 18:
        fail("frozen aggregate/template must each contain exactly 18 methods")
    by_kind = {row.get("kind"): row for row in methods if isinstance(row, dict)}
    if len(by_kind) != 18:
        fail("frozen template method kinds are not unique")
    proposal = copy.deepcopy(template)
    proposed_by_kind = {row["kind"]: row for row in proposal["methods"]}
    for artifact in artifacts:
        if not isinstance(artifact, dict):
            fail("frozen receipt artifact is not an object")
        receipt_member = assert_sha(staging, artifact, "frozen receipt")
        receipt = load(member_path(staging, receipt_member))
        kind = receipt.get("method_kind")
        declared = by_kind.get(kind)
        covers = receipt.get("covers")
        assertions = receipt.get("cover_assertions")
        if not (receipt.get("schema_version") == 1 and receipt.get("suite_id") == FROZEN_SUITE and receipt.get("target_release") == TARGET and receipt.get("status") == "verified_current_ui_capture" and isinstance(declared, dict)):
            fail(f"invalid frozen method receipt: {kind!r}")
        required = declared.get("representative_results")
        if not isinstance(covers, list) or set(covers) != set(required or []) or len(covers) != len(required or []):
            fail(f"frozen receipt does not exactly cover declared result families: {kind}")
        if not isinstance(assertions, list) or {row.get("family") for row in assertions if isinstance(row, dict)} != set(covers) or not all(isinstance(row, dict) and row.get("passed") is True and row.get("observed_value") for row in assertions):
            fail(f"frozen receipt lacks observed cover assertions: {kind}")
        archive_member = assert_sha(staging, receipt.get("archive") or {}, f"{kind} archive")
        screenshot_member = assert_sha(staging, receipt.get("screenshot") or {}, f"{kind} screenshot")
        identity = receipt.get("declared_identity")
        if not isinstance(identity, dict) or not isinstance(identity.get("value"), str) or not identity["value"]:
            fail(f"frozen receipt lacks declared result identity: {kind}")
        proposed_by_kind[kind]["status"] = "verified"
        proposed_by_kind[kind]["evidence"] = [{
            "covers": covers,
            "cover_assertions": assertions,
            "archive": {"member": archive_member, "sha256": receipt["archive"]["sha256"]},
            "screenshot": {"member": screenshot_member, "sha256": receipt["screenshot"]["sha256"]},
            "receipt": {"member": receipt_member, "sha256": artifact["sha256"], "method_kind_json_pointer": "/method_kind", "canonical_result_id_json_pointer": "/declared_identity/value"},
            "identity": {"canonical_result_id": identity["value"], "method_kind": kind},
        }]
    if set(proposed_by_kind) != {row.get("method_kind") for row in (load(member_path(staging, item["member"])) for item in artifacts)}:
        fail("frozen receipts do not cover each public method exactly once")
    proposal["status"] = "verified"
    return proposal


def main() -> int:
    parser = argparse.ArgumentParser(description="Build a new deterministic QuickPLS 2.55 evidence ZIP from passing staged evidence.")
    parser.add_argument("--named-staging", required=True, type=Path)
    parser.add_argument("--named-collector-report", required=True, type=Path)
    parser.add_argument("--named-collected-index", required=True, type=Path)
    parser.add_argument("--frozen-staging", required=True, type=Path)
    parser.add_argument("--frozen-aggregate-report", required=True, type=Path)
    parser.add_argument("--frozen-index-template", type=Path, default=ROOT / "validation/v255_frozen_result_archive_index.json")
    parser.add_argument("--bundle-manifest-template", type=Path, default=ROOT / "validation/v255_evidence_bundle_manifest.json")
    parser.add_argument("--output-dir", required=True, type=Path)
    args = parser.parse_args()
    named_root, frozen_root, output = args.named_staging.resolve(), args.frozen_staging.resolve(), args.output_dir.resolve()
    if output.exists():
        fail(f"refusing to overwrite output directory: {output}")
    output.mkdir(parents=True, exist_ok=False)
    collector = load(args.named_collector_report.resolve())
    named_index = load(args.named_collected_index.resolve())
    summary = collector.get("summary", {})
    entries = named_index.get("entries", [])
    normal_collection = (
        collector.get("status") == "passed"
        and summary.get("verified") == 55
        and summary.get("waived") == 0
        and named_index.get("status") == "verified"
        and all(isinstance(entry, dict) and entry.get("status") == "verified" for entry in entries)
    )
    waived_collection = (
        collector.get("status") == "passed_with_waiver"
        and summary.get("verified") == 54
        and summary.get("waived") == 1
        and exact_population_status(entries, named_index.get("status"))
    )
    if not (collector.get("schema_version") == 1 and collector.get("suite_id") == COLLECTOR_SUITE and collector.get("target_release") == TARGET and collector.get("passed") is True and summary.get("collected") == 55 and len(entries) == 55 and (normal_collection or waived_collection)):
        fail("named collector/index are not an exact 55-case collection with zero waivers or the one approved DPI waiver")
    named_members = collector.get("bundle_members")
    if not isinstance(named_members, list) or len(named_members) != len(set(named_members)):
        fail("named collector has no unique exact member inventory")
    for member in named_members:
        member_path(named_root, member)
    expected_named_files = set(named_members) | {args.named_collected_index.resolve().relative_to(named_root).as_posix()}
    if files_below(named_root) != expected_named_files:
        fail("named staging contains missing or undeclared files")

    candidate_member = collector.get("sources", {}).get("candidate_report_member")
    candidate_path = member_path(named_root, candidate_member)
    candidate = load(candidate_path)
    candidate_hash = collector.get("sources", {}).get("candidate_report_sha256")
    if not isinstance(candidate_hash, str) or digest(candidate_path) != candidate_hash:
        fail("named collector does not hash-bind its candidate report")
    if waived_collection:
        waived_entries = [
            entry
            for entry in entries
            if isinstance(entry, dict) and entry.get("status") == "waived"
        ]
        waived_entry = waived_entries[0]
        receipt_ref = waived_entry.get("receipt", {})
        receipt_path = member_path(named_root, receipt_ref.get("member"))
        if digest(receipt_path) != receipt_ref.get("sha256"):
            fail("waived receipt does not match its collected-index SHA-256")
        receipt = load(receipt_path)
        source_ref = receipt.get("source_report", {})
        source_path = member_path(named_root, source_ref.get("member"))
        if digest(source_path) != source_ref.get("sha256"):
            fail("waived source report does not match its collected receipt SHA-256")
        source = load(source_path)
        observation = pointer_value(source, source_ref.get("json_pointer"))
        screenshot_ref = waived_entry.get("screenshot", {})
        screenshot_path = member_path(named_root, screenshot_ref.get("member"))
        candidate_waivers = candidate.get("release_waivers")
        if not (
            canonical_digest(observation) == source_ref.get("observation_sha256")
            and isinstance(candidate_waivers, list)
            and len(candidate_waivers) == 1
            and exact_release_waiver_matches_observation(
                candidate_waivers[0], observation
            )
            and exact_cross_report_waiver_binding(source, observation)
            and exact_case_waiver_receipt_matches_observation(receipt, observation)
            and receipt.get("candidate_report", {}).get("member") == candidate_member
            and receipt.get("candidate_report", {}).get("sha256") == candidate_hash
            and receipt.get("screenshot", {}).get("member")
            == screenshot_ref.get("member")
            and receipt.get("screenshot", {}).get("sha256")
            == screenshot_ref.get("sha256")
            and digest(screenshot_path) == screenshot_ref.get("sha256")
        ):
            fail(
                "waiver is not exactly bound across candidate, cross-report DPI, source observation, receipt, and screenshot"
            )
    elif not (
        candidate.get("qualification_status") == "passed"
        and candidate.get("release_waivers") == []
    ):
        fail("strict named evidence requires an exact zero-waiver candidate report")

    frozen_aggregate = load(args.frozen_aggregate_report.resolve())
    frozen_index = derive_frozen(load(args.frozen_index_template.resolve()), frozen_root, frozen_aggregate)
    before = frozen_aggregate.get("staged_members_before_manifest")
    aggregate_member = args.frozen_aggregate_report.resolve().relative_to(frozen_root).as_posix()
    if not isinstance(before, list) or files_below(frozen_root) != set(before) | {aggregate_member}:
        fail("frozen staging does not exactly match its aggregate member inventory")
    frozen_members = sorted(set(before) | {aggregate_member})
    names = sorted(set(named_members) | set(frozen_members))
    if len(names) != len(named_members) + len(frozen_members):
        fail("named and frozen staged members collide")

    zip_path = output / "QuickPLS-2.55-evidence.zip"
    with zipfile.ZipFile(zip_path, "x", compression=zipfile.ZIP_DEFLATED, compresslevel=9, strict_timestamps=True) as archive:
        for name in names:
            source = member_path(named_root if name in set(named_members) else frozen_root, name)
            info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 3
            info.external_attr = (stat.S_IFREG | 0o644) << 16
            info.flag_bits = 0
            archive.writestr(info, source.read_bytes(), compress_type=zipfile.ZIP_DEFLATED, compresslevel=9)
    with zipfile.ZipFile(zip_path, "r") as archive:
        if archive.testzip() is not None or [row.filename for row in archive.infolist()] != names or any(row.flag_bits & 1 for row in archive.infolist()):
            fail("new deterministic ZIP failed CRC/member/encryption verification")
        infos = archive.infolist()
        compressed = sum(row.compress_size for row in infos)
        uncompressed = sum(row.file_size for row in infos)

    manifest = copy.deepcopy(load(args.bundle_manifest_template.resolve()))
    collector_member = args.named_collector_report.resolve().relative_to(named_root).as_posix()
    candidate_member = collector["sources"]["candidate_report_member"]
    schema_member = collector["sources"]["observation_schema_member"]
    named_manifest_member = collector["sources"]["named_case_manifest_member"]
    manifest["status"] = "verified_with_waiver" if waived_collection else "verified"
    manifest["bundle"].update({"sha256": digest(zip_path), "member_count": len(names), "ordered_member_names_sha256": hashlib.sha256("\n".join(names).encode()).hexdigest(), "compressed_bytes": compressed, "uncompressed_bytes": uncompressed})
    manifest["named_evidence"].update({
        "collector_report": {"member": collector_member, "sha256": digest(member_path(named_root, collector_member))},
        "candidate_report": {"member": candidate_member, "sha256": collector["sources"]["candidate_report_sha256"]},
        "observation_schema": {"member": schema_member, "sha256": collector["sources"]["observation_schema_sha256"]},
        "named_case_manifest": {"member": named_manifest_member, "sha256": collector["sources"]["named_case_manifest_sha256"]},
        "source_commit": collector["provenance"]["source_commit"],
        "candidate_executables": collector["provenance"]["candidate_executables"],
        "approved_release_waiver": DPI_WAIVER_MANIFEST_DECLARATION,
    })
    write_json_new(output / "v255_named_evidence_index.proposed.json", named_index)
    write_json_new(output / "v255_frozen_result_archive_index.proposed.json", frozen_index)
    write_json_new(output / "v255_evidence_bundle_manifest.proposed.json", manifest)
    report = {"schema_version": 1, "suite_id": "quickpls_v255_evidence_bundle_builder_v1", "target_release": TARGET, "passed": True, "qualification_status": "passed_with_waiver" if waived_collection else "passed", "approved_release_waiver": DPI_WAIVER_MANIFEST_DECLARATION if waived_collection else None, "bundle": str(zip_path), "bundle_sha256": digest(zip_path), "member_count": len(names), "compressed_bytes": compressed, "uncompressed_bytes": uncompressed, "proposals": {"named_index": str(output / "v255_named_evidence_index.proposed.json"), "frozen_index": str(output / "v255_frozen_result_archive_index.proposed.json"), "bundle_manifest": str(output / "v255_evidence_bundle_manifest.proposed.json")}, "apply_rule": "Review then atomically replace the three source contracts; never edit hashes by hand."}
    write_json_new(output / "v255_evidence_bundle_builder.json", report)
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
