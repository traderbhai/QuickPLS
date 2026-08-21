#!/usr/bin/env python3
"""Independent micro-reference for the bounded General SEM mediation cell.

This validation-only program deliberately does not import QuickPLS code.  It
locks the path-product identities, Type-7 percentile rule, sample standard
error, null-centred plus-one probability, and 90% usable-replicate boundary
used by ``general_sem_pls_full_model_case_bootstrap_v1``.  It is contract-level
engine evidence only; it is not a full independent PLS-PM implementation or a
release-qualification oracle.
"""

from __future__ import annotations

import json
import math
from collections.abc import Iterable


METHOD_VERSION = "general_sem_pls_full_model_case_bootstrap_v1"
TOLERANCE = 1e-15


def type7(values: Iterable[float], probability: float) -> float:
    ordered = sorted(float(value) for value in values)
    if not ordered:
        raise ValueError("Type-7 quantile requires at least one value")
    if not 0.0 <= probability <= 1.0:
        raise ValueError("probability must be in [0, 1]")
    h = (len(ordered) - 1) * probability
    lower = math.floor(h)
    upper = math.ceil(h)
    if lower == upper:
        return ordered[lower]
    fraction = h - lower
    return ordered[lower] + fraction * (ordered[upper] - ordered[lower])


def sample_standard_error(values: Iterable[float]) -> float:
    values = [float(value) for value in values]
    if len(values) < 2:
        raise ValueError("sample standard error requires at least two values")
    mean = math.fsum(values) / len(values)
    return math.sqrt(math.fsum((value - mean) ** 2 for value in values) / (len(values) - 1))


def plus_one_two_sided(original: float, values: Iterable[float]) -> tuple[int, float]:
    values = [float(value) for value in values]
    exceedances = sum(abs(value - original) >= abs(original) for value in values)
    return exceedances, (exceedances + 1) / (len(values) + 1)


def minimum_usable(requested: int) -> int:
    if not 2 <= requested <= 10_000:
        raise ValueError("requested resamples must be in [2, 10000]")
    return max(2, math.ceil(0.9 * requested))


def multiple_mediation_effects() -> dict[str, float]:
    # X -> M1 -> Y; X -> M2 -> Y; and X -> M1 -> M2 -> Y.
    x_m1, x_m2, m1_m2 = 0.5, 0.3, 0.4
    m1_y, m2_y, direct = 0.6, 0.7, 0.2
    via_m1 = x_m1 * m1_y
    via_m2 = x_m2 * m2_y
    serial = x_m1 * m1_m2 * m2_y
    total_indirect = math.fsum([via_m1, via_m2, serial])
    return {
        "specific_x_m1_y": via_m1,
        "specific_x_m2_y": via_m2,
        "specific_x_m1_m2_y": serial,
        "total_indirect_x_y": total_indirect,
        "total_x_y": direct + total_indirect,
    }


def main() -> int:
    effects = multiple_mediation_effects()
    replicates = [0.1, 0.2, 0.3, 0.4]
    exceedances, probability = plus_one_two_sided(0.25, [0.0, 0.2, 0.3, 0.4])
    checks = {
        "specific_parallel_1": abs(effects["specific_x_m1_y"] - 0.3) <= TOLERANCE,
        "specific_parallel_2": abs(effects["specific_x_m2_y"] - 0.21) <= TOLERANCE,
        "specific_serial": abs(effects["specific_x_m1_m2_y"] - 0.14) <= TOLERANCE,
        "total_indirect": abs(effects["total_indirect_x_y"] - 0.65) <= TOLERANCE,
        "total": abs(effects["total_x_y"] - 0.85) <= TOLERANCE,
        "type7_lower": abs(type7(replicates, 0.025) - 0.1075) <= TOLERANCE,
        "type7_upper": abs(type7(replicates, 0.975) - 0.3925) <= TOLERANCE,
        "sample_standard_error": abs(sample_standard_error(replicates) - 0.12909944487358055) <= TOLERANCE,
        "plus_one_exceedances": exceedances == 1,
        "plus_one_probability": abs(probability - 0.4) <= TOLERANCE,
        "usable_gate_2": minimum_usable(2) == 2,
        "usable_gate_10": minimum_usable(10) == 9,
        "usable_gate_11": minimum_usable(11) == 10,
        "usable_gate_10000": minimum_usable(10_000) == 9_000,
    }
    report = {
        "schema_version": 1,
        "kind": "general_sem_pls_multiple_mediation_bootstrap_v1_contract_reference",
        "method_version": METHOD_VERSION,
        "evidence_scope": "contract_level_engine_only",
        "qualification_ready": False,
        "passed": all(checks.values()),
        "checks": checks,
        "effects": effects,
    }
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
