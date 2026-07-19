"""Independent equation-level reference for Dijkstra-Henseler rho_A.

This runner intentionally does not import QuickPLS code or a PLS package. It
uses Decimal arithmetic on supplied correlation matrices and incoming weights,
then checks the committed reference output.
"""

from decimal import Decimal, getcontext
import json
from pathlib import Path


getcontext().prec = 50
ROOT = Path(__file__).resolve().parent

CASES = {
    "equicorrelation_three": {
        "correlations": [[1, ".5", ".5"], [".5", 1, ".5"], [".5", ".5", 1]],
        "incoming_weights": [1, 1, 1],
    },
    "unequal_signed": {
        "correlations": [[1, "-.4", ".1"], ["-.4", 1, "-.2"], [".1", "-.2", 1]],
        "incoming_weights": [".7", "-.2", ".5"],
    },
    "improper_below_zero": {
        "correlations": [[1, "-.2", "-.2"], ["-.2", 1, "-.2"], ["-.2", "-.2", 1]],
        "incoming_weights": [1, 1, 1],
    },
    "improper_above_one": {
        "correlations": [[1, "-.7", "-.7"], ["-.7", 1, ".1"], ["-.7", ".1", 1]],
        "incoming_weights": [-2, ".5", ".5"],
    },
}


def decimal(value):
    return Decimal(str(value))


def dot(left, right):
    return sum((a * b for a, b in zip(left, right)), Decimal(0))


def rho_a(case):
    correlations = [[decimal(value) for value in row] for row in case["correlations"]]
    incoming = [decimal(value) for value in case["incoming_weights"]]
    multiplied = [dot(row, incoming) for row in correlations]
    score_variance = dot(incoming, multiplied)
    weights = [value / score_variance.sqrt() for value in incoming]
    weight_norm_squared = dot(weights, weights)
    numerator = sum(
        (
            weights[row] * weights[column] * correlations[row][column]
            for row in range(len(weights))
            for column in range(len(weights))
            if row != column
        ),
        Decimal(0),
    )
    denominator = weight_norm_squared**2 - sum(
        (value**4 for value in weights), Decimal(0)
    )
    value = weight_norm_squared**2 * numerator / denominator
    return {
        "score_variance": str(score_variance),
        "weight_norm_squared": str(weight_norm_squared),
        "off_diagonal_numerator": str(numerator),
        "off_diagonal_denominator": str(denominator),
        "rho_a": str(value),
    }


actual = {name: rho_a(case) for name, case in CASES.items()}
expected_path = ROOT / "results" / "rho_a_reference.json"
if expected_path.exists():
    expected = json.loads(expected_path.read_text(encoding="ascii"))
    if actual != expected:
        raise SystemExit(
            "rho_A reference mismatch\n"
            + json.dumps({"expected": expected, "actual": actual}, indent=2, sort_keys=True)
        )
print(json.dumps(actual, indent=2, sort_keys=True))
