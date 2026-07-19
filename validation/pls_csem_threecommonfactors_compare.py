import csv
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "validation" / "fixtures" / "csem_threecommonfactors.csv"
RECIPE = ROOT / "validation" / "fixtures" / "csem_threecommonfactors.recipe.json"
CSEM = ROOT / "validation" / "results" / "pls_csem_threecommonfactors_0_6_1.csv"
QUICKPLS = ROOT / "validation" / "results" / "pls_quickpls_csem_threecommonfactors.json"
OUTPUT = ROOT / "validation" / "results" / "pls_csem_threecommonfactors_comparison.json"
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
            "--allow-experimental",
        ],
        cwd=ROOT,
        check=True,
    )
    envelope = json.loads(QUICKPLS.read_text(encoding="utf-8"))
    estimation = envelope["payload"]["estimation"]
    values = {}
    for row in estimation["paths"]:
        values[("path", row["source"], row["target"], "")] = float(row["coefficient"])
    for row in estimation["outer_estimates"]:
        values[("loading", row["construct"], "", row["indicator"])] = float(row["loading"])
        values[("weight", row["construct"], "", row["indicator"])] = float(row["weight"])
    return values


def load_csem():
    rows = {}
    with CSEM.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            key = (row["kind"], row["source"], row["target"], row["indicator"])
            rows[key] = float(row["value"])
    return rows


def main():
    quickpls = run_quickpls()
    csem = load_csem()
    comparisons = []
    for key, reference in sorted(csem.items()):
        actual = quickpls[key]
        comparisons.append(
            {
                "kind": key[0],
                "source": key[1],
                "target": key[2],
                "indicator": key[3],
                "quickpls": actual,
                "csem": reference,
                "difference": actual - reference,
                "abs_diff": abs(actual - reference),
                "passed": abs(actual - reference) <= TOLERANCE,
            }
        )
    report = {
        "status": "passed" if all(row["passed"] for row in comparisons) else "failed",
        "tolerance": TOLERANCE,
        "source": {
            "dataset": "cSEM::threecommonfactors",
            "documentation": "validation/r-library/cSEM/examples/example_csem.R",
            "population_path_values": {
                "eta2~eta1": 0.6,
                "eta3~eta1": 0.4,
                "eta3~eta2": 0.35,
            },
        },
        "artifacts": {
            "data": str(DATA.relative_to(ROOT)),
            "recipe": str(RECIPE.relative_to(ROOT)),
            "csem": str(CSEM.relative_to(ROOT)),
            "quickpls": str(QUICKPLS.relative_to(ROOT)),
        },
        "compared_quantities": ["path coefficients", "loadings", "weights"],
        "max_abs_diff": max(row["abs_diff"] for row in comparisons),
        "comparisons": comparisons,
    }
    OUTPUT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        raise SystemExit(f"published PLS cSEM comparison failed; see {OUTPUT}")


if __name__ == "__main__":
    main()
