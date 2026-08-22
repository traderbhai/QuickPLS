import unittest
from pathlib import Path
import sys

VALIDATION_ROOT = Path(__file__).resolve().parent
if str(VALIDATION_ROOT) not in sys.path:
    sys.path.insert(0, str(VALIDATION_ROOT))

from v255_product_completion_audit import post_gate_version_text


class ProductCompletionAuditContractTests(unittest.TestCase):
    def test_release_test_promotion_keeps_the_actual_public_download_at_v254(self) -> None:
        before = "\n".join(
            (
                'REPOSITORY_RELEASE_VERSION = "2.54.0"',
                'REPOSITORY_ARTIFACT_LABEL = "v2_54_0_canvas_results"',
                "Latest published public pre-release: "
                "[`v2.54.0`](https://github.com/traderbhai/QuickPLS/releases/tag/v2.54.0).",
            )
        )

        promoted = post_gate_version_text(
            "validation/test_package_release_artifacts.py",
            before,
        )

        self.assertIn('REPOSITORY_RELEASE_VERSION = "2.55.0"', promoted)
        self.assertIn('REPOSITORY_ARTIFACT_LABEL = "v2_55_0_calculate_evidence"', promoted)
        self.assertIn(
            "[`v2.54.0`](https://github.com/traderbhai/QuickPLS/releases/tag/v2.54.0)",
            promoted,
        )
        self.assertNotIn(
            "[`v2.55.0`](https://github.com/traderbhai/QuickPLS/releases/tag/v2.55.0)",
            promoted,
        )


if __name__ == "__main__":
    unittest.main()
