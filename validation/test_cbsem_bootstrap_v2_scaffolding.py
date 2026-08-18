from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def test_method_audit_is_present_and_fail_closed_while_manifest_is_absent() -> None:
    completed = subprocess.run(
        [sys.executable, str(ROOT / "validation" / "cbsem_bootstrap_v2_method_promotion_audit.py")],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    assert completed.returncode == 1
    assert '"passed": false' in completed.stdout.lower()
    assert "manifest_declared_state_absent" in completed.stdout
    assert "coordinated_witness_semantic_rewrite_not_replayed" in completed.stdout


def test_persistence_gate_blocks_coordinated_witness_rewrite_without_ml_replay() -> None:
    completed = subprocess.run(
        [sys.executable, str(ROOT / "validation" / "cbsem_bootstrap_v2_release_persistence_gate.py")],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    assert completed.returncode == 1
    report = json.loads(
        (ROOT / "validation/results/cbsem_bootstrap_v2_release_persistence.json")
        .read_text(encoding="utf-8")
    )
    assert report["passed"] is False
    assert report["structural_checks_passed"] is True
    assert report["semantic_replay_authentication"] == "not_implemented"
    assert "coordinated_witness_semantic_rewrite_not_replayed" in report["blockers"]


def test_packaged_wrapper_guards_before_any_gui_launch() -> None:
    source = (ROOT / "validation" / "run_v252_cbsem_bootstrap_v2_native_acceptance.ps1").read_text(encoding="utf-8")
    guard = source.index("if (-not $RunQualifiedAcceptance)")
    state_guard = source.index("$declaredState -notin")
    launch = source.index("Start-Process")
    assert guard < state_guard < launch
    assert "Future-only" in source
    assert "native_qualified" in source and "release_qualified" in source
