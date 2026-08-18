from __future__ import annotations

import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]


class FrontendBuildAccelerationContractTests(unittest.TestCase):
    def test_production_typecheck_only_partitions_test_sources(self) -> None:
        root_config = json.loads((ROOT / "tsconfig.json").read_text(encoding="utf-8"))
        app_config = json.loads((ROOT / "tsconfig.app.json").read_text(encoding="utf-8"))
        build_config = json.loads((ROOT / "tsconfig.build.json").read_text(encoding="utf-8"))

        self.assertEqual(
            root_config["references"],
            [{"path": "./tsconfig.app.json"}, {"path": "./tsconfig.node.json"}],
        )
        self.assertEqual(build_config["extends"], "./tsconfig.app.json")
        self.assertEqual(app_config["include"], ["src"])
        self.assertEqual(build_config["include"], ["src/main.tsx", "src/vite-env.d.ts"])
        self.assertEqual(
            build_config["compilerOptions"],
            {
                "incremental": True,
                "tsBuildInfoFile": "./tsconfig.build.tsbuildinfo",
            },
        )
        self.assertEqual(
            build_config["exclude"],
            [
                "src/**/*.test.ts",
                "src/**/*.test.tsx",
                "src/**/*.testFixture.ts",
                "src/**/*.testFixture.tsx",
            ],
        )

    def test_package_keeps_full_and_build_typechecks_separate(self) -> None:
        package = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))
        scripts = package["scripts"]

        self.assertEqual(
            scripts["typecheck:build"],
            "tsc -p tsconfig.build.json && tsc -b tsconfig.node.json",
        )
        self.assertEqual(scripts["typecheck:full"], "tsc -b")
        self.assertEqual(scripts["build:bundle"], "vite build")
        self.assertEqual(scripts["build"], "npm run typecheck:build && npm run build:bundle")
        self.assertEqual(scripts["test"], "vitest run")

    def test_ci_runs_the_full_typecheck_exactly_once(self) -> None:
        workflow = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")

        self.assertEqual(workflow.count("npm run typecheck:full"), 1)
        self.assertIn("- name: Frontend full typecheck (source and tests)", workflow)
        self.assertEqual(workflow.count("npm run build:bundle"), 1)
        self.assertNotIn("npm run typecheck:build", workflow)
        self.assertNotIn("npm run build\n", workflow)


if __name__ == "__main__":
    unittest.main()
