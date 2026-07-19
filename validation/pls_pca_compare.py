import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "validation" / "fixtures" / "simple_reflective.csv"
RECIPE = ROOT / "validation" / "fixtures" / "simple_reflective.pca.recipe.json"
REFERENCE = ROOT / "validation" / "results" / "pls_pca_numpy_reference.json"
QPLS = ROOT / "validation" / "results" / "pls_quickpls_pca.json"
OUTPUT = ROOT / "validation" / "results" / "pls_pca_numpy_comparison.json"
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
            str(QPLS.relative_to(ROOT)),
            "--allow-experimental",
        ],
        cwd=ROOT,
        check=True,
    )


def values_from_quickpls():
    envelope = json.loads(QPLS.read_text(encoding="utf-8"))
    estimation = envelope["payload"]["estimation"]
    values = {}
    for row in estimation["paths"]:
        values[("path", row["source"], row["target"], "")] = float(row["coefficient"])
    for row in estimation["outer_estimates"]:
        values[("loading", row["construct"], "", row["indicator"])] = float(row["loading"])
        values[("weight", row["construct"], "", row["indicator"])] = float(row["weight"])
    return values


def values_from_reference():
    reference = json.loads(REFERENCE.read_text(encoding="utf-8"))
    values = {}
    for row in reference["paths"]:
        values[("path", row["source"], row["target"], "")] = float(row["value"])
    for row in reference["outer"]:
        values[("loading", row["construct"], "", row["indicator"])] = float(row["loading"])
        values[("weight", row["construct"], "", row["indicator"])] = float(row["weight"])
    return values


def main():
    run_quickpls()
    quickpls = values_from_quickpls()
    reference = values_from_reference()
    comparisons = []
    for key, expected in sorted(reference.items()):
        actual = quickpls[key]
        comparisons.append(
            {
                "kind": key[0],
                "source": key[1],
                "target": key[2],
                "indicator": key[3],
                "quickpls": actual,
                "numpy_reference": expected,
                "difference": actual - expected,
                "abs_diff": abs(actual - expected),
                "passed": abs(actual - expected) <= TOLERANCE,
            }
        )
    report = {
        "status": "passed" if all(row["passed"] for row in comparisons) else "failed",
        "tolerance": TOLERANCE,
        "reference": REFERENCE.as_posix().removeprefix(ROOT.as_posix() + "/"),
        "quickpls": QPLS.as_posix().removeprefix(ROOT.as_posix() + "/"),
        "compared_quantities": ["path coefficients", "loadings", "weights"],
        "comparisons": comparisons,
        "max_abs_diff": max(row["abs_diff"] for row in comparisons),
    }
    OUTPUT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        raise SystemExit(f"PCA NumPy comparison failed; see {OUTPUT}")


if __name__ == "__main__":
    main()
