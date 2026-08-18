#!/usr/bin/env python3
"""Compatibility entrypoint for the source-bound exact-CFA bootstrap audit."""

from __future__ import annotations

import json

try:
    from validation.cbsem_exact_case_bootstrap_v1_factory import (
        FactoryError,
        audit_prior_roles,
        mint_method_audit,
    )
except ModuleNotFoundError:
    from cbsem_exact_case_bootstrap_v1_factory import (
        FactoryError,
        audit_prior_roles,
        mint_method_audit,
    )


def audit() -> dict[str, object]:
    """Read-only assessment; it never turns missing roles into evidence."""

    return audit_prior_roles()


def main() -> int:
    checks = audit()
    if not checks["passed"]:
        print(json.dumps(checks, indent=2, sort_keys=True))
        return 1
    try:
        path = mint_method_audit()
    except (FactoryError, OSError, ValueError) as error:
        print(f"CB-SEM exact-bootstrap audit failed closed: {error}")
        return 1
    print(path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
