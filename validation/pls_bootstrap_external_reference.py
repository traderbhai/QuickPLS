#!/usr/bin/env python3
"""PLS-integrated bootstrap reference evidence against cSEM.

This is development-only validation evidence. It does not make cSEM or R a
runtime dependency of QuickPLS.
"""

from __future__ import annotations

import csv
import json
import math
import random
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Iterable

from r_runtime import find_rscript_optional

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "validation" / "fixtures" / "simple_reflective.csv"
VARIANTS = {
    "PATH_MODE_A": ROOT / "validation" / "fixtures" / "simple_reflective.recipe.json",
    "MODE_B": ROOT / "validation" / "fixtures" / "simple_reflective.mode_b.recipe.json",
    "FACTOR": ROOT / "validation" / "fixtures" / "simple_reflective.factor.recipe.json",
    "PCA": ROOT / "validation" / "fixtures" / "simple_reflective.pca.recipe.json",
}
RESULTS = ROOT / "validation" / "results"
WORK = RESULTS / "pls_bootstrap_external_reference"
OUTPUT = RESULTS / "pls_bootstrap_external_reference.json"
TOLERANCE = 1.0e-6
SUMMARY_TOLERANCE = 1.0e-6
TARGET_ACCEPTED_REPLICATES = 12
MAX_CANDIDATES = 48
SEED = 2026071901
PARAMETERS = [
    ("path", "x", "y", ""),
    ("loading", "x", "", "x1"),
    ("loading", "x", "", "x2"),
    ("loading", "y", "", "y1"),
    ("loading", "y", "", "y2"),
    ("weight", "x", "", "x1"),
    ("weight", "x", "", "x2"),
    ("weight", "y", "", "y1"),
    ("weight", "y", "", "y2"),
]


def relative(path: Path) -> str:
    return path.as_posix().removeprefix(ROOT.as_posix() + "/")


def find_rscript() -> str | None:
    found = find_rscript_optional()
    return found[0] if found is not None else None


def read_source_rows() -> list[dict[str, str]]:
    with DATA.open("r", encoding="utf-8", newline="") as handle:
        return list(csv.DictReader(handle))


def write_resample(rows: list[dict[str, str]], indices: list[int], output: Path) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0].keys()))
        writer.writeheader()
        for index in indices:
            writer.writerow(rows[index])


def dataset_fingerprint(data: Path) -> str:
    project = data.with_suffix(".fingerprint.qpls")
    subprocess.run(
        [
            "cargo",
            "run",
            "-p",
            "qpls-cli",
            "--",
            "import",
            relative(data),
            relative(project),
            "--name",
            data.stem,
        ],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=True,
    )
    completed = subprocess.run(
        [
            "cargo",
            "run",
            "-p",
            "qpls-cli",
            "--",
            "inspect",
            relative(project),
            "--json",
        ],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=True,
    )
    report = json.loads(completed.stdout)
    return str(report["datasets"][0]["fingerprint"])


def write_recipe(template: Path, fingerprint: str, variant: str, output: Path) -> None:
    recipe = json.loads(template.read_text(encoding="utf-8"))
    recipe["dataset_fingerprint"] = fingerprint
    recipe["id"] = f"00000000-0000-0000-0000-{list(VARIANTS).index(variant) + 501:012d}"
    recipe["metadata"] = {
        "fixture": "PLS bootstrap external-reference multi-variant matched-resample validation",
        "source_recipe": relative(template),
        "variant": variant,
    }
    output.write_text(json.dumps(recipe, indent=2) + "\n", encoding="utf-8")


def run_quickpls(
    variant: str, data: Path, recipe: Path, output: Path
) -> dict[tuple[str, str, str, str, str], float]:
    subprocess.run(
        [
            "cargo",
            "run",
            "-p",
            "qpls-cli",
            "--",
            "run",
            relative(recipe),
            "--data",
            relative(data),
            "--output",
            relative(output),
            "--allow-experimental",
            "--bootstrap-samples",
            "0",
            "--workers",
            "1",
        ],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=True,
    )
    envelope = json.loads(output.read_text(encoding="utf-8"))
    estimation = envelope["payload"]["estimation"]
    if not estimation["converged"]:
        raise RuntimeError("QuickPLS estimator did not converge")
    values: dict[tuple[str, str, str, str], float] = {}
    for row in estimation["paths"]:
        values[("path", row["source"], row["target"], "")] = float(row["coefficient"])
    for row in estimation["outer_estimates"]:
        values[("loading", row["construct"], "", row["indicator"])] = float(row["loading"])
        values[("weight", row["construct"], "", row["indicator"])] = float(row["weight"])
    return {(variant, *key): values[key] for key in PARAMETERS}


def run_csem(rscript: str, data: Path, output: Path) -> dict[tuple[str, str, str, str, str], float]:
    completed = subprocess.run(
        [
            rscript,
            "--vanilla",
            str(ROOT / "validation" / "r_csem_reference.R"),
            str(data),
            str(output),
        ],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"cSEM reference failed\nstdout:\n{completed.stdout}\n\nstderr:\n{completed.stderr}"
        )
    values: dict[tuple[str, str, str, str, str], float] = {}
    with output.open("r", encoding="utf-8-sig", newline="") as handle:
        for row in csv.DictReader(handle):
            key = (row["kind"], row["source"], row["target"], row["indicator"])
            if row["variant"] in VARIANTS and key in PARAMETERS:
                values[(row["variant"], *key)] = float(row["value"])
    return {(variant, *key): values[(variant, *key)] for variant in VARIANTS for key in PARAMETERS}


def sample_standard_error(values: list[float]) -> float:
    if len(values) < 2:
        raise ValueError("at least two values are required")
    mean = sum(values) / len(values)
    variance = sum((value - mean) ** 2 for value in values) / (len(values) - 1)
    return math.sqrt(variance)


def type7_quantile(values: Iterable[float], probability: float) -> float:
    sorted_values = sorted(values)
    if probability <= 0.0:
        return sorted_values[0]
    if probability >= 1.0:
        return sorted_values[-1]
    position = probability * (len(sorted_values) - 1)
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return sorted_values[lower]
    fraction = position - lower
    return sorted_values[lower] * (1.0 - fraction) + sorted_values[upper] * fraction


def summarize(values: dict[tuple[str, str, str, str, str], list[float]]) -> list[dict[str, object]]:
    rows = []
    for key in values:
        series = values[key]
        rows.append(
            {
                "variant": key[0],
                "kind": key[1],
                "source": key[2],
                "target": key[3],
                "indicator": key[4],
                "mean": sum(series) / len(series),
                "sample_standard_error": sample_standard_error(series),
                "percentile_2_5": type7_quantile(series, 0.025),
                "percentile_97_5": type7_quantile(series, 0.975),
            }
        )
    return rows


def key_from_row(row: dict[str, object]) -> tuple[str, str, str, str, str]:
    return (
        str(row["variant"]),
        str(row["kind"]),
        str(row["source"]),
        str(row["target"]),
        str(row["indicator"]),
    )


def main() -> int:
    rscript = find_rscript()
    if rscript is None:
        raise SystemExit("Rscript.exe was not found. Set QPLS_RSCRIPT to the full path.")
    os.environ["R_LIBS_USER"] = str(ROOT / "validation" / "r-library")
    if WORK.exists():
        shutil.rmtree(WORK)
    WORK.mkdir(parents=True, exist_ok=True)

    source_rows = read_source_rows()
    rng = random.Random(SEED)
    accepted = []
    skipped = []
    parameter_keys = [(variant, *key) for variant in VARIANTS for key in PARAMETERS]
    quick_values = {key: [] for key in parameter_keys}
    csem_values = {key: [] for key in parameter_keys}

    for candidate_index in range(MAX_CANDIDATES):
        indices = [rng.randrange(len(source_rows)) for _ in source_rows]
        data_path = WORK / f"candidate_{candidate_index:03d}.csv"
        csem_path = WORK / f"candidate_{candidate_index:03d}.csem.csv"
        write_resample(source_rows, indices, data_path)
        try:
            fingerprint = dataset_fingerprint(data_path)
            csem = run_csem(rscript, data_path, csem_path)
            quick = {}
            quick_artifacts = {}
            recipe_artifacts = {}
            for variant, template in VARIANTS.items():
                recipe_path = WORK / f"candidate_{candidate_index:03d}.{variant.lower()}.recipe.json"
                quick_path = WORK / f"candidate_{candidate_index:03d}.{variant.lower()}.quickpls.json"
                write_recipe(template, fingerprint, variant, recipe_path)
                quick.update(run_quickpls(variant, data_path, recipe_path, quick_path))
                recipe_artifacts[variant] = relative(recipe_path)
                quick_artifacts[variant] = relative(quick_path)
        except Exception as error:  # noqa: BLE001 - validation report keeps skip reason.
            skipped.append(
                {
                    "candidate": candidate_index,
                    "indices": indices,
                    "reason": str(error).splitlines()[0],
                }
            )
            continue
        comparisons = []
        candidate_passed = True
        for key in parameter_keys:
            difference = quick[key] - csem[key]
            row = {
                "variant": key[0],
                "kind": key[1],
                "source": key[2],
                "target": key[3],
                "indicator": key[4],
                "quickpls": quick[key],
                "csem": csem[key],
                "difference": difference,
                "abs_diff": abs(difference),
                "passed": abs(difference) <= TOLERANCE,
            }
            candidate_passed = candidate_passed and bool(row["passed"])
            comparisons.append(row)
        if not candidate_passed:
            skipped.append(
                {
                    "candidate": candidate_index,
                    "indices": indices,
                    "reason": "estimate_mismatch",
                    "max_abs_diff": max(row["abs_diff"] for row in comparisons),
                }
            )
            continue
        for key in parameter_keys:
            quick_values[key].append(quick[key])
            csem_values[key].append(csem[key])
        accepted.append(
            {
                "candidate": candidate_index,
                "indices": indices,
                "data": relative(data_path),
                "recipes": recipe_artifacts,
                "quickpls": quick_artifacts,
                "csem": relative(csem_path),
                "max_abs_diff": max(row["abs_diff"] for row in comparisons),
                "comparisons": comparisons,
            }
        )
        if len(accepted) >= TARGET_ACCEPTED_REPLICATES:
            break

    quick_summary = summarize(quick_values) if len(accepted) >= 2 else []
    csem_summary = summarize(csem_values) if len(accepted) >= 2 else []
    csem_summary_by_key = {key_from_row(row): row for row in csem_summary}
    summary_comparisons = []
    for quick_row in quick_summary:
        key = key_from_row(quick_row)
        csem_row = csem_summary_by_key[key]
        metric_diffs = {
            metric: float(quick_row[metric]) - float(csem_row[metric])
            for metric in [
                "mean",
                "sample_standard_error",
                "percentile_2_5",
                "percentile_97_5",
            ]
        }
        summary_comparisons.append(
            {
                "variant": key[0],
                "kind": key[1],
                "source": key[2],
                "target": key[3],
                "indicator": key[4],
                "differences": metric_diffs,
                "max_abs_diff": max(abs(value) for value in metric_diffs.values()),
                "passed": all(abs(value) <= SUMMARY_TOLERANCE for value in metric_diffs.values()),
            }
        )

    passed = (
        len(accepted) == TARGET_ACCEPTED_REPLICATES
        and all(row["passed"] for row in summary_comparisons)
    )
    report = {
        "schema_version": 1,
        "kind": "pls_bootstrap_external_reference_v1",
        "passed": passed,
        "tolerance": TOLERANCE,
        "summary_tolerance": SUMMARY_TOLERANCE,
        "reference": {
            "engine": "cSEM",
            "runtime": "Rscript",
            "rscript": rscript,
            "package_scope": "development validation only; not distributed with QuickPLS",
        },
        "fixture": {
            "source_data": relative(DATA),
            "recipes": {variant: relative(path) for variant, path in VARIANTS.items()},
            "resample_seed": SEED,
            "target_accepted_replicates": TARGET_ACCEPTED_REPLICATES,
            "max_candidates": MAX_CANDIDATES,
            "variants": list(VARIANTS),
            "parameters": [
                {
                    "kind": key[0],
                    "source": key[1],
                    "target": key[2],
                    "indicator": key[3],
                }
                for key in PARAMETERS
            ],
        },
        "accepted_replicates": accepted,
        "skipped_candidates": skipped,
        "quickpls_summary": quick_summary,
        "csem_summary": csem_summary,
        "summary_comparisons": summary_comparisons,
        "max_replicate_abs_diff": max(
            (replicate["max_abs_diff"] for replicate in accepted),
            default=None,
        ),
        "max_summary_abs_diff": max(
            (row["max_abs_diff"] for row in summary_comparisons),
            default=None,
        ),
        "note": "Fixed resample-index PLS integration fixture across path Mode A, Mode B, factorial, and PCA variants. This verifies estimator and aggregate bootstrap summary parity on matched resamples; it is not stochastic coverage qualification.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {OUTPUT} | passed={passed} | accepted={len(accepted)}")
    return 0 if passed else 1


if __name__ == "__main__":
    sys.exit(main())
