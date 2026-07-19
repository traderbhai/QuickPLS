import csv
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "validation" / "fixtures" / "corporate_reputation.csv"
RECIPE = ROOT / "validation" / "fixtures" / "corporate_reputation.recipe.json"
SEMINR = ROOT / "validation" / "results" / "htmt_seminr_2_5_0.csv"
QUICKPLS = ROOT / "validation" / "results" / "htmt_quickpls_reference.json"
OUTPUT = ROOT / "validation" / "results" / "htmt_seminr_comparison.json"
TOLERANCE = 1e-6


def run_quickpls():
    subprocess.run(
        [
            "cargo",
            "run",
            "-p",
            "qpls-cli",
            "--",
            "run",
            str(RECIPE.relative_to(ROOT)),
            "--data",
            str(DATA.relative_to(ROOT)),
            "--output",
            str(QUICKPLS.relative_to(ROOT)),
        ],
        cwd=ROOT,
        check=True,
    )
    envelope = json.loads(QUICKPLS.read_text(encoding="utf-8"))
    return envelope["payload"]["assessment"]["htmt_plus"]


def load_seminr():
    values = {}
    with SEMINR.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            values[(row["variant"], row["row"], row["column"])] = float(row["value"])
    return values


def compare_matrix(artifact, seminr):
    constructs = artifact["constructs"]
    comparisons = []
    for row_index, row_name in enumerate(constructs):
        for column_index, column_name in enumerate(constructs):
            cell = artifact["cells"][row_index][column_index]
            reference = seminr[("htmt_plus", row_name, column_name)]
            actual = cell["value"]
            if actual is None:
                difference = None
                abs_diff = None
                passed = False
            else:
                actual = float(actual)
                difference = actual - reference
                abs_diff = abs(difference)
                passed = abs_diff <= TOLERANCE
            comparisons.append(
                {
                    "variant": "htmt_plus",
                    "row": row_name,
                    "column": column_name,
                    "quickpls_status": cell["status"],
                    "quickpls": actual,
                    "seminr": reference,
                    "difference": difference,
                    "abs_diff": abs_diff,
                    "passed": passed,
                    "required_for_status": True,
                    "classification": "equivalent" if passed else "failed_required_comparison",
                }
            )
    return comparisons


def main():
    quickpls = run_quickpls()
    seminr = load_seminr()
    comparisons = compare_matrix(quickpls, seminr)
    finite_diffs = [row["abs_diff"] for row in comparisons if row["abs_diff"] is not None]
    report = {
        "status": "passed" if all(row["passed"] for row in comparisons) else "failed",
        "tolerance": TOLERANCE,
        "reference": "seminr 2.5.0 summary(model)$validity$htmt on the corporate-reputation fixture",
        "definition_mapping": {
            "htmt_plus": "seminr reports the mean-absolute-correlation HTMT matrix for this fixture, matching QuickPLS ringle_et_al_htmt_plus_v1 values including mixed-sign cross-block correlations.",
            "scope": "Development-only external reference. seminr is not linked into or distributed with QuickPLS.",
        },
        "artifacts": {
            "data": str(DATA.relative_to(ROOT)),
            "recipe": str(RECIPE.relative_to(ROOT)),
            "seminr": str(SEMINR.relative_to(ROOT)),
            "quickpls": str(QUICKPLS.relative_to(ROOT)),
        },
        "metric_count": len(comparisons),
        "max_abs_diff": max(finite_diffs) if finite_diffs else None,
        "comparisons": comparisons,
    }
    OUTPUT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        raise SystemExit(f"HTMT+ seminr comparison failed; see {OUTPUT}")


if __name__ == "__main__":
    main()
