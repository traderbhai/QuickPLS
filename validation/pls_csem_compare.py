import csv
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "validation" / "fixtures" / "simple_reflective.csv"
CSEM = ROOT / "validation" / "results" / "pls_csem_0_6_1.csv"
OUTPUT = ROOT / "validation" / "results" / "pls_csem_comparison.json"
TOLERANCE = 1e-6

VARIANTS = {
    "PATH_MODE_A": ROOT / "validation" / "fixtures" / "simple_reflective.recipe.json",
    "MODE_B": ROOT / "validation" / "fixtures" / "simple_reflective.mode_b.recipe.json",
    "FACTOR": ROOT / "validation" / "fixtures" / "simple_reflective.factor.recipe.json",
    "PCA": ROOT / "validation" / "fixtures" / "simple_reflective.pca.recipe.json",
}


def run_quickpls(variant, recipe):
    result_path = ROOT / "validation" / "results" / f"pls_quickpls_{variant.lower()}.json"
    subprocess.run(
        [
            "cargo",
            "run",
            "-p",
            "qpls-cli",
            "--",
            "run",
            str(recipe.relative_to(ROOT)),
            "--data",
            str(DATA.relative_to(ROOT)),
            "--output",
            str(result_path.relative_to(ROOT)),
            "--allow-experimental",
        ],
        cwd=ROOT,
        check=True,
    )
    envelope = json.loads(result_path.read_text(encoding="utf-8"))
    estimation = envelope["payload"]["estimation"]
    values = {}
    for row in estimation["paths"]:
        values[("path", row["source"], row["target"], "")] = float(row["coefficient"])
    for row in estimation["outer_estimates"]:
        values[("loading", row["construct"], "", row["indicator"])] = float(row["loading"])
        values[("weight", row["construct"], "", row["indicator"])] = float(row["weight"])
    return result_path, values


def load_csem():
    rows = {}
    with CSEM.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            key = (row["kind"], row["source"], row["target"], row["indicator"])
            rows.setdefault(row["variant"], {})[key] = float(row["value"])
    return rows


def main():
    csem = load_csem()
    reports = []
    artifacts = {"csem": CSEM.as_posix().removeprefix(ROOT.as_posix() + "/")}
    for variant, recipe in VARIANTS.items():
        result_path, quickpls = run_quickpls(variant, recipe)
        artifacts[f"quickpls_{variant.lower()}"] = result_path.as_posix().removeprefix(
            ROOT.as_posix() + "/"
        )
        comparisons = []
        for key, reference in sorted(csem[variant].items()):
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
        reports.append(
            {
                "variant": variant,
                "status": "passed"
                if all(row["passed"] for row in comparisons)
                else "failed",
                "max_abs_diff": max(row["abs_diff"] for row in comparisons),
                "comparisons": comparisons,
            }
        )
    report = {
        "status": "passed" if all(item["status"] == "passed" for item in reports) else "failed",
        "tolerance": TOLERANCE,
        "artifacts": artifacts,
        "variants": reports,
    }
    OUTPUT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        raise SystemExit(f"PLS cSEM comparison failed; see {OUTPUT}")


if __name__ == "__main__":
    main()
