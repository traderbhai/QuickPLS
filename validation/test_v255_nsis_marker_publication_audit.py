from __future__ import annotations

import copy
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
if str(VALIDATION) not in sys.path:
    sys.path.insert(0, str(VALIDATION))

from general_sem_rank0_packaged_runner import (  # noqa: E402
    TAURI_NSIS_BUNDLE_MARKER,
    TAURI_PORTABLE_BUNDLE_MARKER,
)
from v255_product_completion_audit import (  # noqa: E402
    exact_installed_portable_equivalence,
)


def equivalence_receipt(marker_offset: int) -> dict[str, object]:
    return {
        "kind": "tauri_nsis_bundle_marker_variant_v1",
        "passed": True,
        "portable_marker": TAURI_PORTABLE_BUNDLE_MARKER.decode("ascii"),
        "installed_marker": TAURI_NSIS_BUNDLE_MARKER.decode("ascii"),
        "marker_offset": marker_offset,
        "all_other_bytes_identical": True,
    }


class V255NsisMarkerPublicationAuditTests(unittest.TestCase):
    def test_accepts_only_exact_distinct_tauri_nsis_marker_variant(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            directory = Path(raw)
            portable = directory / "portable.exe"
            installed = directory / "installed.exe"
            prefix = b"portable-prefix\x00"
            suffix = b"\x00portable-suffix"
            portable.write_bytes(prefix + TAURI_PORTABLE_BUNDLE_MARKER + suffix)
            installed.write_bytes(prefix + TAURI_NSIS_BUNDLE_MARKER + suffix)
            receipt = equivalence_receipt(len(prefix))

            self.assertTrue(
                exact_installed_portable_equivalence(receipt, installed, portable)
            )

            installed.write_bytes(portable.read_bytes())
            self.assertFalse(
                exact_installed_portable_equivalence(receipt, installed, portable)
            )

    def test_rejects_non_marker_byte_changes_and_inexact_receipts(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            directory = Path(raw)
            portable = directory / "portable.exe"
            installed = directory / "installed.exe"
            prefix = b"prefix"
            suffix = b"suffix"
            portable.write_bytes(prefix + TAURI_PORTABLE_BUNDLE_MARKER + suffix)
            installed.write_bytes(prefix + TAURI_NSIS_BUNDLE_MARKER + b"changed")
            receipt = equivalence_receipt(len(prefix))

            self.assertFalse(
                exact_installed_portable_equivalence(receipt, installed, portable)
            )

            installed.write_bytes(prefix + TAURI_NSIS_BUNDLE_MARKER + suffix)
            invalid_receipts = []
            for field, invalid_value in (
                ("kind", "byte_equivalent"),
                ("passed", False),
                ("portable_marker", "UNK"),
                ("installed_marker", "NSS"),
                ("marker_offset", len(prefix) + 1),
                ("marker_offset", True),
                ("all_other_bytes_identical", False),
            ):
                candidate = copy.deepcopy(receipt)
                candidate[field] = invalid_value
                invalid_receipts.append(candidate)
            extra_field = copy.deepcopy(receipt)
            extra_field["unbound_detail"] = True
            invalid_receipts.append(extra_field)

            for invalid in invalid_receipts:
                with self.subTest(receipt=invalid):
                    self.assertFalse(
                        exact_installed_portable_equivalence(
                            invalid, installed, portable
                        )
                    )


if __name__ == "__main__":
    unittest.main()
