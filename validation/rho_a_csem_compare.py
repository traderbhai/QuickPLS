import csv
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CSEM = ROOT / "validation" / "results" / "rho_a_csem_0_6_1.csv"
QPLS = ROOT / "validation" / "results" / "rho_a_quickpls_reference.json"
OUTPUT = ROOT / "validation" / "results" / "rho_a_csem_comparison.json"
TOLERANCE = 1e-6


def load_csem():
    with CSEM.open(newline="", encoding="utf-8") as handle:
        return {row["construct"]: row for row in csv.DictReader(handle)}


def load_quickpls():
    envelope = json.loads(QPLS.read_text(encoding="utf-8"))
    assessment = envelope["payload"]["assessment"]
    rows = assessment["construct_quality"]
    return {row["construct"]: row for row in rows}


def main():
    csem = load_csem()
    quickpls = load_quickpls()
    comparisons = []
    for construct in sorted(csem):
        reference = float(csem[construct]["rho_a_csem"])
        manual = float(csem[construct]["rho_a_manual"])
        actual = float(quickpls[construct]["rho_a"])
        comparisons.append(
            {
                "construct": construct,
                "quickpls_rho_a": actual,
                "csem_rho_a": reference,
                "csem_manual_rho_a": manual,
                "quickpls_minus_csem": actual - reference,
                "quickpls_minus_csem_manual": actual - manual,
                "abs_diff": abs(actual - reference),
                "passed": abs(actual - reference) <= TOLERANCE,
            }
        )
    report = {
        "status": "passed" if all(row["passed"] for row in comparisons) else "failed",
        "tolerance": TOLERANCE,
        "sources": {
            "csem": CSEM.as_posix().removeprefix(ROOT.as_posix() + "/"),
            "quickpls": QPLS.as_posix().removeprefix(ROOT.as_posix() + "/"),
        },
        "comparisons": comparisons,
        "max_abs_diff": max(row["abs_diff"] for row in comparisons),
    }
    OUTPUT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        raise SystemExit(f"rho_A cSEM comparison failed; see {OUTPUT}")


if __name__ == "__main__":
    main()
