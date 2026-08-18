from __future__ import annotations

import importlib.util
import json
import os
import re
import subprocess
import tempfile
import time
import unittest
import zipfile
from copy import deepcopy
from hashlib import sha256
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker


ROOT = Path(__file__).resolve().parents[1]
SCHEMA_PATH = ROOT / "validation" / "diagnostic_bundle_packaged_acceptance.schema.json"
HARNESS_PATH = ROOT / "validation" / "diagnostic_bundle_packaged_acceptance.mjs"
HELPER_PATH = ROOT / "validation" / "windows_native_save_diagnostic_bundle.py"
WRAPPER_PATH = ROOT / "validation" / "run_diagnostic_bundle_packaged_acceptance.ps1"
NETWORK_MONITOR_PATH = ROOT / "validation" / "monitor_quickpls_network.ps1"
PROCESS_MONITOR_PATH = ROOT / "validation" / "monitor_quickpls_process_tree.ps1"
VALIDATOR_PATH = ROOT / "validation" / "diagnostic_bundle_packaged_acceptance.py"
SOURCE_MANIFEST_PATH = ROOT / "validation" / "diagnostic_bundle_source_manifest.py"
SETTINGS_PATH = ROOT / "src" / "components" / "SettingsWorkspace.tsx"
UTILITY_PATH = ROOT / "src" / "native" / "NativeUtilityDialog.tsx"
APP_PATH = ROOT / "src" / "App.tsx"
GATE_ONLY_PATHS = [
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
]
REQUIRED_BROWSER_SWITCHES = [
    "--disable-background-networking",
    "--disable-component-update",
    "--disable-sync",
    "--metrics-recording-only",
    "--disable-quic",
    "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection",
    "--proxy-server=http://127.0.0.1:17846",
    "--remote-debugging-port=9222",
]


def load_helper():
    spec = importlib.util.spec_from_file_location("diagnostic_save_helper", HELPER_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec and spec.loader
    spec.loader.exec_module(module)
    return module


def load_validator_module():
    spec = importlib.util.spec_from_file_location("diagnostic_report_validator", VALIDATOR_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec and spec.loader
    spec.loader.exec_module(module)
    return module


def load_source_manifest_module():
    spec = importlib.util.spec_from_file_location("diagnostic_source_manifest", SOURCE_MANIFEST_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec and spec.loader
    spec.loader.exec_module(module)
    return module


def artifact(path: str) -> dict:
    return {"path": path, "size": 123, "sha256": "a" * 64}


def source_descriptor(path: str, *, sha: str = "a" * 64, size: int = 123, mtime_ns: int = 1_799_999_999_000_000_000) -> dict:
    return {"path": path, "size": size, "sha256": sha, "mtime_ns": mtime_ns}


def descriptor_manifest(paths: list[str]) -> dict:
    return {"descriptors": [source_descriptor(path) for path in paths], "manifest_sha256": "b" * 64}


def source_artifacts() -> dict:
    product_paths = ["src/App.tsx"]
    discovery = {
        "paths": product_paths,
        "vite_config_precedence": [
            "vite.config.js", "vite.config.mjs", "vite.config.ts",
            "vite.config.cjs", "vite.config.mts", "vite.config.cts",
        ],
        "present_vite_configs": ["vite.config.js", "vite.config.ts"],
        "active_vite_config": "vite.config.js",
        "present_production_env_files": [],
        "desktop_cargo_manifests": [
            "crates/qpls-assessment/Cargo.toml", "crates/qpls-core/Cargo.toml",
            "crates/qpls-data/Cargo.toml", "crates/qpls-estimation/Cargo.toml",
            "crates/qpls-project/Cargo.toml", "crates/qpls-resampling/Cargo.toml",
            "crates/qpls-runner/Cargo.toml", "src-tauri/Cargo.toml",
        ],
        "tauri_configs": ["src-tauri/tauri.conf.json"],
        "package_build_script": "npm run typecheck:build && npm run build:bundle",
        "package_tauri_script": "tauri",
        "tauri_before_build_command": "npm run build",
        "tauri_frontend_dist": "../dist",
    }
    product = {"discovery": discovery, **descriptor_manifest(product_paths)}
    gate = descriptor_manifest(GATE_ONLY_PATHS)
    dist = descriptor_manifest(["dist/index.html"])
    dep_info = {
        "descriptor": source_descriptor("target/release/quickpls-desktop.d"),
        "repository_dependency_paths": ["dist/index.html", "src/App.tsx"],
        "dist_dependency_paths": ["dist/index.html"],
        "dist_set_exact": True,
    }
    desktop = source_descriptor(
        "target/release/quickpls-desktop.exe",
        sha="a" * 64,
        mtime_ns=1_800_000_000_000_000_000,
    )
    freshness = {
        "passed": True,
        "tested_desktop_path": "target/release/quickpls-desktop.exe",
        "tested_desktop_mtime_unix_ns": desktop["mtime_ns"],
        "newest_product_input_path": "src/App.tsx",
        "newest_product_input_mtime_unix_ns": 1_799_999_999_000_000_000,
        "build_started_unix_ns": 1_799_999_999_500_000_000,
        "build_finished_unix_ns": 1_800_000_000_500_000_000,
        "desktop_not_older_than_every_product_input": True,
        "desktop_created_during_recorded_build": True,
    }
    receipt = {
        "schema_version": "quickpls.diagnostic_bundle_build_receipt.v1",
        "kind": "quickpls_diagnostic_packaged_frozen_build_v1",
        "passed": True,
        "generated_at_utc": "2026-08-13T12:20:00Z",
        "build_command": ["npm.cmd", "run", "tauri", "--", "build"],
        "build_started_at_utc": "2026-08-13T12:00:00Z",
        "build_finished_at_utc": "2026-08-13T12:10:00Z",
        "build_started_unix_ns": freshness["build_started_unix_ns"],
        "build_finished_unix_ns": freshness["build_finished_unix_ns"],
        "build_exit_code": 0,
        "source_before": product,
        "source_after": deepcopy(product),
        "source_stable_during_build": True,
        "dist_after": dist,
        "cargo_dep_info": dep_info,
        "tested_desktop": desktop,
        "dist_bound_to_dep_info": True,
        "freshness": freshness,
    }
    snapshot = {
        "schema_version": "quickpls.diagnostic_bundle_gate_source_snapshot.v1",
        "discovery_contract": "quickpls_diagnostic_packaged_source_manifest_v1",
        "product_source": product,
        "gate_only": gate,
        "dist": dist,
        "cargo_dep_info": dep_info,
        "tested_desktop": desktop,
        "build_receipt": source_descriptor("validation/results/diagnostic_bundle_build_receipt.json"),
        "freshness": freshness,
    }
    return {
        "schema_version": "quickpls.diagnostic_bundle_source_evidence.v1",
        "discovery_contract": "quickpls_diagnostic_packaged_source_manifest_v1",
        "build_receipt_path": "validation/results/diagnostic_bundle_build_receipt.json",
        "build_receipt": receipt,
        "before": snapshot,
        "after": deepcopy(snapshot),
        "source_stable_during_gate": True,
        "freshness": freshness,
    }


def page_state() -> dict:
    return {
        "index": 0,
        "url": "http://tauri.localhost/",
        "origin": "http://tauri.localhost",
        "title": "QuickPLS",
        "shell_visible": True,
        "tauri_runtime": True,
    }


def redaction_counts() -> dict:
    return {
        "windowsPaths": 0,
        "emailAddresses": 0,
        "urlQueriesOrFragments": 0,
        "bearerTokens": 0,
    }


def system_metadata() -> dict:
    return {
        "schemaVersion": 1,
        "quickplsVersion": "2.46.0",
        "releaseChannel": "internal",
        "sourceRevision": "not_provided",
        "osFamily": "windows",
        "architecture": "x86_64",
        "desktopRuntime": "Tauri 2",
        "locale": "not_collected",
        "webview2Version": "not_collected",
        "userDataIncluded": False,
        "networkAccessed": False,
    }


def event_rows() -> list[dict]:
    return [
        {
            "timestamp": "2026-08-13T12:00:00.000Z",
            "sequence": 1,
            "severity": "info",
            "code": "desktop.session.started",
        },
        {
            "timestamp": "2026-08-13T12:00:01.000Z",
            "sequence": 2,
            "severity": "info",
            "code": "diagnostic.preview.requested",
        },
    ]


def manifest() -> dict:
    return {
        "schemaVersion": 1,
        "policyVersion": "quickpls-diagnostics-v1",
        "createdAt": "2026-08-13T12:00:01.000Z",
        "quickplsVersion": "2.46.0",
        "entries": [
            {"name": "metadata/system.json", "sha256": "b" * 64, "bytes": 320},
            {"name": "logs/events.jsonl", "sha256": "c" * 64, "bytes": 220},
        ],
        "redactionCounts": redaction_counts(),
        "redactionTotal": 0,
        "archiveLimits": {
            "maximumEntries": 3,
            "maximumEntryBytes": 262144,
            "maximumUncompressedBytes": 524288,
            "maximumArchiveBytes": 532480,
            "compression": "stored",
        },
        "localOnly": True,
        "networkAccessed": False,
    }


def helper_bundle() -> dict:
    return {
        "path": "D:\\QuickPLS\\validation\\results\\diagnostic.zip",
        "size": 2048,
        "sha256": "d" * 64,
        "entryNames": ["metadata/system.json", "logs/events.jsonl", "manifest.json"],
        "entryCompression": ["stored", "stored", "stored"],
        "entrySizes": {
            "metadata/system.json": 320,
            "logs/events.jsonl": 220,
            "manifest.json": 640,
        },
        "uncompressedBytes": 1180,
        "system": system_metadata(),
        "events": event_rows(),
        "manifest": manifest(),
        "forbiddenPatternMatches": [],
    }


def browser_network() -> dict:
    return {
        "passed": True,
        "observation": "playwright_page_request_events_during_reload_preview_cancel_navigation_save_and_negative_paths_v1",
        "allowed_origins": ["http://ipc.localhost", "http://tauri.localhost"],
        "observed_origins": ["http://ipc.localhost", "http://tauri.localhost"],
        "request_count": 2,
        "external_request_count": 0,
        "external_requests": [],
    }


def runtime_process_identities() -> list[dict]:
    return [
        {
            "pid": 1234, "parent_pid": 1000, "name": "quickpls-desktop.exe", "role": "desktop_root",
            "creation_date": "20260813120000.000000+000", "executable_path": "D:\\QuickPLS\\target\\release\\quickpls-desktop.exe",
            "command_line": '"D:\\QuickPLS\\target\\release\\quickpls-desktop.exe"',
        },
        {
            "pid": 2345, "parent_pid": 1234, "name": "msedgewebview2.exe", "role": "webview_browser",
            "creation_date": "20260813120001.000000+000", "executable_path": "C:\\Program Files (x86)\\Microsoft\\EdgeWebView\\msedgewebview2.exe",
            "command_line": '"C:\\Program Files (x86)\\Microsoft\\EdgeWebView\\msedgewebview2.exe" ' + " ".join(REQUIRED_BROWSER_SWITCHES),
        },
    ]


def runtime_process_samples(count: int = 20) -> list[dict]:
    identities = runtime_process_identities()
    return [
        {
            "recorded_at_utc": f"2026-08-13T12:00:{index:02d}.000Z", "root_present": True, "root_pid": 1234,
            "total_working_set_bytes": 500_000_000, "total_private_memory_bytes": 400_000_000,
            "total_handle_count": 500, "total_thread_count": 80,
            "process_role_counts": {"desktop_root": 1, "webview_browser": 1},
            "processes": deepcopy(identities),
        }
        for index in range(count)
    ]


def listener(pid: int, port: int) -> dict:
    return {
        "owning_process": pid, "local_address": "127.0.0.1", "local_port": port,
        "remote_address": "0.0.0.0", "remote_port": 0, "state": "Listen", "remote_access": False,
    }


def runtime_network_samples(count: int = 20) -> list[dict]:
    connections = [listener(1234, 17846), listener(2345, 9222)]
    return [
        {
            "recorded_at_utc": f"2026-08-13T12:00:{index:02d}.000Z", "root_pid": 1234,
            "root_present": True, "process_ids": [1234, 2345], "connections": deepcopy(connections),
            "remote_connections": [], "observation": "sampled_exact_process_tree_tcp_v1",
        }
        for index in range(count)
    ]


def monitor_cleanup(label: str) -> dict:
    return {
        "label": label,
        "exit_confirmed": True,
        "exit_code": 0,
        "forced_termination": False,
        "stderr": "",
    }


def valid_report() -> dict:
    staged = {"system": system_metadata(), "events": event_rows(), "manifest": manifest()}
    helper = helper_bundle()
    rejection_specs = [
        ("relative", "relative.zip", "DIAGNOSTIC_PATH_NOT_LOCAL_DRIVE"),
        ("unc", "\\\\server\\share\\bundle.zip", "DIAGNOSTIC_PATH_NAMESPACE_BLOCKED"),
        ("verbatim_namespace", "\\\\?\\C:\\Support\\bundle.zip", "DIAGNOSTIC_PATH_NAMESPACE_BLOCKED"),
        ("wrong_extension", "D:\\QuickPLS\\validation\\results\\bundle.qpls", "DIAGNOSTIC_EXTENSION_INVALID"),
        ("reserved_device", "D:\\QuickPLS\\validation\\results\\NUL.zip", "DIAGNOSTIC_DEVICE_NAME_BLOCKED"),
        ("existing_destination", "D:\\QuickPLS\\validation\\results\\existing.zip", "DIAGNOSTIC_DESTINATION_EXISTS"),
    ]
    checks = {
        "runtime_preflight": {
            "passed": True,
            "expected_origin": "http://tauri.localhost",
            "qualifying_page_count": 1,
            "pre_reload": page_state(),
            "reload_count": 1,
            "post_reload": page_state(),
            "same_origin": True,
        },
        "abandoned_preview_recovery": {
            "passed": True,
            "abandoned_preview_count": 7,
            "backend_capacity": 4,
            "evicted_oldest_count": 4,
            "evicted_oldest": [
                {"preview_id": f"00000000-0000-4000-8000-00000000000{index}", "error": "DIAGNOSTIC_PREVIEW_REQUIRED: absent"}
                for index in range(4)
            ],
            "surviving_abandoned_cancelled_count": 3,
            "surviving_abandoned_cancelled_ids": [
                f"00000000-0000-4000-8000-00000000000{index}" for index in range(4, 7)
            ],
            "live_ui_recovered_at_capacity": True,
        },
        "live_settings_preview": {
            "passed": True,
            "production_entry": "src/App.tsx -> NativeDesktopApp -> NativeUtilityDialog -> DiagnosticBundlePanel",
            "dialog_title": "Preferences",
            "panel_marker": "live",
            "existing_preferences_preserved": True,
            "preview_before_save": True,
            "local_only": True,
            "network_activity": "none",
            "entry_count": 3,
            "event_count": 2,
            "included_categories": [
                "QuickPLS build and release identity",
                "Operating-system family and architecture",
                "Bounded session diagnostic event codes",
                "Manifest hashes, sizes, limits, and redaction counts",
            ],
            "excluded_categories": [
                "Dataset rows, values, and variable names",
                "Project contents, model labels, and project titles",
                "Results, reports, and exports",
                "Credentials, environment values, and command lines",
                "Arbitrary files, registry data, and memory dumps",
            ],
            "staged_contents": staged,
            "exact_system_labels": [
                "Metadata schema", "QuickPLS version", "Release channel", "Source revision",
                "Operating system", "Architecture", "Desktop runtime", "Locale",
                "WebView2 version", "User data included", "Network accessed",
            ],
            "exact_descriptor_names": ["metadata/system.json", "logs/events.jsonl"],
            "accessible_regions": ["Redacted diagnostic event rows", "Diagnostic manifest payload descriptors"],
            "live_status_region": True,
        },
        "navigation_cancellation": {
            "passed": True,
            "explicit_preview_cancellation": {
                "passed": True,
                "preview_id": "00000000-0000-4000-8000-000000000007",
                "no_file_created_message": True,
            },
            "unmount_cycles": 5,
            "every_unmount_recorded": True,
            "recovery_preview_id": "00000000-0000-4000-8000-000000000008",
        },
        "native_save_dialog": {
            "passed": True,
            "target_path": helper["path"],
            "new_target": True,
            "local_drive_rooted": True,
            "helper_ready": {
                "event": "ready", "passed": True, "phase": "main_window_binding",
                "targetPath": helper["path"], "mainWindow": {"pid": 1234},
            },
            "helper_completion": {
                "event": "complete", "passed": True,
                "phase": "diagnostic_zip_creation_and_readback", "targetPath": helper["path"],
                "mainWindow": {"pid": 1234}, "dialog": {},
                "boundControls": {"filenameEditControlId": 1001, "saveButtonControlId": 1},
                "saveSubmission": {}, "bundle": helper,
                "transport": {"exitCode": 0, "signal": None, "stderr": "", "events": [{}, {}], "protocolErrors": []},
            },
            "save_result": {"bytes": 2048, "archiveSha256": "d" * 64},
            "app_feedback": "Diagnostic bundle saved locally (2 KiB). QuickPLS did not upload it.",
        },
        "archive_integrity": {
            "passed": True,
            "exact_entry_names": ["metadata/system.json", "logs/events.jsonl", "manifest.json"],
            "exact_entry_count": 3,
            "stored_compression_only": True,
            "entry_sizes": helper["entrySizes"],
            "uncompressed_bytes": 1180,
            "archive_bytes": 2048,
            "archive_sha256": "d" * 64,
            "manifest_payload_descriptors_exact": True,
            "preview_archive_exact": True,
            "redaction_counts": redaction_counts(),
            "redaction_total": 0,
            "forbidden_pattern_matches": [],
            "user_data_included": False,
            "network_accessed_declared": False,
        },
        "destination_rejections": {
            "passed": True,
            "cases": [
                {
                    "case_id": case_id,
                    "path": path,
                    "expected_code": code,
                    "observed_error": f"{code}: rejected",
                    "preview_consumed": True,
                }
                for case_id, path, code in rejection_specs
            ],
            "existing_destination_unchanged": True,
            "existing_destination_sha256_before": "e" * 64,
            "existing_destination_sha256_after": "e" * 64,
        },
        "browser_network_observation": browser_network(),
        "process_observation": {
            "schema_version": 1,
            "passed": True,
            "root_pid": 1234,
            "sample_count": 20,
            "peak_total_working_set_bytes": 500_000_000,
            "peak_total_private_memory_bytes": 400_000_000,
            "peak_total_handle_count": 500,
            "peak_total_thread_count": 80,
            "peak_process_count": 2,
            "peak_working_set_under_2_gib": True,
            "zero_other_descendants": True,
            "process_command_lines_persisted": True,
            "process_identity_stable": True,
            "observed_processes": runtime_process_identities(),
            "direct_webview_browser_child_count": 1,
            "direct_webview_browser_child": runtime_process_identities()[1],
            "direct_webview_browser_observed_in_every_sample": True,
            "frozen_product_browser_switches": REQUIRED_BROWSER_SWITCHES[:-1],
            "acceptance_only_browser_switches": [REQUIRED_BROWSER_SWITCHES[-1]],
            "missing_browser_switches": [],
            "duplicate_browser_switches": [],
            "conflicting_browser_switches": [],
            "browser_switch_contract_passed": True,
            "observation": "sampled exact root process identity, executable path, command line, and descendants with a configured 250 ms inter-sample delay; bounded run, not a sustained no-leak claim",
        },
        "network_observation": {
            "passed": True,
            "tcp_monitor": {
                "schema_version": 1,
                "passed": True,
                "root_pid": 1234,
                "sample_count": 20,
                "observed_tcp_connection_rows": 40,
                "remote_connection_count": 0,
                "remote_connections": [],
                "allowed_loopback_ports": [9222, 17846],
                "loopback_allowed_for_cdp": True,
                "loopback_allowed_for_proxy": True,
                "allowed_loopback_connection_rows": 40,
                "unexpected_loopback_connection_count": 0,
                "unexpected_loopback_connections": [],
                "proxy_listener": listener(1234, 17846),
                "proxy_listener_sample_count": 20,
                "proxy_listener_present_in_every_sample": True,
                "cdp_listener": listener(2345, 9222),
                "cdp_listener_sample_count": 20,
                "cdp_listener_present_in_every_sample": True,
                "exact_loopback_allowances": True,
                "udp_observation_performed": False,
                "packet_capture_performed": False,
                "boundary": "sampled_exact_process_tree_tcp_only_no_udp_or_packet_capture",
                "observation": "sampled exact-process-tree TCP endpoint snapshots with a configured 250 ms inter-sample delay; only loopback CDP 9222 and QuickPLS proxy 17846 active paths allowed; zero non-loopback rows required; no UDP or packet-capture claim",
            },
            "browser_request_observation": browser_network(),
        },
        "cleanup": {
            "passed": True,
            "launched_pid": 1234,
            "descendants_at_shutdown": [],
            "graceful_close_exit_code": 0,
            "graceful_exit_confirmed": True,
            "forced_parent_termination": False,
            "forced_descendant_pids": [],
            "parent_exit_confirmed": True,
            "lingering_descendant_pids": [],
            "process_monitor": monitor_cleanup("process_tree"),
            "network_monitor": monitor_cleanup("network_tcp"),
        },
    }
    return {
        "schema_version": "quickpls.diagnostic_bundle_packaged_acceptance.v1",
        "kind": "quickpls3_packaged_diagnostic_bundle_v1_acceptance",
        "passed": True,
        "generated_at_utc": "2026-08-13T12:30:00.000Z",
        "target": "windows_10_11_x64_packaged_tauri",
        "runtime": {"node": "v22.0.0", "platform": "win32", "architecture": "x64", "playwright": "chromium-connect-over-cdp"},
        "endpoint": "http://127.0.0.1:9222",
        "generator": "validation/run_diagnostic_bundle_packaged_acceptance.ps1",
        "source_generator": "validation/diagnostic_bundle_packaged_acceptance.mjs",
        "tested_product": {"quickpls_desktop_exe": artifact("target/release/quickpls-desktop.exe")},
        "source_artifacts": source_artifacts(),
        "checks": checks,
        "artifacts": {
            "diagnostic_zip": artifact("validation/results/diagnostic.zip"),
            "live_preview_screenshot": artifact("validation/results/screens/diagnostic-bundle-packaged-acceptance/01-live.png"),
            "saved_screenshot": artifact("validation/results/screens/diagnostic-bundle-packaged-acceptance/02-saved.png"),
            "raw_report": artifact("validation/results/diagnostic.raw.json"),
            "process_samples": artifact("validation/results/process.jsonl"),
            "network_samples": artifact("validation/results/network.jsonl"),
            "process_report": artifact("validation/results/process.json"),
            "network_report": artifact("validation/results/network.json"),
            "cleanup_report": artifact("validation/results/cleanup.json"),
            "build_receipt": artifact("validation/results/diagnostic_bundle_build_receipt.json"),
            "source_before": artifact("validation/results/diagnostic_bundle_source_before.json"),
            "source_evidence": artifact("validation/results/diagnostic_bundle_source_evidence.json"),
        },
        "browser_requests": [
            {"url": "http://tauri.localhost/assets/app.js", "origin": "http://tauri.localhost", "method": "GET", "resource_type": "script"},
            {"url": "http://ipc.localhost/preview_diagnostic_bundle", "origin": "http://ipc.localhost", "method": "POST", "resource_type": "fetch"},
        ],
        "console_errors": [],
        "failures": [],
        "source_report": "validation/results/diagnostic_bundle_packaged_acceptance.raw.json",
    }


class DiagnosticBundlePackagedAcceptanceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        cls.validator = Draft202012Validator(cls.schema, format_checker=FormatChecker())
        cls.harness = HARNESS_PATH.read_text(encoding="utf-8")
        cls.helper_source = HELPER_PATH.read_text(encoding="utf-8")
        cls.wrapper = WRAPPER_PATH.read_text(encoding="utf-8")
        cls.network_monitor = NETWORK_MONITOR_PATH.read_text(encoding="utf-8")
        cls.process_monitor = PROCESS_MONITOR_PATH.read_text(encoding="utf-8")
        cls.validator_source = VALIDATOR_PATH.read_text(encoding="utf-8")
        cls.settings = SETTINGS_PATH.read_text(encoding="utf-8")
        cls.utility = UTILITY_PATH.read_text(encoding="utf-8")
        cls.app = APP_PATH.read_text(encoding="utf-8")
        cls.helper = load_helper()
        cls.report_validator = load_validator_module()
        cls.source_manifest = load_source_manifest_module()

    def assert_invalid(self, mutate) -> None:
        report = valid_report()
        mutate(report)
        self.assertTrue(list(self.validator.iter_errors(report)))

    def test_schema_is_draft_2020_and_mutations_fail_closed(self) -> None:
        Draft202012Validator.check_schema(self.schema)
        self.assertEqual(list(self.validator.iter_errors(valid_report())), [])
        mutations = [
            lambda report: report.update(passed=False),
            lambda report: report.update(target="unsigned_preview_only"),
            lambda report: report.pop("source_artifacts"),
            lambda report: report["source_artifacts"]["before"]["product_source"]["descriptors"].pop(),
            lambda report: report["source_artifacts"]["after"]["gate_only"]["descriptors"].pop(),
            lambda report: report["source_artifacts"]["after"]["gate_only"]["descriptors"][0].update(size=0),
            lambda report: report["source_artifacts"]["before"]["gate_only"]["descriptors"][0].update(sha256="not-a-sha256"),
            lambda report: report["source_artifacts"]["after"]["product_source"]["descriptors"][0].update(mtime_ns=0),
            lambda report: report["source_artifacts"]["build_receipt"].update(build_command=["npm.cmd", "run", "build"]),
            lambda report: report["source_artifacts"].update(source_stable_during_gate=False),
            lambda report: report["source_artifacts"]["freshness"].update(passed=False),
            lambda report: report["source_artifacts"]["freshness"].update(desktop_not_older_than_every_product_input=False),
            lambda report: report["checks"].pop("live_settings_preview"),
            lambda report: report["checks"]["runtime_preflight"].update(qualifying_page_count=2),
            lambda report: report["checks"]["abandoned_preview_recovery"].update(abandoned_preview_count=4),
            lambda report: report["checks"]["abandoned_preview_recovery"].update(evicted_oldest_count=3),
            lambda report: report["checks"]["live_settings_preview"].update(production_entry="prototype only"),
            lambda report: report["checks"]["live_settings_preview"].update(preview_before_save=False),
            lambda report: report["checks"]["live_settings_preview"].update(local_only=False),
            lambda report: report["checks"]["live_settings_preview"]["staged_contents"]["system"].update(userDataIncluded=True),
            lambda report: report["checks"]["live_settings_preview"]["staged_contents"]["manifest"].update(networkAccessed=True),
            lambda report: report["checks"]["navigation_cancellation"].update(unmount_cycles=4),
            lambda report: report["checks"]["navigation_cancellation"].update(every_unmount_recorded=False),
            lambda report: report["checks"]["native_save_dialog"].update(local_drive_rooted=False),
            lambda report: report["checks"]["native_save_dialog"]["helper_completion"].update(passed=False),
            lambda report: report["checks"]["native_save_dialog"]["helper_completion"]["boundControls"].update(filenameEditControlId=999),
            lambda report: report["checks"]["archive_integrity"].update(exact_entry_count=4),
            lambda report: report["checks"]["archive_integrity"].update(preview_archive_exact=False),
            lambda report: report["checks"]["archive_integrity"]["forbidden_pattern_matches"].append("email"),
            lambda report: report["checks"]["destination_rejections"].update(existing_destination_unchanged=False),
            lambda report: report["checks"]["destination_rejections"]["cases"].pop(),
            lambda report: report["checks"]["browser_network_observation"].update(external_request_count=1),
            lambda report: report["checks"]["browser_network_observation"].update(observed_origins=["http://tauri.localhost"]),
            lambda report: report["checks"]["process_observation"].update(peak_working_set_under_2_gib=False),
            lambda report: report["checks"]["process_observation"].update(process_command_lines_persisted=False),
            lambda report: report["checks"]["process_observation"].update(direct_webview_browser_child_count=2),
            lambda report: report["checks"]["process_observation"]["frozen_product_browser_switches"].remove("--disable-quic"),
            lambda report: report["checks"]["process_observation"]["missing_browser_switches"].append("--proxy-server=http://127.0.0.1:17846"),
            lambda report: report["checks"]["process_observation"]["conflicting_browser_switches"].append("--disable-quic=false"),
            lambda report: report["checks"]["network_observation"]["tcp_monitor"].update(remote_connection_count=1),
            lambda report: report["checks"]["network_observation"]["tcp_monitor"].update(proxy_listener_present_in_every_sample=False),
            lambda report: report["checks"]["network_observation"]["tcp_monitor"].update(exact_loopback_allowances=False),
            lambda report: report["checks"]["network_observation"]["tcp_monitor"].update(udp_observation_performed=True),
            lambda report: report["checks"]["network_observation"]["tcp_monitor"].update(packet_capture_performed=True),
            lambda report: report["checks"]["cleanup"].update(forced_parent_termination=True),
            lambda report: report["checks"]["cleanup"]["network_monitor"].update(exit_code=1),
            lambda report: report["console_errors"].append("boom"),
            lambda report: report["failures"].append("boom"),
        ]
        for mutate in mutations:
            with self.subTest(mutation=mutate):
                self.assert_invalid(mutate)

    def _temporary_source_evidence(self, root: Path) -> tuple[dict, Path, Path, Path]:
        module = self.source_manifest
        text_files = set(module.FIXED_PRODUCT_PATHS + module.REQUIRED_EXTERNAL_PRODUCT_PATHS + module.GATE_ONLY_PATHS)
        text_files.update({
            "vite.config.js", "vite.config.ts", "src/App.tsx", "src/App.test.tsx",
            "src-tauri/src/main.rs", "crates/demo/src/lib.rs", "src-tauri/capabilities/default.json",
        })
        for index, relative in enumerate(sorted(text_files)):
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(f"fixture-{index}-{relative}\n", encoding="utf-8")

        (root / "Cargo.toml").write_text("[workspace]\nmembers = [\"crates/demo\", \"src-tauri\"]\n", encoding="utf-8")
        (root / "package.json").write_text(
            json.dumps(
                {
                    "scripts": {
                        "build": "npm run typecheck:build && npm run build:bundle",
                        "tauri": "tauri",
                    }
                }
            ),
            encoding="utf-8",
        )
        (root / "crates/demo/Cargo.toml").write_text("[package]\nname='demo'\nversion='1.0.0'\n", encoding="utf-8")
        (root / "src-tauri/Cargo.toml").write_text(
            "[package]\nname='quickpls-desktop'\nversion='1.0.0'\n[dependencies]\ndemo={path='../crates/demo'}\n",
            encoding="utf-8",
        )
        icon = root / "src-tauri/icons/icon.ico"
        icon.parent.mkdir(parents=True, exist_ok=True)
        icon.write_bytes(b"icon")
        (root / "src-tauri/tauri.conf.json").write_text(json.dumps({
            "build": {"beforeBuildCommand": "npm run build", "frontendDist": "../dist"},
            "bundle": {
                "icon": ["icons/icon.ico"],
                "resources": {"../THIRD_PARTY_NOTICES.md": "THIRD_PARTY_NOTICES.md"},
            }
        }), encoding="utf-8")
        dist = root / "dist/index.html"
        dist.parent.mkdir(parents=True, exist_ok=True)
        dist.write_text("<html>frozen</html>", encoding="utf-8")
        desktop = root / module.DESKTOP_PATH
        desktop.parent.mkdir(parents=True, exist_ok=True)
        desktop.write_bytes(b"frozen-desktop")
        dep_info = root / module.DEP_INFO_PATH
        dep_info_dependencies = [
            dist,
            root / "src-tauri/capabilities",
            *(root / relative for relative in module.REQUIRED_EXTERNAL_PRODUCT_PATHS),
            root / "src-tauri/src/main.rs",
            root / "crates/demo/src/lib.rs",
        ]
        dep_info.write_text(f"{desktop}: {' '.join(str(path) for path in dep_info_dependencies)}\n", encoding="utf-8")

        now = time.time_ns()
        source_ns = now - 30_000_000_000
        desktop_ns = now - 10_000_000_000
        gate_ns = now - 5_000_000_000
        for path in root.rglob("*"):
            if path.is_file():
                os.utime(path, ns=(source_ns, source_ns))
        os.utime(dist, ns=(source_ns, source_ns))
        os.utime(dep_info, ns=(desktop_ns, desktop_ns))
        os.utime(desktop, ns=(desktop_ns, desktop_ns))
        for relative in module.GATE_ONLY_PATHS:
            os.utime(root / relative, ns=(gate_ns, gate_ns))

        product = module.capture_product_source(root)
        dist_manifest = module.capture_dist(root)
        dep_info_manifest = module.capture_dep_info(root)
        desktop_descriptor = module.describe_file(module.DESKTOP_PATH, root)
        build_started = desktop_ns - 1_000_000_000
        build_finished = desktop_ns + 1_000_000_000
        freshness = module.source_freshness(
            product, dist_manifest, desktop_descriptor,
            build_started_unix_ns=build_started,
            build_finished_unix_ns=build_finished,
        )
        receipt = {
            "schema_version": module.RECEIPT_SCHEMA,
            "kind": "quickpls_diagnostic_packaged_frozen_build_v1",
            "passed": True,
            "generated_at_utc": "2026-08-13T12:20:00Z",
            "build_command": list(module.BUILD_COMMAND),
            "build_started_at_utc": "2026-08-13T12:00:00Z",
            "build_finished_at_utc": "2026-08-13T12:10:00Z",
            "build_started_unix_ns": build_started,
            "build_finished_unix_ns": build_finished,
            "build_exit_code": 0,
            "source_before": product,
            "source_after": deepcopy(product),
            "source_stable_during_build": True,
            "dist_after": dist_manifest,
            "cargo_dep_info": dep_info_manifest,
            "tested_desktop": desktop_descriptor,
            "dist_bound_to_dep_info": True,
            "freshness": freshness,
        }
        receipt_path = root / module.RECEIPT_PATH
        module.write_json(receipt_path, receipt)
        before = module.capture_gate_snapshot(receipt_path, root)
        evidence = {
            "schema_version": module.EVIDENCE_SCHEMA,
            "discovery_contract": module.DISCOVERY_CONTRACT,
            "build_receipt_path": module.RECEIPT_PATH,
            "build_receipt": receipt,
            "before": before,
            "after": deepcopy(before),
            "source_stable_during_gate": True,
            "freshness": freshness,
        }
        module.validate_gate_evidence(evidence, root)
        return evidence, desktop, root / "src/App.tsx", receipt_path

    def test_source_discovery_closes_product_and_exact_gate_only_sets(self) -> None:
        discovery = self.source_manifest.discover_product_source(ROOT)
        paths = discovery["paths"]
        self.assertIn("src/components/SettingsWorkspace.tsx", paths)
        self.assertIn("src/components/SettingsWorkspace.test.tsx", paths)
        self.assertIn("src-tauri/src/sample_projects.rs", paths)
        self.assertIn("validation/fixtures/mediation_sample.csv", paths)
        self.assertIn("vite.config.js", paths)
        self.assertIn("vite.config.ts", paths)
        self.assertNotIn("vite.config.d.ts", paths)
        self.assertNotIn("crates/qpls-cli/Cargo.toml", discovery["desktop_cargo_manifests"])
        self.assertEqual(discovery["active_vite_config"], "vite.config.js")
        self.assertEqual(list(self.source_manifest.discover_gate_only(ROOT)), GATE_ONLY_PATHS)
        self.assertFalse(set(paths).intersection(GATE_ONLY_PATHS))

    def test_gate_only_sources_may_be_newer_than_the_frozen_desktop_but_are_gate_stable(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            evidence, _desktop, _source, _receipt = self._temporary_source_evidence(root)
            desktop_mtime = evidence["after"]["tested_desktop"]["mtime_ns"]
            self.assertTrue(all(row["mtime_ns"] > desktop_mtime for row in evidence["after"]["gate_only"]["descriptors"]))
            self.assertTrue(evidence["freshness"]["passed"])
            self.source_manifest.validate_gate_evidence(evidence, root)

    def test_final_source_manifest_rejects_hash_size_removal_extra_change_and_staleness(self) -> None:
        def wrong_hash(evidence, _desktop, _source, _receipt, _root):
            evidence["after"]["product_source"]["descriptors"][0]["sha256"] = "0" * 64

        def wrong_size(evidence, _desktop, _source, _receipt, _root):
            evidence["after"]["gate_only"]["descriptors"][0]["size"] += 1

        def removed_descriptor(evidence, _desktop, _source, _receipt, _root):
            evidence["after"]["product_source"]["descriptors"].pop()

        def extra_descriptor(evidence, _desktop, _source, _receipt, _root):
            evidence["after"]["product_source"]["descriptors"].append(source_descriptor("src/not-discovered.ts"))

        def reordered_descriptors(evidence, _desktop, _source, _receipt, _root):
            evidence["after"]["product_source"]["descriptors"].reverse()

        def manifest_digest_tamper(evidence, _desktop, _source, _receipt, _root):
            evidence["after"]["product_source"]["manifest_sha256"] = "0" * 64

        def changed_during_gate(evidence, _desktop, _source, _receipt, _root):
            evidence["after"]["gate_only"]["descriptors"][0]["sha256"] = "f" * 64

        def removed_source(_evidence, _desktop, source, _receipt, _root):
            source.unlink()

        def identical_size_replacement(_evidence, _desktop, source, _receipt, _root):
            original = source.read_bytes()
            source.write_bytes(b"X" * len(original))

        def stale_desktop(_evidence, desktop, _source, _receipt, _root):
            stale = time.time_ns() - 60_000_000_000
            os.utime(desktop, ns=(stale, stale))

        def new_untracked_source(_evidence, _desktop, _source, _receipt, root):
            (root / "src/new-untracked.ts").write_text("export {};\n", encoding="utf-8")

        def tampered_receipt_command(evidence, _desktop, _source, receipt, _root):
            tampered = deepcopy(evidence["build_receipt"])
            tampered["build_command"] = ["npm.cmd", "run", "build"]
            self.source_manifest.write_json(receipt, tampered)
            evidence["build_receipt"] = tampered

        mutations = {
            "hash": wrong_hash,
            "size": wrong_size,
            "descriptor_removal": removed_descriptor,
            "descriptor_extra": extra_descriptor,
            "descriptor_reordered": reordered_descriptors,
            "manifest_digest": manifest_digest_tamper,
            "changed_during_gate": changed_during_gate,
            "source_removed": removed_source,
            "identical_size_content_change": identical_size_replacement,
            "desktop_stale": stale_desktop,
            "new_untracked_source": new_untracked_source,
            "unsupported_build_command": tampered_receipt_command,
        }
        for label, mutate in mutations.items():
            with self.subTest(label=label), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                evidence, desktop, source, receipt = self._temporary_source_evidence(root)
                mutate(evidence, desktop, source, receipt, root)
                with self.assertRaises(self.source_manifest.SourceManifestFailure):
                    self.source_manifest.validate_gate_evidence(evidence, root)

    def test_production_preferences_mounts_one_shared_panel_with_unmount_cancellation(self) -> None:
        self.assertIn('return <NativeDesktopApp />;', self.app)
        self.assertIn('import { DiagnosticBundlePanel } from "../components/SettingsWorkspace";', self.utility)
        self.assertIn('<DiagnosticBundlePanel />', self.utility)
        self.assertIn('data-live-preferences-dialog="true"', self.utility)
        self.assertIn('export function DiagnosticBundlePanel()', self.settings)
        self.assertIn('data-diagnostic-bundle-panel="live"', self.settings)
        self.assertIn('const previewId = diagnosticPreviewIdRef.current;', self.settings)
        self.assertIn('void cancelNativeDiagnosticBundlePreview(previewId).catch(() => undefined);', self.settings)
        self.assertRegex(
            self.settings,
            re.compile(r"useEffect\(\(\) => \(\) => \{[\s\S]+cancelNativeDiagnosticBundlePreview\(previewId\)[\s\S]+\}, \[\]\);"),
        )
        self.assertEqual(self.utility.count("<DiagnosticBundlePanel />"), 1)

    def test_harness_covers_live_ui_bounded_recovery_save_and_negative_paths(self) -> None:
        required = [
            'page.getByRole("menubar", { name: "Application menu" })',
            'applicationMenu.getByRole("menuitem", { name: "Tools", exact: true })',
            'page.getByRole("menu", { name: "Tools", exact: true })',
            '.getByRole("menuitem", { name: /^Preferences/ })',
            '[data-live-preferences-dialog="true"] [data-diagnostic-bundle-panel="live"]',
            'for (let index = 0; index < 7; index += 1)',
            'abandonedPreviews.slice(0, 4)',
            'abandonedPreviews.slice(4)',
            'for (let cycle = 1; cycle <= 5; cycle += 1)',
            'cancellationEventCount(navigationPreview)',
            'name: "Close dialog"',
            'startNativeSaveHelper(targetZip, await page.title())',
            'name: /^Save new ZIP/',
            'canonicalJson(finalPreview.stagedContents.system) === canonicalJson(bundle.system)',
            'canonicalJson(finalPreview.stagedContents.events) === canonicalJson(bundle.events)',
            'canonicalJson(finalPreview.stagedContents.manifest) === canonicalJson(bundle.manifest)',
            'preview_archive_exact: previewArchiveExact',
            '"relative", "relative.zip", "DIAGNOSTIC_PATH_NOT_LOCAL_DRIVE"',
            '"unc", "\\\\\\\\server\\\\share\\\\bundle.zip", "DIAGNOSTIC_PATH_NAMESPACE_BLOCKED"',
            '"existing_destination", temporaryExistingPath, "DIAGNOSTIC_DESTINATION_EXISTS"',
            'existingAfter.equals(existingSentinel)',
            'external_request_count: externalBrowserRequests.length',
        ]
        for token in required:
            self.assertIn(token, self.harness)
        self.assertNotIn("quickpls_external_beta", self.harness)
        self.assertNotIn("quickpls_signed_candidate", self.harness)

    def test_canonical_preview_equality_ignores_object_order_but_not_values_or_array_order(self) -> None:
        match = re.search(r"function canonicalJson\(value\) \{[\s\S]+?\n\}", self.harness)
        self.assertIsNotNone(match)
        script = match.group(0) + r'''
const left = { system: { schemaVersion: 1, networkAccessed: false }, events: [{ timestamp: "t", sequence: 1 }], manifest: { entries: [{ name: "a", bytes: 1 }] } };
const reordered = { manifest: { entries: [{ bytes: 1, name: "a" }] }, events: [{ sequence: 1, timestamp: "t" }], system: { networkAccessed: false, schemaVersion: 1 } };
const changed = structuredClone(reordered); changed.system.networkAccessed = true;
const arrayChanged = structuredClone(reordered); arrayChanged.events = [{ sequence: 2, timestamp: "later" }, ...arrayChanged.events];
if (canonicalJson(left) !== canonicalJson(reordered)) process.exit(1);
if (canonicalJson(left) === canonicalJson(changed)) process.exit(2);
if (canonicalJson(left) === canonicalJson(arrayChanged)) process.exit(3);
'''
        completed = subprocess.run(
            ["node", "--input-type=module", "-e", script],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)

    def test_ui_ipc_capture_is_direct_exact_prearmed_and_fail_closed(self) -> None:
        def assert_capture_contract(source: str) -> None:
            required = [
                'const expectedUrl = `http://ipc.localhost/${encodeURIComponent(command)}`;',
                'request.url() === expectedUrl',
                'request.method() === "POST"',
                'request.resourceType() === "fetch"',
                'page.waitForResponse(matchesResponse, { timeout })',
                'matchingRequests.length === 1',
                'matchingResponses.length === 1',
                'response.request() === matchingRequests[0]',
                'response.request().postDataJSON()',
                'canonicalJson(requestPayload) === canonicalJson(expectedRequestPayload)',
                'assert(response.ok()',
                'headers["tauri-response"] === "ok"',
                'contentType === "application/json"',
                'return await response.json()',
                '"preview_diagnostic_bundle",\n    { replacesPreviewId: null },',
                'return requireDiagnosticPreviewPayload(preview);',
                '"save_diagnostic_bundle",\n    { path: targetZip, previewId: finalPreview.previewId },',
            ]
            for token in required:
                self.assertIn(token, source)
            self.assertLess(
                source.index('const responsePromise = page.waitForResponse(matchesResponse, { timeout });'),
                source.index('Promise.resolve().then(action)'),
            )
            preview_function = re.search(
                r"async function previewFromUi\(dialog, page\) \{([\s\S]+?)\n\}",
                source,
            )
            self.assertIsNotNone(preview_function)
            self.assertIn("captureExactIpcJsonForAction", preview_function.group(1))
            self.assertNotIn("invokeNative", preview_function.group(1))
            self.assertNotIn("installInvokeRecorder", source)
            self.assertNotIn("lastRecordedResult", source)
            self.assertNotIn("__quickplsDiagnosticInvocations", source)

        assert_capture_contract(self.harness)
        mutations = {
            "wrong_ipc_origin": self.harness.replace(
                '`http://ipc.localhost/${encodeURIComponent(command)}`',
                '`http://tauri.localhost/${encodeURIComponent(command)}`',
                1,
            ),
            "missing_response_timeout": self.harness.replace(
                'page.waitForResponse(matchesResponse, { timeout })',
                'page.waitForEvent("response")',
                1,
            ),
            "ambiguous_request_allowed": self.harness.replace(
                "matchingRequests.length === 1",
                "matchingRequests.length >= 1",
                1,
            ),
            "ambiguous_response_allowed": self.harness.replace(
                "matchingResponses.length === 1",
                "matchingResponses.length >= 1",
                1,
            ),
            "request_arguments_unbound": self.harness.replace(
                "canonicalJson(requestPayload) === canonicalJson(expectedRequestPayload)",
                "Boolean(requestPayload)",
                1,
            ),
            "http_error_allowed": self.harness.replace(
                "assert(response.ok()",
                "assert(true",
                1,
            ),
            "tauri_error_allowed": self.harness.replace(
                'headers["tauri-response"] === "ok"',
                "Boolean(headers)",
                1,
            ),
            "wrong_content_type_allowed": self.harness.replace(
                'contentType === "application/json"',
                "Boolean(contentType)",
                1,
            ),
            "response_body_not_decoded": self.harness.replace(
                "return await response.json()",
                "return {}",
                1,
            ),
            "fabricated_preview": self.harness.replace(
                "return requireDiagnosticPreviewPayload(preview);",
                'return invokeNative(page, "preview_diagnostic_bundle", { replacesPreviewId: null });',
                1,
            ),
        }
        for label, mutated in mutations.items():
            with self.subTest(label=label), self.assertRaises(AssertionError):
                assert_capture_contract(mutated)

    def test_live_tools_locator_and_empty_monitor_stderr_regressions_fail_closed(self) -> None:
        def assert_locator_contract(source: str) -> None:
            self.assertIn('page.getByRole("menubar", { name: "Application menu" })', source)
            self.assertIn('applicationMenu.getByRole("menuitem", { name: "Tools", exact: true })', source)
            self.assertIn('page.getByRole("menu", { name: "Tools", exact: true })', source)
            self.assertNotIn('getByRole("button", { name: "Tools", exact: true })', source)

        def assert_empty_stderr_contract(source: str) -> None:
            self.assertIn('[System.IO.File]::ReadAllText(', source)
            self.assertIn('[System.Text.Encoding]::UTF8', source)
            self.assertNotIn('([string](Get-Content -LiteralPath $StderrPath -Raw -Encoding UTF8)).Trim()', source)

        assert_locator_contract(self.harness)
        assert_empty_stderr_contract(self.wrapper)
        mutations = {
            "implicit_button_role": (
                self.harness.replace(
                    'applicationMenu.getByRole("menuitem", { name: "Tools", exact: true })',
                    'applicationMenu.getByRole("button", { name: "Tools", exact: true })',
                    1,
                ),
                assert_locator_contract,
            ),
            "unscoped_tools_trigger": (
                self.harness.replace(
                    'applicationMenu.getByRole("menuitem", { name: "Tools", exact: true })',
                    'page.getByRole("menuitem", { name: "Tools", exact: true })',
                    1,
                ),
                assert_locator_contract,
            ),
            "empty_get_content_trim": (
                self.wrapper.replace(
                    '$result.stderr = [System.IO.File]::ReadAllText(\n'
                    '            $StderrPath,\n'
                    '            [System.Text.Encoding]::UTF8\n'
                    '        ).Trim()',
                    '$result.stderr = ([string](Get-Content -LiteralPath $StderrPath -Raw -Encoding UTF8)).Trim()',
                    1,
                ),
                assert_empty_stderr_contract,
            ),
        }
        for label, (mutated, contract) in mutations.items():
            with self.subTest(label=label), self.assertRaises(AssertionError):
                contract(mutated)

    def test_native_helper_and_wrapper_are_exact_pid_and_never_broad_kill(self) -> None:
        helper_tokens = [
            'EXPECTED_EXECUTABLE = "quickpls-desktop.exe"',
            'class_name="#32770"',
            'control.class_name() == "Edit" and int(control.control_id()) == 1001',
            'control.class_name() == "Button" and int(control.control_id()) == 1',
            'save_control.send_message(button_click_message, 0, 0)',
            'EXPECTED_ENTRIES = (',
            'info.compress_type != zipfile.ZIP_STORED',
            'hashlib.sha256(payload).hexdigest()',
            'forbiddenPatternMatches',
        ]
        for token in helper_tokens:
            self.assertIn(token, self.helper_source)
        wrapper_tokens = [
            'validation\\monitor_quickpls_process_tree.ps1',
            'validation\\monitor_quickpls_network.ps1',
            'Get-ExactDescendantProcesses -RootProcessId $application.Id',
            '& node .\\validation\\close_tauri_test_window.mjs',
            'Stop-Process -Id $application.Id -Force',
            '$cleanup.forced_parent_termination',
            '$cleanup.lingering_descendant_pids',
            '$remoteConnections.Count -eq 0',
            '$processCommandLinesPersisted',
            '$directBrowserIdentityKeys.Count -eq 1',
            '"--proxy-server=http://127.0.0.1:17846"',
            '"--disable-quic"',
            '$proxyListenerPresentInEverySample',
            '$cdpListenerPresentInEverySample',
            '$unexpectedLoopbackConnections.Count -eq 0',
            'diagnostic_bundle_source_manifest.py snapshot --receipt $buildReceiptPath --output $sourceBeforePath',
            'diagnostic_bundle_source_manifest.py finish-gate --receipt $buildReceiptPath --before $sourceBeforePath --output $sourceEvidencePath',
            'diagnostic_bundle_packaged_acceptance.py --report $finalReportPath',
        ]
        for token in wrapper_tokens:
            self.assertIn(token, self.wrapper)
        self.assertNotRegex(self.wrapper, re.compile(r"Stop-Process\s+-Name", re.IGNORECASE))
        self.assertNotIn("quickpls_external_beta", self.wrapper)
        self.assertNotIn("quickpls_signed_candidate", self.wrapper)

    def test_network_monitor_is_exact_tree_sampled_and_excludes_only_loopback(self) -> None:
        required = [
            '$rootCreationDate = [string]$rootDescriptor.CreationDate',
            '[string]$_.CreationDate -eq $rootCreationDate',
            'Get-NetTCPConnection -ErrorAction Stop',
            'remote_access = [bool]$remoteAccess',
            'remote_connections = @($connections | Where-Object { $_.remote_access -eq $true })',
            'sampled_exact_process_tree_tcp_v1',
            '$normalized.StartsWith("127.")',
            '$normalized -eq "::1"',
        ]
        for token in required:
            self.assertIn(token, self.network_monitor)
        for token in ('executable_path = [string]$descriptor.ExecutablePath', 'command_line = [string]$descriptor.CommandLine', 'metrics_available = [bool]$live'):
            self.assertIn(token, self.process_monitor)
        self.assertNotIn('if ($live) {\n            $processes +=', self.process_monitor)
        self.assertNotIn("Get-NetUDPEndpoint", self.network_monitor)
        self.assertIn('udp_observation_performed = $false', self.wrapper)
        self.assertIn('packet_capture_performed = $false', self.wrapper)
        self.assertIn('sampled_exact_process_tree_tcp_only_no_udp_or_packet_capture', self.wrapper)

    def test_runtime_evidence_validator_rejects_command_line_listener_and_tcp_mutations(self) -> None:
        process_report = valid_report()["checks"]["process_observation"]
        process_samples = runtime_process_samples()
        browser_pid = self.report_validator.validate_runtime_process_evidence(process_report, process_samples, 1234)
        self.assertEqual(browser_pid, 2345)

        process_mutations = {
            "missing_command_line": lambda samples: samples[0]["processes"][1].update(command_line=""),
            "missing_proxy_switch": lambda samples: samples[0]["processes"][1].update(
                command_line=samples[0]["processes"][1]["command_line"].replace(" --proxy-server=http://127.0.0.1:17846", "")
            ),
            "missing_quic_switch": lambda samples: samples[0]["processes"][1].update(
                command_line=samples[0]["processes"][1]["command_line"].replace(" --disable-quic", "")
            ),
            "duplicate_proxy_switch": lambda samples: samples[0]["processes"][1].update(
                command_line=samples[0]["processes"][1]["command_line"] + " --proxy-server=http://127.0.0.1:17846"
            ),
            "conflicting_quic_switch": lambda samples: samples[0]["processes"][1].update(
                command_line=samples[0]["processes"][1]["command_line"] + " --disable-quic=false"
            ),
            "second_direct_browser": lambda samples: samples[0]["processes"].append({
                **deepcopy(samples[0]["processes"][1]), "pid": 3456,
            }),
        }
        for label, mutate in process_mutations.items():
            with self.subTest(label=label):
                mutated = runtime_process_samples()
                mutate(mutated)
                with self.assertRaises(self.report_validator.GateFailure):
                    self.report_validator.validate_runtime_process_evidence(process_report, mutated, 1234)

        tcp_report = valid_report()["checks"]["network_observation"]["tcp_monitor"]
        self.report_validator.validate_runtime_tcp_evidence(tcp_report, runtime_network_samples(), 1234, 2345)
        tcp_mutations = {
            "proxy_wrong_owner": lambda samples: samples[0]["connections"][0].update(owning_process=2345),
            "proxy_listener_missing": lambda samples: samples[0]["connections"].pop(0),
            "unexpected_loopback_port": lambda samples: samples[0]["connections"].append({
                "owning_process": 2345, "local_address": "127.0.0.1", "local_port": 50000,
                "remote_address": "127.0.0.1", "remote_port": 80, "state": "SynSent", "remote_access": False,
            }),
            "non_loopback_tcp": lambda samples: samples[0]["connections"].append({
                "owning_process": 2345, "local_address": "10.1.1.2", "local_port": 50001,
                "remote_address": "52.110.15.135", "remote_port": 443, "state": "Established", "remote_access": True,
            }),
        }
        for label, mutate in tcp_mutations.items():
            with self.subTest(label=label):
                mutated = runtime_network_samples()
                mutate(mutated)
                with self.assertRaises(self.report_validator.GateFailure):
                    self.report_validator.validate_runtime_tcp_evidence(tcp_report, mutated, 1234, 2345)

    def test_runtime_jsonl_parser_rejects_blank_and_malformed_rows(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "samples.jsonl"
            path.write_text('{"root_pid":1234}\n', encoding="utf-8")
            self.assertEqual(self.report_validator.read_jsonl(path, "samples"), [{"root_pid": 1234}])
            for payload in ('{"root_pid":1234}\n\n', '{"root_pid":1234}\nnot-json\n'):
                path.write_text(payload, encoding="utf-8")
                with self.assertRaises((self.report_validator.GateFailure, json.JSONDecodeError)):
                    self.report_validator.read_jsonl(path, "samples")

    def test_helper_verifies_a_real_fixed_three_entry_stored_bundle(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            target = root / "bundle.zip"
            metadata_bytes = json.dumps(system_metadata(), indent=2).encode("utf-8")
            events_bytes = ("\n".join(json.dumps(row, separators=(",", ":")) for row in event_rows()) + "\n").encode("utf-8")
            fixture_manifest = manifest()
            fixture_manifest["entries"] = [
                {"name": "metadata/system.json", "sha256": sha256(metadata_bytes).hexdigest(), "bytes": len(metadata_bytes)},
                {"name": "logs/events.jsonl", "sha256": sha256(events_bytes).hexdigest(), "bytes": len(events_bytes)},
            ]
            manifest_bytes = json.dumps(fixture_manifest, indent=2).encode("utf-8")
            with zipfile.ZipFile(target, "w", compression=zipfile.ZIP_STORED) as archive:
                archive.writestr("metadata/system.json", metadata_bytes)
                archive.writestr("logs/events.jsonl", events_bytes)
                archive.writestr("manifest.json", manifest_bytes)

            inspected = self.helper.verify_bundle(target)

            self.assertEqual(inspected["entryNames"], ["metadata/system.json", "logs/events.jsonl", "manifest.json"])
            self.assertEqual(inspected["forbiddenPatternMatches"], [])
            self.assertEqual(inspected["manifest"]["entries"], fixture_manifest["entries"])

            with self.assertRaisesRegex(self.helper.GateFailure, "already exists"):
                self.helper.validate_target(str(target), str(root))
            valid_new, _ = self.helper.validate_target(str(root / "new.zip"), str(root))
            self.assertEqual(valid_new.name, "new.zip")
            self.assertTrue(valid_new.parent.samefile(root))
            with tempfile.TemporaryDirectory() as outside:
                with self.assertRaisesRegex(self.helper.GateFailure, "inside validation/results"):
                    self.helper.validate_target(str(Path(outside) / "outside.zip"), str(root))

    def test_helper_rejects_extra_entries_and_descriptor_tampering(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            metadata_bytes = json.dumps(system_metadata()).encode("utf-8")
            events_bytes = (json.dumps(event_rows()[0]) + "\n").encode("utf-8")
            fixture_manifest = manifest()
            fixture_manifest["entries"] = [
                {"name": "metadata/system.json", "sha256": sha256(metadata_bytes).hexdigest(), "bytes": len(metadata_bytes)},
                {"name": "logs/events.jsonl", "sha256": sha256(events_bytes).hexdigest(), "bytes": len(events_bytes)},
            ]
            extra = root / "extra.zip"
            with zipfile.ZipFile(extra, "w", compression=zipfile.ZIP_STORED) as archive:
                archive.writestr("metadata/system.json", metadata_bytes)
                archive.writestr("logs/events.jsonl", events_bytes)
                archive.writestr("manifest.json", json.dumps(fixture_manifest))
                archive.writestr("extra.txt", b"no")
            with self.assertRaisesRegex(self.helper.GateFailure, "entry order/set"):
                self.helper.verify_bundle(extra)

            tampered = root / "tampered.zip"
            fixture_manifest["entries"][0]["sha256"] = "0" * 64
            with zipfile.ZipFile(tampered, "w", compression=zipfile.ZIP_STORED) as archive:
                archive.writestr("metadata/system.json", metadata_bytes)
                archive.writestr("logs/events.jsonl", events_bytes)
                archive.writestr("manifest.json", json.dumps(fixture_manifest))
            with self.assertRaisesRegex(self.helper.GateFailure, "descriptor mismatch"):
                self.helper.verify_bundle(tampered)

    def test_final_validator_rechecks_artifacts_and_zip_instead_of_trusting_passed_flags(self) -> None:
        required = [
            "Draft202012Validator.check_schema(schema)",
            "verify_artifact(artifact, name)",
            'validate_gate_evidence(report["source_artifacts"], ROOT)',
            'report["source_artifacts"]["after"]["tested_desktop"]',
            'save_result["archiveSha256"] == archive["archive_sha256"] == zip_artifact["sha256"]',
            'checks["live_settings_preview"]["staged_contents"]',
            'native_save["helper_ready"]["mainWindow"]["pid"] == root_pid',
            'tuple(info.filename for info in infos) == EXPECTED_ENTRIES',
            'info.compress_type == zipfile.ZIP_STORED',
            'descriptor["sha256"] == sha256_bytes(payload)',
        ]
        for token in required:
            self.assertIn(token, self.validator_source)


if __name__ == "__main__":
    unittest.main()
