from __future__ import annotations

import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
EXPECTED_KIND_ORDER = [
    "pls_algorithm",
    "plsc",
    "wpls",
    "gsca",
    "cca",
    "cta_pls",
    "ipma",
    "cbsem",
    "pls_bootstrap",
    "pls_permutation",
    "mga",
    "predict",
    "nca",
    "pca",
    "regression",
]


def derive_catalogue(
    catalog_source: str,
    recipe_source: str,
    calculation_mode_source: str,
) -> list[tuple[str, str]]:
    catalog_match = re.search(
        r"const CATALOG_DRAFTS[^=]*= \[([\s\S]*?)\n\] as const;",
        catalog_source,
    )
    if catalog_match is None:
        raise ValueError("catalog declaration is missing")
    kinds = re.findall(
        r'^\s{4}kind:\s*"([a-z_]+)",\r?$',
        catalog_match.group(1),
        re.MULTILINE,
    )
    if kinds != EXPECTED_KIND_ORDER:
        raise ValueError(f"catalog kind order differs: {kinds!r}")

    labels_by_kind = dict(
        re.findall(
            r'\{\s*kind:\s*"([a-z_]+)"[^{}]*?\blabel:\s*"([^"]+)"',
            recipe_source,
        )
    )
    prediction_match = re.search(
        r'export const NATIVE_PREDICTION_METHOD_LABEL\s*=\s*"([^"]+)";',
        calculation_mode_source,
    )
    regression_match = re.search(
        r'item\.kind\s*===\s*"regression"\s*\?\s*"([^"]+)"',
        catalog_source,
    )
    if prediction_match is None or regression_match is None:
        raise ValueError("derived prediction or regression label is missing")
    labels_by_kind["predict"] = prediction_match.group(1)
    labels_by_kind["regression"] = regression_match.group(1)

    methods = [(kind, labels_by_kind.get(kind, "")) for kind in kinds]
    labels = [label for _, label in methods]
    if any(not label for label in labels) or len(set(labels)) != len(labels):
        raise ValueError(f"catalog labels are missing or duplicated: {methods!r}")
    return methods


class NativeCatalogueContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.catalog_source = (ROOT / "src/native/nativeAnalysisCatalog.ts").read_text(
            encoding="utf-8"
        )
        cls.recipe_source = (ROOT / "src/native/nativeAnalysisRecipe.ts").read_text(
            encoding="utf-8"
        )
        cls.calculation_mode_source = (
            ROOT / "src/native/nativeCalculationMode.ts"
        ).read_text(encoding="utf-8")

    def test_canonical_catalogue_is_exactly_fifteen_methods_with_cta(self) -> None:
        methods = derive_catalogue(
            self.catalog_source,
            self.recipe_source,
            self.calculation_mode_source,
        )
        self.assertEqual([kind for kind, _ in methods], EXPECTED_KIND_ORDER)
        self.assertEqual(dict(methods)["cta_pls"], "Confirmatory Tetrad Analysis")
        self.assertEqual(len(methods), 15)

    def test_catalogue_identity_mutations_fail_closed(self) -> None:
        mutations = {
            "cta_removed": (
                self.catalog_source.replace(
                    '    kind: "cta_pls",', '    kind: "cta_removed",', 1
                ),
                self.recipe_source,
                self.calculation_mode_source,
            ),
            "cta_reordered": (
                self.catalog_source.replace('    kind: "cca",', '    kind: "swap",', 1)
                .replace('    kind: "cta_pls",', '    kind: "cca",', 1)
                .replace('    kind: "swap",', '    kind: "cta_pls",', 1),
                self.recipe_source,
                self.calculation_mode_source,
            ),
            "cta_label_missing": (
                self.catalog_source,
                self.recipe_source.replace(
                    'label: "Confirmatory Tetrad Analysis"',
                    "label: NATIVE_CTA_LABEL",
                    1,
                ),
                self.calculation_mode_source,
            ),
            "cta_label_duplicated": (
                self.catalog_source,
                self.recipe_source.replace(
                    'label: "Confirmatory Tetrad Analysis"',
                    'label: "CCA composite residual diagnostics"',
                    1,
                ),
                self.calculation_mode_source,
            ),
        }
        for name, sources in mutations.items():
            with self.subTest(name=name):
                with self.assertRaises(ValueError):
                    derive_catalogue(*sources)

    def test_acceptance_harnesses_derive_labels_and_cover_cta_setup(self) -> None:
        packaged = (VALIDATION / "v247_tauri_native_acceptance.mjs").read_text(
            encoding="utf-8"
        )
        visual = (
            VALIDATION / "v247_native_desktop_visual_acceptance.mjs"
        ).read_text(encoding="utf-8")

        for source in (packaged, visual):
            self.assertIn("EXPECTED_NATIVE_CALCULATION_KIND_ORDER", source)
            self.assertIn("async function canonicalNativeAnalysisCatalog()", source)
            self.assertIn("exact 15-kind order", source)
            self.assertIn('"cta_pls"', source)
        self.assertNotIn("const expectedOptionLabels = [", packaged)
        self.assertNotIn("const nativeCalculationMethods = [", visual)
        self.assertNotIn('check.countStatus !== "14 methods"', visual)

        for token in (
            'query: "confirmatory tetrad"',
            'expectedKind: "cta_pls"',
            "async function auditCtaPlsDialog",
            'selectCalculationMethod(dialog, "cta_pls")',
            'capture(page, "cta-pls-dialog"',
            'name: "Start tetrad diagnostics"',
            "descriptiveOnlyBoundary",
            "missingEligibleBlockerVisible",
        ):
            self.assertIn(token, visual)
        for token in (
            "const ctaPlsOption",
            "descriptiveTetradScope",
            "four-or-more-indicator PLS blocks",
            "without inferential classification",
        ):
            self.assertIn(token, packaged)


if __name__ == "__main__":
    unittest.main()
