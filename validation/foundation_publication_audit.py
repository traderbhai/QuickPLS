"""Publication audit for v0.1 foundation offline/recovery readiness."""

import json
import os
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "foundation_publication_audit.json"
NPM = "npm.cmd" if os.name == "nt" else "npm"


def run(command):
    proc = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=240)
    return {
        "command": command,
        "returncode": proc.returncode,
        "passed": proc.returncode == 0,
        "stdout_tail": proc.stdout[-4000:],
        "stderr_tail": proc.stderr[-4000:],
    }


def read_json(path):
    return json.loads((ROOT / path).read_text(encoding="utf-8"))


def scan_sources():
    disallowed_terms = ["localStorage", "sessionStorage", "fetch(", "XMLHttpRequest", "telemetry", "activation"]
    findings = []
    for base in ["src", "src-tauri/src", "crates"]:
        for path in (ROOT / base).rglob("*"):
            if path.suffix not in {".rs", ".ts", ".tsx", ".json", ".toml"}:
                continue
            text = path.read_text(encoding="utf-8", errors="ignore")
            for term in disallowed_terms:
                if term in text:
                    findings.append({"path": str(path.relative_to(ROOT)), "term": term})
    return findings


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    tauri = read_json("src-tauri/tauri.conf.json")
    capability = read_json("src-tauri/capabilities/default.json")
    build = tauri.get("build", {})
    csp = tauri.get("app", {}).get("security", {}).get("csp", "")
    checks = {
        "frontend_dist_configured": build.get("frontendDist") == "../dist",
        "dev_url_is_dev_only_localhost": build.get("devUrl") == "http://localhost:1420",
        "csp_only_self_ipc_and_data": "default-src 'self'" in csp and "connect-src ipc: http://ipc.localhost" in csp,
        "capabilities_no_remote_urls": "remote" not in json.dumps(capability).lower() and "urls" not in capability,
        "permissions_are_local_desktop_only": set(capability.get("permissions", [])) <= {"core:default", "dialog:allow-open", "dialog:allow-save"},
    }
    source_findings = scan_sources()
    command_checks = [
        run([NPM, "run", "build"]),
        run(["cargo", "test", "-p", "quickpls-desktop"]),
        run(["cargo", "test", "-p", "qpls-project"]),
    ]
    required_tests = [
        "cancellation_wins_before_commit_and_does_not_persist",
        "desktop_runner_payload_matches_cli_serialized_artifact",
        "valid_autosave_takes_precedence_and_can_be_discarded",
        "previous_generation_recovers_a_corrupt_primary_archive",
        "stale_autosave_does_not_replace_a_newer_explicit_save",
    ]
    combined_output = "\n".join((item["stdout_tail"] + item["stderr_tail"]) for item in command_checks)
    test_coverage = {name: name in combined_output for name in required_tests}
    passed = (
        all(checks.values())
        and not source_findings
        and all(item["passed"] for item in command_checks)
        and all(test_coverage.values())
        and (ROOT / "dist" / "index.html").exists()
    )
    report = {
        "schema_version": 1,
        "target": "v0.1 foundation publication audit",
        "passed": passed,
        "static_checks": checks,
        "source_findings": source_findings,
        "test_coverage": test_coverage,
        "command_checks": command_checks,
        "built_frontend_index": str((ROOT / "dist" / "index.html")),
        "note": "Allows dev-only localhost and Tauri IPC localhost; rejects runtime telemetry, activation, browser localStorage/sessionStorage, and remote computation hooks.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
