from __future__ import annotations

import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class V255PackagedDriverReadinessTests(unittest.TestCase):
    def test_posthoc_waits_for_the_complete_public_calculate_catalogue(self) -> None:
        source = (
            ROOT / "validation/v255_posthoc_minimum_sample_packaged_smoke.mjs"
        ).read_text(encoding="utf-8")
        wait = (
            'document.querySelectorAll("#nd-calculation-method-list '
            "[role='option']\").length === 18"
        )
        self.assertIn(wait, source)
        self.assertLess(source.index(wait), source.index("const methodCount = await options.count()"))
        self.assertIn("assert(methodCount === 18", source)
        self.assertIn("methods instead of exactly 18.", source)


if __name__ == "__main__":
    unittest.main()
