#!/usr/bin/env python3
"""Focused source/mutation coverage for v247 visual evidence contracts."""

from __future__ import annotations

import copy
import re
import unittest
from pathlib import Path, PurePosixPath


ROOT = Path(__file__).resolve().parents[1]
HARNESS = ROOT / "validation" / "v247_native_desktop_visual_acceptance.mjs"
PATH_PREFIX = "validation/results/screens/v247-native-desktop-visual/"
VIEWPORTS = {"1024x700", "1280x720", "1440x900"}
STATE = "structural-path-randomization-dialog"
RANDOMIZATION_DESCRIPTION = (
    "Structural Path Randomization Run candidate single-model Freedman-Lane "
    "randomization for structural paths using fixed original PLS construct scores "
    "and unadjusted pathwise p values."
)


def valid_descriptor(viewport: str = "1024x700") -> dict[str, object]:
    return {
        "path": f"{PATH_PREFIX}11-{STATE}-{viewport}.png",
        "size": 4096,
        "sha256": "a" * 64,
        "viewport": viewport,
        "state": STATE,
    }


def descriptor_contract(row: object) -> bool:
    if not isinstance(row, dict) or set(row) != {"path", "size", "sha256", "viewport", "state"}:
        return False
    path = row.get("path")
    size = row.get("size")
    sha256 = row.get("sha256")
    viewport = row.get("viewport")
    state = row.get("state")
    return (
        isinstance(path, str)
        and path.startswith(PATH_PREFIX)
        and PurePosixPath(path).name == f"11-{state}-{viewport}.png"
        and isinstance(size, int)
        and not isinstance(size, bool)
        and size > 0
        and isinstance(sha256, str)
        and re.fullmatch(r"[0-9a-f]{64}", sha256) is not None
        and viewport in VIEWPORTS
        and state == STATE
    )


def randomization_scope_contract(description: object) -> bool:
    if not isinstance(description, str):
        return False
    return (
        re.search(r"single-model Freedman(?:\u2013|-|\s)Lane randomization", description, re.IGNORECASE)
        is not None
        and re.search(r"structural paths", description, re.IGNORECASE) is not None
        and re.search(r"fixed original PLS construct scores", description, re.IGNORECASE) is not None
        and re.search(r"unadjusted pathwise p values", description, re.IGNORECASE) is not None
        and re.search(r"\bMGA\b|\bMICOM\b", description, re.IGNORECASE) is None
    )


class VisualScreenshotIntegrityTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.source = HARNESS.read_text(encoding="utf-8")

    def test_capture_records_post_write_relative_digest_descriptor(self) -> None:
        required = (
            'import { createHash } from "node:crypto";',
            'const screenshotDir = path.join(RESULTS, "screens", "v247-native-desktop-visual");',
            'const screenshotPathPrefix = "validation/results/screens/v247-native-desktop-visual/";',
            "const screenshotViewportIds = new Set([",
            "!screenshotViewportIds.has(viewport?.id)",
            'const [stat, bytes] = await Promise.all([fs.stat(screenshot), fs.readFile(screenshot)]);',
            'const relativePath = path.relative(ROOT, screenshot).replaceAll("\\\\", "/");',
            'sha256: createHash("sha256").update(bytes).digest("hex")',
            'stat.size !== screenshot.size',
            'actualSha256 !== screenshot.sha256',
            'duplicatePath',
        )
        for token in required:
            self.assertIn(token, self.source)
        push = re.search(r"evidence\.screenshots\.push\(\{(?P<body>.*?)\}\);", self.source, re.DOTALL)
        self.assertIsNotNone(push)
        keys = re.findall(r"^\s*([a-zA-Z][a-zA-Z0-9]*):?", push.group("body"), re.MULTILINE)
        self.assertEqual(keys, ["path", "size", "sha256", "viewport", "state"])
        self.assertNotIn("runtime:", push.group("body"))

    def test_exact_three_randomization_descriptors_are_viewport_ordered(self) -> None:
        rows = [valid_descriptor(viewport) for viewport in ("1024x700", "1280x720", "1440x900")]
        self.assertTrue(all(descriptor_contract(row) for row in rows))
        self.assertEqual([row["viewport"] for row in rows], ["1024x700", "1280x720", "1440x900"])
        self.assertEqual(len({row["path"] for row in rows}), 3)

    def test_descriptor_mutations_fail_closed(self) -> None:
        mutations = (
            lambda row: row.update(path="D:/QuickPLS/screenshot.png"),
            lambda row: row.update(path=f"{PATH_PREFIX}11-{STATE}-1280x720.png"),
            lambda row: row.update(size=0),
            lambda row: row.update(sha256="A" * 64),
            lambda row: row.update(sha256="a" * 63),
            lambda row: row.update(viewport="1366x768"),
            lambda row: row.update(state="calculation-dialog"),
            lambda row: row.update(runtime="chromium-preview"),
        )
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                row = copy.deepcopy(valid_descriptor())
                mutate(row)
                self.assertFalse(descriptor_contract(row))

    def test_randomization_scope_predicate_requires_exact_semantic_disclosure(self) -> None:
        required_source_tokens = (
            r"/single-model Freedman(?:\u2013|-|\s)Lane randomization/i",
            r"/structural paths/i",
            r"/fixed original PLS construct scores/i",
            r"/unadjusted pathwise p values/i",
            r"!/\bMGA\b|\bMICOM\b/i",
            "did not preserve its required single-model Freedman-Lane structural-path, fixed-score, unadjusted pathwise scope",
        )
        for token in required_source_tokens:
            self.assertIn(token, self.source)
        self.assertNotIn("randomization inference/i.test(check.methodDescription)", self.source)
        self.assertTrue(randomization_scope_contract(RANDOMIZATION_DESCRIPTION))

    def test_randomization_scope_mutations_fail_closed(self) -> None:
        mutations = (
            lambda value: value.replace("single-model ", ""),
            lambda value: value.replace("Freedman-Lane randomization", "path randomization"),
            lambda value: value.replace("structural paths", "model paths"),
            lambda value: value.replace("fixed original PLS construct scores", "construct scores"),
            lambda value: value.replace("unadjusted pathwise p values", "p values"),
            lambda value: f"{value} MICOM",
            lambda value: f"{value} MGA",
        )
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                self.assertFalse(randomization_scope_contract(mutate(RANDOMIZATION_DESCRIPTION)))


if __name__ == "__main__":
    unittest.main()
