import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "validation" / "fixtures" / "simple_reflective.csv"
REFERENCE = ROOT / "validation" / "results" / "pls_plspm_0_5_7.json"
OUTPUT = ROOT / "validation" / "results" / "pls_plspm_comparison.json"
TOLERANCE = 1e-6

VARIANTS = {
    "PATH_MODE_A": ROOT / "validation" / "fixtures" / "simple_reflective.recipe.json",
    "MODE_B": ROOT / "validation" / "fixtures" / "simple_reflective.mode_b.recipe.json",
    "FACTOR": ROOT / "validation" / "fixtures" / "simple_reflective.factor.recipe.json",
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
    return result_path, values


def load_reference():
    reference = json.loads(REFERENCE.read_text(encoding="utf-8"))
    rows = {}
    for variant in reference["variants"]:
        values = {}
        for row in variant["paths"]:
            values[("path", row["source"], row["target"], "")] = float(row["value"])
        for row in variant["outer"]:
            values[("loading", row["construct"], "", row["indicator"])] = float(row["loading"])
        rows[variant["variant"]] = values
    return rows


def main():
    reference = load_reference()
    reports = []
    artifacts = {"plspm": REFERENCE.as_posix().removeprefix(ROOT.as_posix() + "/")}
    for variant, recipe in VARIANTS.items():
        result_path, quickpls = run_quickpls(variant, recipe)
        artifacts[f"quickpls_{variant.lower()}"] = result_path.as_posix().removeprefix(
            ROOT.as_posix() + "/"
        )
        comparisons = []
        for key, expected in sorted(reference[variant].items()):
            actual = quickpls[key]
            comparisons.append(
                {
                    "kind": key[0],
                    "source": key[1],
                    "target": key[2],
                    "indicator": key[3],
                    "quickpls": actual,
                    "plspm": expected,
                    "difference": actual - expected,
                    "abs_diff": abs(actual - expected),
                    "passed": abs(actual - expected) <= TOLERANCE,
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
        "compared_quantities": ["path coefficients", "loadings"],
        "excluded_quantities": [
            "outer weights because python-plspm uses a different normalization convention"
        ],
        "artifacts": artifacts,
        "variants": reports,
    }
    OUTPUT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        raise SystemExit(f"PLS plspm comparison failed; see {OUTPUT}")


if __name__ == "__main__":
    main()
