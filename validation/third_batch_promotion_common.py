#!/usr/bin/env python3
"""Shared helpers for v1.2.2 third-batch method-promotion audits."""

from __future__ import annotations

from typing import Any

from promotion_audit_integrity import (
    RESULTS,
    ROOT,
    load_json,
    report_passed,
    write_method_audit,
)


TARGET = "v1_2_2_group_prediction_regression_promotion"


def audit_method(
    method_id: str,
    promoted_scope: str,
    required_reports: list[str | dict[str, Any]],
    required_docs: list[str | dict[str, Any]],
    extra_checks: list[dict[str, Any]] | None = None,
) -> int:
    return write_method_audit(
        target=TARGET,
        method_id=method_id,
        promoted_scope=promoted_scope,
        required_reports=required_reports,
        required_docs=required_docs,
        extra_checks=extra_checks,
    )
