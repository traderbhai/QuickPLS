#!/usr/bin/env python3
"""Invoke the validation-only R/cSEM General SEM Rank 0 oracle.

The wrapper supplies deterministic row-index bootstrap plans so R and Python
can evaluate identical case samples without pretending that their RNGs are
equivalent.  It never imports QuickPLS product code and never writes evidence;
callers decide whether a completed comparison is eligible for a later,
source-bound qualification receipt.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Literal

from general_sem_rank0_independent_pls_oracle import _replicate_indices
from general_sem_rank0_qualification import GeneralSemScenario


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "validation" / "general_sem_rank0_csem_oracle.R"
DEFAULT_WINDOWS_RSCRIPT = Path(
    r"C:\Users\mohd.naved\AppData\Local\Programs\R\R-4.6.1\bin\x64\Rscript.exe"
)

Operation = Literal[
    "mediation_point",
    "mediation_bootstrap",
    "moderation_point",
    "moderation_bootstrap",
]


class CsemOracleUnavailable(RuntimeError):
    pass


class CsemOracleFailure(RuntimeError):
    pass


def find_rscript() -> Path:
    configured = os.environ.get("QPLS_RSCRIPT", "").strip()
    candidates = [
        Path(configured) if configured else None,
        Path(shutil.which("Rscript")) if shutil.which("Rscript") else None,
        DEFAULT_WINDOWS_RSCRIPT,
    ]
    for candidate in candidates:
        if candidate is not None and candidate.is_file():
            return candidate.resolve()
    raise CsemOracleUnavailable(
        "Rscript was not found; set QPLS_RSCRIPT to a validation-only R 4.x runtime"
    )


def _columns(scenario: GeneralSemScenario) -> dict[str, list[float | None]]:
    indicators = [
        indicator
        for block in scenario.model.blocks
        for indicator in block.indicator_ids
    ]
    return {
        indicator: [
            None if row.get(indicator) is None else float(row[indicator])
            for row in scenario.rows
        ]
        for indicator in indicators
    }


def _complete_row_count(scenario: GeneralSemScenario) -> int:
    indicators = [
        indicator
        for block in scenario.model.blocks
        for indicator in block.indicator_ids
    ]
    return sum(
        all(row.get(indicator) is not None for indicator in indicators)
        for row in scenario.rows
    )


def build_request(
    scenario: GeneralSemScenario,
    operation: Operation,
    *,
    requested: int | None = None,
    seed: int = 20_260_819,
    confidence_level: float = 0.95,
    index_plan: list[list[int]] | tuple[tuple[int, ...], ...] | None = None,
) -> dict[str, Any]:
    moderation = operation.startswith("moderation")
    bootstrap = operation.endswith("bootstrap")
    if moderation and not scenario.interactions:
        raise ValueError("moderation operation requires scenario interactions")
    if not moderation and (scenario.source_id is None or scenario.target_id is None):
        raise ValueError("mediation operation requires source and target identities")
    if bootstrap:
        if requested is None or not 2 <= requested <= 10_000:
            raise ValueError("bootstrap requested count must be in [2, 10000]")
        rows = _complete_row_count(scenario)
        replicate_indices = (
            [list(_replicate_indices(rows, seed, index)) for index in range(requested)]
            if index_plan is None
            else [list(indices) for indices in index_plan]
        )
        if len(replicate_indices) != requested or any(
            len(indices) != rows
            or any(type(index) is not int or index < 0 or index >= rows for index in indices)
            for indices in replicate_indices
        ):
            raise ValueError("bootstrap index plan leaves the complete-case frame")
        bootstrap_request: dict[str, Any] | None = {
            "confidence_level": confidence_level,
            "replicate_indices": replicate_indices,
        }
    else:
        if requested is not None:
            raise ValueError("point operation cannot request bootstrap replicates")
        bootstrap_request = None
    return {
        "schema_version": 1,
        "operation": operation,
        "scenario_id": scenario.scenario_id,
        "columns": _columns(scenario),
        "blocks": [
            {
                "construct_id": block.construct_id,
                "indicator_ids": list(block.indicator_ids),
                "mode": block.mode,
            }
            for block in scenario.model.blocks
        ],
        "paths": [
            {"source_id": path.source_id, "target_id": path.target_id}
            for path in scenario.model.paths
        ],
        "interactions": [
            {
                "interaction_id": interaction.interaction_id,
                "focal_id": interaction.focal_id,
                "moderator_id": interaction.moderator_id,
                "outcome_id": interaction.outcome_id,
            }
            for interaction in scenario.interactions
        ],
        "effect_target": (
            None
            if moderation
            else {"source_id": scenario.source_id, "target_id": scenario.target_id}
        ),
        "bootstrap": bootstrap_request,
    }


def run_csem_oracle(
    scenario: GeneralSemScenario,
    operation: Operation,
    *,
    requested: int | None = None,
    seed: int = 20_260_819,
    confidence_level: float = 0.95,
    timeout_seconds: int = 300,
    index_plan: list[list[int]] | tuple[tuple[int, ...], ...] | None = None,
) -> dict[str, Any]:
    request = build_request(
        scenario,
        operation,
        requested=requested,
        seed=seed,
        confidence_level=confidence_level,
        index_plan=index_plan,
    )
    temporary_root = Path(
        os.environ.get("QPLS_VALIDATION_TMP", ROOT / "target" / "validation-temp")
    )
    temporary_root.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(
        prefix="qpls-rank0-csem-", dir=temporary_root
    ) as raw:
        directory = Path(raw)
        input_path = directory / "request.json"
        output_path = directory / "result.json"
        input_path.write_text(
            json.dumps(request, ensure_ascii=False, allow_nan=False),
            encoding="utf-8",
        )
        process = subprocess.run(
            [
                str(find_rscript()),
                "--vanilla",
                str(SCRIPT),
                str(input_path),
                str(output_path),
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout_seconds,
            check=False,
        )
        if process.returncode != 0 or not output_path.is_file():
            detail = (process.stderr or process.stdout).strip()
            raise CsemOracleFailure(
                f"R/cSEM oracle exited {process.returncode}: {detail[-4000:]}"
            )
        try:
            result = json.loads(output_path.read_text(encoding="utf-8"))
        except (OSError, UnicodeError, json.JSONDecodeError) as error:
            raise CsemOracleFailure(f"R/cSEM oracle output is invalid: {error}") from error
    if not isinstance(result, dict):
        raise CsemOracleFailure("R/cSEM oracle output must be an object")
    expected = {
        "schema_version",
        "report_kind",
        "scenario_id",
        "operation",
        "runtime",
        "used_observations",
        "point",
        "bootstrap",
        "qualification_ready",
        "independence",
    }
    if set(result) != expected:
        raise CsemOracleFailure("R/cSEM oracle output fields are not exact")
    if (
        result.get("schema_version") != 1
        or result.get("report_kind")
        != "quickpls_general_sem_rank0_independent_csem_oracle"
        or result.get("scenario_id") != scenario.scenario_id
        or result.get("operation") != operation
        or result.get("qualification_ready") is not False
    ):
        raise CsemOracleFailure("R/cSEM oracle output authority is inconsistent")
    return result


__all__ = [
    "CsemOracleFailure",
    "CsemOracleUnavailable",
    "Operation",
    "build_request",
    "find_rscript",
    "run_csem_oracle",
]
