#!/usr/bin/env python3
"""Shared helpers for the final v1.2 method-promotion audits."""

from __future__ import annotations

from typing import Any

from promotion_audit_integrity import (
    RESULTS,
    ROOT,
    load_json,
    report_passed,
    write_method_audit,
)


def audit_method(
    target: str,
    method_id: str,
    promoted_scope: str,
    required_reports: list[str | dict[str, Any]],
    required_docs: list[str | dict[str, Any]],
    extra_checks: list[dict[str, Any]] | None = None,
) -> int:
    return write_method_audit(
        target=target,
        method_id=method_id,
        promoted_scope=promoted_scope,
        required_reports=required_reports,
        required_docs=required_docs,
        extra_checks=extra_checks,
    )
