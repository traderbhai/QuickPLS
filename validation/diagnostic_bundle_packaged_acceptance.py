"""Fail-closed validator for the frozen packaged diagnostic-bundle report."""

from __future__ import annotations

import argparse
import hashlib
import ipaddress
import json
import re
import sys
import zipfile
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

VALIDATION_DIR = Path(__file__).resolve().parent
if str(VALIDATION_DIR) not in sys.path:
    sys.path.insert(0, str(VALIDATION_DIR))
from diagnostic_bundle_source_manifest import (  # noqa: E402
    SourceManifestFailure,
    validate_gate_evidence,
)


ROOT = Path(__file__).resolve().parents[1]
SCHEMA = ROOT / "validation" / "diagnostic_bundle_packaged_acceptance.schema.json"
DEFAULT_REPORT = ROOT / "validation" / "results" / "diagnostic_bundle_packaged_acceptance.json"
EXPECTED_ENTRIES = (
    "metadata/system.json",
    "logs/events.jsonl",
    "manifest.json",
)
FROZEN_PRODUCT_BROWSER_SWITCHES = (
    "--disable-background-networking",
    "--disable-component-update",
    "--disable-sync",
    "--metrics-recording-only",
    "--disable-quic",
    "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection",
    "--proxy-server=http://127.0.0.1:17846",
)
REQUIRED_BROWSER_SWITCHES = (*FROZEN_PRODUCT_BROWSER_SWITCHES, "--remote-debugging-port=9222")
ALLOWED_PAGE_ORIGINS = ("http://ipc.localhost", "http://tauri.localhost")


class GateFailure(RuntimeError):
    pass


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def require(condition: bool, message: str) -> None:
    if not condition:
        raise GateFailure(message)


def workspace_path(relative: str) -> Path:
    candidate = (ROOT / relative).resolve()
    try:
        candidate.relative_to(ROOT.resolve())
    except ValueError as error:
        raise GateFailure(f"Evidence artifact escapes the workspace: {relative}") from error
    return candidate


def verify_artifact(artifact: dict[str, Any], label: str) -> Path:
    path = workspace_path(artifact["path"])
    require(path.is_file(), f"{label} is missing: {artifact['path']}")
    require(path.stat().st_size == artifact["size"], f"{label} size drifted")
    require(sha256_file(path) == artifact["sha256"], f"{label} SHA-256 drifted")
    return path


def read_jsonl(path: Path, label: str) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for index, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        require(bool(line.strip()), f"{label} contains a blank row at line {index}")
        value = json.loads(line)
        require(isinstance(value, dict), f"{label} row {index} is not an object")
        rows.append(value)
    require(bool(rows), f"{label} is empty")
    return rows


def static_process_identity(row: dict[str, Any]) -> dict[str, Any]:
    return {key: row[key] for key in (
        "pid", "parent_pid", "name", "role", "creation_date", "executable_path", "command_line"
    )}


def exact_switch_count(command_line: str, switch: str) -> int:
    return len(re.findall(rf"(?<!\S){re.escape(switch)}(?=\s|$)", command_line, flags=re.IGNORECASE))


def validate_runtime_process_evidence(
    process_report: dict[str, Any], samples: list[dict[str, Any]], root_pid: int
) -> int:
    require(len(samples) == process_report["sample_count"], "Process sample count disagrees with report")
    identities: dict[tuple[int, str, str], dict[str, Any]] = {}
    browser_keys: set[tuple[int, str, str]] = set()
    for index, sample in enumerate(samples, start=1):
        require(sample.get("root_present") is True and sample.get("root_pid") == root_pid, f"Process sample {index} root identity drifted")
        processes = sample.get("processes")
        require(isinstance(processes, list) and processes, f"Process sample {index} has no exact-tree rows")
        roots = [row for row in processes if row.get("pid") == root_pid and row.get("role") == "desktop_root"]
        require(len(roots) == 1 and str(roots[0].get("name", "")).lower() == "quickpls-desktop.exe", f"Process sample {index} root row is not exact")
        browsers = [row for row in processes if row.get("parent_pid") == root_pid and str(row.get("name", "")).lower() == "msedgewebview2.exe" and row.get("role") == "webview_browser"]
        require(len(browsers) == 1, f"Process sample {index} does not have exactly one direct WebView2 browser child")
        for row in processes:
            require(row.get("role") != "other_descendant", f"Process sample {index} contains an unexpected descendant")
            require(bool(row.get("executable_path")) and bool(row.get("command_line")), f"Process sample {index} omitted an exact executable path or command line")
            identity = static_process_identity(row)
            key = (identity["pid"], identity["creation_date"], identity["name"].lower())
            require(key not in identities or identities[key] == identity, f"Process identity changed during sampling: {key}")
            identities[key] = identity
        browser = static_process_identity(browsers[0])
        browser_keys.add((browser["pid"], browser["creation_date"], browser["name"].lower()))
    require(len(browser_keys) == 1, "More than one direct WebView2 browser identity was observed")
    browser = identities[next(iter(browser_keys))]
    require(browser["parent_pid"] == root_pid, "WebView2 browser is not a direct QuickPLS child")
    require(tuple(process_report["frozen_product_browser_switches"]) == FROZEN_PRODUCT_BROWSER_SWITCHES, "Frozen product browser-switch list drifted")
    require(tuple(process_report["acceptance_only_browser_switches"]) == ("--remote-debugging-port=9222",), "Acceptance-only browser-switch list drifted")
    command_line = browser["command_line"]
    for switch in REQUIRED_BROWSER_SWITCHES:
        require(exact_switch_count(command_line, switch) == 1, f"WebView2 browser switch is missing or duplicated: {switch}")
        switch_family = switch.split("=", 1)[0]
        family_matches = re.findall(rf"(?<!\S){re.escape(switch_family)}(?:=\S+)?(?=\s|$)", command_line, flags=re.IGNORECASE)
        require(family_matches == [switch], f"WebView2 browser switch is missing, duplicated, or conflicting: {switch_family}")
    proxy_switches = re.findall(r"(?<!\S)--proxy-server(?:=\S+|\s+\S+)", command_line, flags=re.IGNORECASE)
    debug_switches = re.findall(r"(?<!\S)--remote-debugging-port(?:=\S+|\s+\S+)", command_line, flags=re.IGNORECASE)
    require(proxy_switches == ["--proxy-server=http://127.0.0.1:17846"], "WebView2 proxy switch is missing, duplicated, or conflicting")
    require(debug_switches == ["--remote-debugging-port=9222"], "WebView2 CDP switch is missing, duplicated, or conflicting")
    observed = sorted(identities.values(), key=lambda row: row["pid"])
    require(process_report["observed_processes"] == observed, "Persisted process identities disagree with exact samples")
    require(process_report["direct_webview_browser_child"] == browser, "Persisted direct WebView2 browser identity disagrees with samples")
    require(process_report["direct_webview_browser_child_count"] == 1, "Direct WebView2 browser count drifted")
    return browser["pid"]


def is_loopback(address: str) -> bool:
    try:
        parsed = ipaddress.ip_address(address.strip("[]"))
        mapped = parsed.ipv4_mapped if isinstance(parsed, ipaddress.IPv6Address) else None
        return parsed.is_loopback or (mapped is not None and mapped.is_loopback)
    except ValueError:
        return False


def is_unspecified(address: str) -> bool:
    try:
        return ipaddress.ip_address(address.strip("[]")).is_unspecified
    except ValueError:
        return False


def validate_runtime_tcp_evidence(
    tcp_report: dict[str, Any], samples: list[dict[str, Any]], root_pid: int, browser_pid: int
) -> None:
    require(len(samples) == tcp_report["sample_count"], "TCP sample count disagrees with report")
    observed: list[dict[str, Any]] = []
    remote: list[dict[str, Any]] = []
    unexpected: list[dict[str, Any]] = []
    allowed_rows = 0
    proxy_listener_samples = 0
    cdp_listener_samples = 0
    expected_proxy = {"owning_process": root_pid, "local_address": "127.0.0.1", "local_port": 17846, "remote_address": "0.0.0.0", "remote_port": 0, "state": "Listen", "remote_access": False}
    expected_cdp = {"owning_process": browser_pid, "local_address": "127.0.0.1", "local_port": 9222, "remote_address": "0.0.0.0", "remote_port": 0, "state": "Listen", "remote_access": False}
    for index, sample in enumerate(samples, start=1):
        require(sample.get("root_present") is True and sample.get("root_pid") == root_pid, f"TCP sample {index} root identity drifted")
        process_ids = sample.get("process_ids")
        connections = sample.get("connections")
        require(isinstance(process_ids, list) and root_pid in process_ids, f"TCP sample {index} omitted the root process")
        require(isinstance(connections, list), f"TCP sample {index} connections are missing")
        proxy_count = sum(row == expected_proxy for row in connections)
        cdp_count = sum(row == expected_cdp for row in connections)
        require(proxy_count == 1, f"TCP sample {index} does not contain one exact QuickPLS proxy listener")
        require(cdp_count == 1, f"TCP sample {index} does not contain one exact WebView2 CDP listener")
        proxy_listener_samples += 1
        cdp_listener_samples += 1
        for row in connections:
            require(row["owning_process"] in process_ids, f"TCP sample {index} contains a socket outside its exact process tree")
            state = row["state"]
            computed_remote = state not in {"Listen", "Bound", "Closed"} and not is_loopback(row["remote_address"]) and not is_unspecified(row["remote_address"])
            require(row["remote_access"] is computed_remote, f"TCP sample {index} remote-access classification drifted")
            observed.append(row)
            if computed_remote:
                remote.append(row)
                continue
            if state in {"Bound", "Closed"}:
                if not is_unspecified(row["remote_address"]) or row["remote_port"] != 0:
                    unexpected.append(row)
                continue
            allowed = row in (expected_proxy, expected_cdp) or (
                is_loopback(row["local_address"])
                and is_loopback(row["remote_address"])
                and (row["local_port"] in {9222, 17846} or row["remote_port"] in {9222, 17846})
            )
            if allowed:
                allowed_rows += 1
            else:
                unexpected.append(row)
    require(not remote, "A non-loopback exact-tree TCP row was observed")
    require(not unexpected, "A loopback TCP row outside CDP 9222 or proxy 17846 was observed")
    require(tcp_report["observed_tcp_connection_rows"] == len(observed), "Observed TCP row count drifted")
    require(tcp_report["remote_connections"] == remote and tcp_report["remote_connection_count"] == len(remote), "Remote TCP evidence drifted")
    require(tcp_report["unexpected_loopback_connections"] == unexpected and tcp_report["unexpected_loopback_connection_count"] == len(unexpected), "Unexpected loopback evidence drifted")
    require(tcp_report["allowed_loopback_connection_rows"] == allowed_rows, "Allowed loopback TCP row count drifted")
    require(tcp_report["proxy_listener"] == expected_proxy and tcp_report["proxy_listener_sample_count"] == proxy_listener_samples, "QuickPLS proxy listener evidence drifted")
    require(tcp_report["cdp_listener"] == expected_cdp and tcp_report["cdp_listener_sample_count"] == cdp_listener_samples, "WebView2 CDP listener evidence drifted")


def validate_report(report_path: Path) -> dict[str, Any]:
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    report = json.loads(report_path.read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema, format_checker=FormatChecker()).iter_errors(report),
        key=lambda error: list(error.absolute_path),
    )
    if errors:
        details = [
            f"/{'/'.join(str(part) for part in error.absolute_path)}: {error.message}"
            for error in errors[:20]
        ]
        raise GateFailure("Schema validation failed:\n" + "\n".join(details))

    desktop = verify_artifact(
        report["tested_product"]["quickpls_desktop_exe"], "tested desktop executable"
    )
    require(desktop.name.lower() == "quickpls-desktop.exe", "Desktop executable identity drifted")
    try:
        validate_gate_evidence(report["source_artifacts"], ROOT)
    except SourceManifestFailure as error:
        raise GateFailure(str(error)) from error
    source_desktop = report["source_artifacts"]["after"]["tested_desktop"]
    require(
        report["tested_product"]["quickpls_desktop_exe"]
        == {key: source_desktop[key] for key in ("path", "size", "sha256")},
        "Tested desktop identity differs from source/build evidence",
    )
    artifact_paths: dict[str, Path] = {}
    for name, artifact in report["artifacts"].items():
        artifact_paths[name] = verify_artifact(artifact, name)
    source_evidence_file = json.loads(
        artifact_paths["source_evidence"].read_text(encoding="utf-8")
    )
    source_before_file = json.loads(
        artifact_paths["source_before"].read_text(encoding="utf-8")
    )
    build_receipt_file = json.loads(
        artifact_paths["build_receipt"].read_text(encoding="utf-8")
    )
    require(
        source_evidence_file == report["source_artifacts"],
        "Embedded source evidence differs from its exact artifact",
    )
    require(
        source_before_file == report["source_artifacts"]["before"],
        "Gate-start source snapshot differs from its exact artifact",
    )
    require(
        build_receipt_file == report["source_artifacts"]["build_receipt"],
        "Embedded build receipt differs from its exact artifact",
    )

    checks = report["checks"]
    require(json.loads(artifact_paths["process_report"].read_text(encoding="utf-8")) == checks["process_observation"], "Embedded process report differs from its exact artifact")
    require(json.loads(artifact_paths["network_report"].read_text(encoding="utf-8")) == checks["network_observation"]["tcp_monitor"], "Embedded TCP report differs from its exact artifact")
    require(json.loads(artifact_paths["cleanup_report"].read_text(encoding="utf-8")) == checks["cleanup"], "Embedded cleanup report differs from its exact artifact")
    native_save = checks["native_save_dialog"]
    archive = checks["archive_integrity"]
    zip_artifact = report["artifacts"]["diagnostic_zip"]
    helper_bundle = native_save["helper_completion"]["bundle"]
    save_result = native_save["save_result"]
    require(save_result["bytes"] == archive["archive_bytes"] == zip_artifact["size"], "Archive byte identities disagree")
    require(save_result["archiveSha256"] == archive["archive_sha256"] == zip_artifact["sha256"], "Archive SHA-256 identities disagree")
    require(helper_bundle["size"] == zip_artifact["size"] and helper_bundle["sha256"] == zip_artifact["sha256"], "Helper bundle identity disagrees with artifact")
    require(Path(helper_bundle["path"]).resolve() == artifact_paths["diagnostic_zip"], "Helper bundle path disagrees with artifact")
    require(
        checks["destination_rejections"]["existing_destination_sha256_before"]
        == checks["destination_rejections"]["existing_destination_sha256_after"],
        "No-overwrite sentinel hashes disagree",
    )
    counts = archive["redaction_counts"]
    require(archive["redaction_total"] == sum(counts.values()), "Redaction total is inconsistent")
    require(
        checks["live_settings_preview"]["staged_contents"]
        == {
            "system": helper_bundle["system"],
            "events": helper_bundle["events"],
            "manifest": helper_bundle["manifest"],
        },
        "Live preview is not byte-content-equivalent to the saved staging payload",
    )

    cleanup = checks["cleanup"]
    root_pid = cleanup["launched_pid"]
    require(checks["process_observation"]["root_pid"] == root_pid, "Process observation root PID drifted")
    require(checks["network_observation"]["tcp_monitor"]["root_pid"] == root_pid, "Network observation root PID drifted")
    require(native_save["helper_ready"]["mainWindow"]["pid"] == root_pid, "Native helper main-window PID drifted")
    require(native_save["helper_completion"]["mainWindow"]["pid"] == root_pid, "Native helper completion PID drifted")
    tauri_config = json.loads((ROOT / "src-tauri" / "tauri.conf.json").read_text(encoding="utf-8"))
    configured_switches = tuple(tauri_config["app"]["windows"][0]["additionalBrowserArgs"].split())
    require(configured_switches == FROZEN_PRODUCT_BROWSER_SWITCHES, "Frozen Tauri additionalBrowserArgs drifted")
    require(tauri_config["app"]["windows"][0]["proxyUrl"] == "http://127.0.0.1:17846", "Frozen Tauri proxyUrl drifted")
    process_samples = read_jsonl(artifact_paths["process_samples"], "process samples")
    network_samples = read_jsonl(artifact_paths["network_samples"], "network samples")
    browser_pid = validate_runtime_process_evidence(checks["process_observation"], process_samples, root_pid)
    validate_runtime_tcp_evidence(checks["network_observation"]["tcp_monitor"], network_samples, root_pid, browser_pid)
    observed_origins = sorted({row["origin"] for row in report["browser_requests"]})
    external_requests = [row for row in report["browser_requests"] if row["origin"] not in ALLOWED_PAGE_ORIGINS]
    require(tuple(observed_origins) == ALLOWED_PAGE_ORIGINS and not external_requests, "Browser request origins escaped exact packaged Tauri/IPC origins")
    for browser_check in (checks["browser_network_observation"], checks["network_observation"]["browser_request_observation"]):
        require(tuple(browser_check["allowed_origins"]) == ALLOWED_PAGE_ORIGINS, "Allowed browser origins drifted")
        require(browser_check["observed_origins"] == observed_origins, "Observed browser origins disagree with request evidence")
        require(browser_check["request_count"] == len(report["browser_requests"]), "Browser request count drifted")
        require(browser_check["external_requests"] == external_requests and browser_check["external_request_count"] == len(external_requests), "External browser request evidence drifted")

    zip_path = artifact_paths["diagnostic_zip"]
    with zipfile.ZipFile(zip_path, "r") as bundle:
        infos = bundle.infolist()
        require(tuple(info.filename for info in infos) == EXPECTED_ENTRIES, "ZIP entry identity drifted during final audit")
        require(bundle.testzip() is None, "ZIP corruption detected during final audit")
        require(all(info.compress_type == zipfile.ZIP_STORED for info in infos), "ZIP compression drifted during final audit")
        contents = {info.filename: bundle.read(info.filename) for info in infos}
    system = json.loads(contents[EXPECTED_ENTRIES[0]])
    events = [json.loads(line) for line in contents[EXPECTED_ENTRIES[1]].decode("utf-8").splitlines()]
    manifest = json.loads(contents[EXPECTED_ENTRIES[2]])
    require(system == helper_bundle["system"], "System metadata differs from helper evidence")
    require(events == helper_bundle["events"], "Event rows differ from helper evidence")
    require(manifest == helper_bundle["manifest"], "Manifest differs from helper evidence")
    for descriptor in manifest["entries"]:
        payload = contents[descriptor["name"]]
        require(descriptor["bytes"] == len(payload), f"Manifest byte count drifted for {descriptor['name']}")
        require(descriptor["sha256"] == sha256_bytes(payload), f"Manifest SHA-256 drifted for {descriptor['name']}")
    require(len(contents) == 3, "Diagnostic bundle no longer has exactly three entries")

    return {
        "passed": True,
        "report": report_path.relative_to(ROOT).as_posix(),
        "diagnostic_zip": zip_artifact,
        "process_samples": checks["process_observation"]["sample_count"],
        "network_samples": checks["network_observation"]["tcp_monitor"]["sample_count"],
        "remote_connections": checks["network_observation"]["tcp_monitor"]["remote_connection_count"],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", type=Path, default=DEFAULT_REPORT)
    args = parser.parse_args()
    report_path = args.report.resolve()
    try:
        report_path.relative_to(ROOT.resolve())
        outcome = validate_report(report_path)
    except (GateFailure, ValueError, OSError, json.JSONDecodeError, zipfile.BadZipFile) as error:
        print(str(error), file=sys.stderr)
        return 1
    print(json.dumps(outcome, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
