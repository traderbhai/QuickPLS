from __future__ import annotations

import contextlib
import io
import unittest
from unittest import mock

from validation import quickpls_3_competitor_program as program


class QuickPls3CompetitorProgramCliTests(unittest.TestCase):
    def invoke(self, report: dict[str, object], *arguments: str) -> int:
        with mock.patch.object(program, "validate_program", return_value=report):
            with contextlib.redirect_stdout(io.StringIO()):
                return program.main(list(arguments))

    def test_default_mode_allows_a_valid_but_pending_program(self) -> None:
        result = self.invoke({"passed": True, "competitor_ready": False})
        self.assertEqual(result, 0)

    def test_require_ready_fails_a_valid_but_pending_program(self) -> None:
        result = self.invoke(
            {"passed": True, "competitor_ready": False},
            "--require-ready",
        )
        self.assertEqual(result, 2)

    def test_require_ready_accepts_only_a_valid_ready_program(self) -> None:
        ready = self.invoke(
            {"passed": True, "competitor_ready": True},
            "--require-ready",
        )
        invalid = self.invoke(
            {"passed": False, "competitor_ready": True},
            "--require-ready",
        )
        self.assertEqual(ready, 0)
        self.assertEqual(invalid, 1)


if __name__ == "__main__":
    unittest.main()
