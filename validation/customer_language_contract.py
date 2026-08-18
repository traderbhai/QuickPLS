#!/usr/bin/env python3
"""Audit customer-facing source for internal qualification language.

This module deliberately separates *inventory* from *enforcement*.  During the
language cutover ``report`` mode gives a deterministic backlog without blocking
development.  Product finalization switches the same scanner to ``strict`` so
that any remaining prohibited phrase fails the gate.

The scanner uses narrow, customer-visible phrases.  It intentionally does not
flag ordinary implementation identifiers such as ``candidate`` or ``evidence``
on their own.
"""

from __future__ import annotations

import argparse
import json
import re
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Iterable, Sequence


SCHEMA_VERSION = 1
DEFAULT_SOURCE_ROOTS = ("src",)
SOURCE_SUFFIXES = frozenset({".ts", ".tsx", ".js", ".jsx"})
EXCLUDED_NAME_FRAGMENTS = (".test.", ".spec.", ".stories.")
EXCLUDED_DIRECTORIES = frozenset({"__tests__", "fixtures", "node_modules", "dist", "target"})
ALLOW_INTERNAL_MARKER = "customer-copy-lint: allow-internal"


@dataclass(frozen=True)
class LanguageRule:
    rule_id: str
    pattern: str
    replacement: str
    rationale: str


@dataclass(frozen=True)
class LanguageViolation:
    rule_id: str
    path: str
    line: int
    column: int
    matched_text: str
    replacement: str


LANGUAGE_RULES: tuple[LanguageRule, ...] = (
    LanguageRule(
        "validated_scope",
        r"\bvalidated\s+scope\b",
        "Supported setup or Requirements",
        "Scientific requirements belong in plain language; evidence maturity is internal governance.",
    ),
    LanguageRule(
        "calculation_scope",
        r"\bcalculation\s+scope\b",
        "Analysis details",
        "The customer needs the analysis configuration, not an internal scope label.",
    ),
    LanguageRule(
        "method_scope",
        r"\bmethod\s+scope\b",
        "Method Details or concrete requirements",
        "A customer should see the actual model and data requirements, not a governance boundary label.",
    ),
    LanguageRule(
        "documented_scope",
        r"\bdocumented(?:\s+quickpls)?(?:\s+[\w./\u2013\u2014-]+){0,8}\s+scope\b|\bsupported\s+scope\b|\bscope\s+status\b",
        "Analysis details or the concrete requirement",
        "Generic scope boilerplate obscures the specific assumption or limitation that matters.",
    ),
    LanguageRule(
        "validated_setup_boilerplate",
        r"(?<!cross-)\bvalidated(?:\s+[\w./\u2013\u2014-]+){0,8}\s+(?:scope|setup|workflow|output|add-on)\b|(?<!cross-)\bvalidated\s+[a-z][\w./\u2013\u2014-]*",
        "List the concrete requirement or use Supported setup",
        "Customers need the actual model and data requirements, not a qualification adjective.",
    ),
    LanguageRule(
        "bounded_scope_boilerplate",
        r"\bbounded(?:\s+[\w./\u2013\u2014-]+){0,8}\s+(?:scope|preview|diagnostic|workflow|model|simulation|segmentation|slice|shape|adapter)\b",
        "List the concrete requirement",
        "Bounded is an internal coverage concept; the product should name the supported setup directly.",
    ),
    LanguageRule(
        "experimental_preview_boilerplate",
        r"\bbounded\s+preview\b|\bpreview-only\b|\bcandidate(?:\s+[\w./\u2013\u2014-]+){0,8}\s+(?:scope|output|evidence|workflow)\b",
        "Experimental plus a concrete limitation",
        "Labs uses one Experimental status and explains the actual limitation in Method Details.",
    ),
    LanguageRule(
        "governance_evidence",
        r"\b(?:current|parity|qualification|release|native|packaged)\s+evidence\b",
        "Method references, Run Details, or a concrete historical limitation",
        "Qualification-state evidence belongs in internal validation reports.",
    ),
    LanguageRule(
        "trust_evidence",
        r"\btrust\s+evidence\b",
        "Method Details",
        "Internal confidence-governance terminology is not a customer navigation concept.",
    ),
    LanguageRule(
        "validation_evidence",
        r"\bvalidation\s+(?:evidence|artifact(?:s)?|audit(?:s)?)\b",
        "Method references or Run Details",
        "Internal qualification artifacts belong in developer validation reports, not normal product copy.",
    ),
    LanguageRule(
        "scope_checked",
        r"\bscope\s+(?:checked|matrix|transparency)\b",
        "Requirements checked or Method compatibility",
        "Internal checklist vocabulary should be translated into the customer action or result.",
    ),
    LanguageRule(
        "bounded_native_scope",
        r"\bbounded\s+native\s+scope\b",
        "List the concrete requirement",
        "Native-stage qualification terminology is not a customer concept.",
    ),
    LanguageRule(
        "qualified_state",
        r"\b(?:native|release)[\s-]+qualified\b|\bqualified(?:\s+[\w./\u2013\u2014-]+){0,8}\s+(?:scope|setup|workflow|output)\b",
        "Supported or Experimental",
        "Manifest evidence states must never be displayed as product availability labels.",
    ),
    LanguageRule(
        "candidate_unqualified",
        r"\bcandidate\s*(?:[/;,]|and)\s*unqualified\b|\bunqualified\s+candidate\b",
        "Experimental",
        "Incomplete capabilities use the single customer-facing Labs status.",
    ),
    LanguageRule(
        "candidate_scope",
        r"\bcandidate\s+scope\b",
        "Experimental",
        "Labs uses one stable Experimental label rather than an internal candidate scope.",
    ),
    LanguageRule(
        "promotion_evidence",
        r"\bpromotion\s+evidence\b",
        "Remove from customer copy",
        "Promotion evidence is an internal qualification concern.",
    ),
    LanguageRule(
        "packaged_evidence",
        r"\bpackaged\s+evidence\b",
        "Remove from customer copy",
        "Packaged evidence is an internal qualification concern.",
    ),
    LanguageRule(
        "promotion_pending",
        r"\b(?:pending|awaiting)\s+(?:current\s+)?(?:release\s+)?promotion\b|\bnot\s+promoted\s+to\b",
        "Experimental",
        "Customers should see a stable product status, not the internal promotion workflow.",
    ),
    LanguageRule(
        "evidence_pending",
        r"\b(?:release\s+)?(?:qualification\s+)?evidence\s+(?:is\s+)?pending\b|\bpending\s+(?:current\s+)?(?:qualification\s+)?evidence\b",
        "Experimental",
        "Evidence collection is internal; Labs communicates the product consequence.",
    ),
)

_BROAD_RULE_IDS = frozenset({
    "validated_setup_boilerplate",
    "bounded_scope_boilerplate",
    "experimental_preview_boilerplate",
    "governance_evidence",
})
_COMPILED_RULES = tuple(
    (rule, re.compile(rule.pattern, re.IGNORECASE))
    for rule in sorted(LANGUAGE_RULES, key=lambda candidate: candidate.rule_id in _BROAD_RULE_IDS)
)


def _is_source_file(path: Path) -> bool:
    if path.suffix.lower() not in SOURCE_SUFFIXES:
        return False
    if any(fragment in path.name.lower() for fragment in EXCLUDED_NAME_FRAGMENTS):
        return False
    return not any(part.lower() in EXCLUDED_DIRECTORIES for part in path.parts)


def iter_customer_source_files(repository_root: Path, source_roots: Sequence[str] = DEFAULT_SOURCE_ROOTS) -> Iterable[Path]:
    """Yield the deterministic set of production UI source files."""

    candidates: list[Path] = []
    for relative_root in source_roots:
        root = repository_root / relative_root
        if not root.exists():
            continue
        candidates.extend(path for path in root.rglob("*") if path.is_file() and _is_source_file(path))
    yield from sorted(set(candidates), key=lambda path: path.as_posix().lower())


def _mask_javascript_comments(text: str) -> str:
    """Replace JS/TS comments with spaces while preserving offsets and lines.

    A small lexer is sufficient here and avoids false positives in internal
    implementation comments. Quoted strings and template literals are retained
    because they may be rendered, thrown as actionable errors, or exported.
    """

    output = list(text)
    state = "normal"
    quote = ""
    index = 0
    while index < len(text):
        current = text[index]
        following = text[index + 1] if index + 1 < len(text) else ""
        if state == "normal":
            if current in {"'", '"', "`"}:
                state = "string"
                quote = current
                index += 1
                continue
            if current == "/" and following == "/":
                output[index] = output[index + 1] = " "
                state = "line_comment"
                index += 2
                continue
            if current == "/" and following == "*":
                output[index] = output[index + 1] = " "
                state = "block_comment"
                index += 2
                continue
        elif state == "string":
            if current == "\\":
                index += 2
                continue
            if current == quote:
                state = "normal"
                quote = ""
        elif state == "line_comment":
            if current == "\n":
                state = "normal"
            else:
                output[index] = " "
        elif state == "block_comment":
            if current == "*" and following == "/":
                output[index] = output[index + 1] = " "
                state = "normal"
                index += 2
                continue
            if current not in {"\r", "\n"}:
                output[index] = " "
        index += 1
    return "".join(output)


def scan_text(text: str, relative_path: str) -> list[LanguageViolation]:
    """Return all prohibited customer-language matches in one source file."""

    violations: list[LanguageViolation] = []
    original_lines = text.splitlines()
    masked_lines = _mask_javascript_comments(text).splitlines()
    for line_number, line in enumerate(masked_lines, start=1):
        if ALLOW_INTERNAL_MARKER in original_lines[line_number - 1]:
            continue
        reported_spans: set[tuple[int, int]] = set()
        for rule, pattern in _COMPILED_RULES:
            for match in pattern.finditer(line):
                span = match.span()
                if any(span[0] < existing[1] and existing[0] < span[1] for existing in reported_spans):
                    continue
                reported_spans.add(span)
                violations.append(
                    LanguageViolation(
                        rule_id=rule.rule_id,
                        path=relative_path.replace("\\", "/"),
                        line=line_number,
                        column=match.start() + 1,
                        matched_text=match.group(0),
                        replacement=rule.replacement,
                    )
                )
    return violations


def scan_repository(repository_root: Path, source_roots: Sequence[str] = DEFAULT_SOURCE_ROOTS) -> dict[str, object]:
    """Build a stable machine-readable customer-language inventory."""

    root = repository_root.resolve()
    files = list(iter_customer_source_files(root, source_roots))
    violations: list[LanguageViolation] = []
    for path in files:
        relative_path = path.relative_to(root).as_posix()
        violations.extend(scan_text(path.read_text(encoding="utf-8"), relative_path))
    violations.sort(key=lambda item: (item.path.lower(), item.line, item.column, item.rule_id))

    counts_by_rule = {rule.rule_id: 0 for rule in LANGUAGE_RULES}
    for violation in violations:
        counts_by_rule[violation.rule_id] += 1

    return {
        "schema_version": SCHEMA_VERSION,
        "scanned_files": len(files),
        "violation_count": len(violations),
        "counts_by_rule": counts_by_rule,
        "strict_passed": not violations,
        "violations": [asdict(violation) for violation in violations],
    }


def format_text_report(report: dict[str, object]) -> str:
    lines = [
        "QuickPLS customer-language contract",
        f"Scanned files: {report['scanned_files']}",
        f"Prohibited occurrences: {report['violation_count']}",
        f"Strict gate: {'PASS' if report['strict_passed'] else 'FAIL'}",
    ]
    violations = report.get("violations", [])
    if isinstance(violations, list):
        for item in violations:
            if not isinstance(item, dict):
                continue
            lines.append(
                f"{item['path']}:{item['line']}:{item['column']} "
                f"[{item['rule_id']}] {item['matched_text']!r} -> {item['replacement']}"
            )
    return "\n".join(lines)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--source-root", action="append", dest="source_roots")
    parser.add_argument("--strict", action="store_true", help="Exit nonzero when any prohibited phrase remains.")
    parser.add_argument("--json", action="store_true", help="Print the machine-readable report.")
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    report = scan_repository(args.root, tuple(args.source_roots or DEFAULT_SOURCE_ROOTS))
    print(json.dumps(report, indent=2, sort_keys=True) if args.json else format_text_report(report))
    return 1 if args.strict and not report["strict_passed"] else 0


if __name__ == "__main__":
    raise SystemExit(main())
