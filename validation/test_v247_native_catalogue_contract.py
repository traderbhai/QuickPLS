from __future__ import annotations

import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
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
    if not kinds or len(kinds) != len(set(kinds)):
        raise ValueError(f"catalog kind order is empty or duplicated: {kinds!r}")

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

    def test_execution_adapter_order_is_derived_and_includes_cta_and_power(self) -> None:
        methods = derive_catalogue(
            self.catalog_source,
            self.recipe_source,
            self.calculation_mode_source,
        )
        self.assertEqual(
            [kind for kind, _ in methods],
            list(dict.fromkeys(kind for kind, _ in methods)),
        )
        self.assertEqual(dict(methods)["cta_pls"], "Confirmatory Tetrad Analysis")
        self.assertEqual(
            dict(methods)["pls_sample_size_power"], "PLS-SEM Sample Size and Power"
        )

    def test_catalogue_identity_mutations_fail_closed(self) -> None:
        mutations = {
            "cta_removed": (
                self.catalog_source.replace(
                    '    kind: "cta_pls",', '    kind: "cta_removed",', 1
                ),
                self.recipe_source,
                self.calculation_mode_source,
            ),
            "cta_duplicated": (
                self.catalog_source.replace(
                    '    kind: "cta_pls",', '    kind: "cca",', 1
                ),
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
            self.assertIn("async function canonicalNativeAnalysisCatalog()", source)
            self.assertIn("execution-adapter order must be non-empty and unique", source)
            self.assertIn("standardSupplementalKinds", source)
            self.assertNotIn("EXPECTED_NATIVE_CALCULATION_KIND_ORDER", source)
            self.assertIn('"cta_pls"', source)
            self.assertIn('"plsc_bootstrap"', source)
            self.assertIn('"pls_posthoc_technical_minimum_sample_size"', source)
            self.assertIn('"pls_sample_size_power"', source)
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
            "const scopeText",
            "scopeText !== ctaPlsScopeNote",
            'eligibleBlockText !== "Predictor: 4 indicators, 3 tetrads"',
            "does not classify blocks or calculate bootstrap, permutation, asymptotic, or vanishing-tetrad decisions",
        ):
            self.assertIn(token, packaged)


if __name__ == "__main__":
    unittest.main()
