#!/usr/bin/env python3
"""Compatibility entry point for current-product PLS work evidence.

The historical implementation invoked ``qpls run --allow-experimental``.
That is no longer a valid qualification seam: the customer CLI is governed by
the Capability Registry and must reject the evidence-absent base PLS cell.

All generation now delegates to the ignored, environment-gated desktop harness
orchestrator in :mod:`pls_algorithm_v1_current_product_work`.  The retained
filename avoids breaking documented automation while removing every customer
CLI execution path and every attachable-receipt implication.
"""

from __future__ import annotations

from pls_algorithm_v1_current_product_work import (
    VARIANTS,
    _assert_exact_identity,
    _bind_harness_binary,
    _cargo_build_command,
    _comparison,
    _extract_harness_executable,
    _harness_command,
    _oracle_values,
    _product_values,
    main,
    write_new_snapshot,
)


__all__ = [
    "VARIANTS",
    "_assert_exact_identity",
    "_bind_harness_binary",
    "_cargo_build_command",
    "_comparison",
    "_extract_harness_executable",
    "_harness_command",
    "_oracle_values",
    "_product_values",
    "write_new_snapshot",
]


if __name__ == "__main__":
    raise SystemExit(main())
