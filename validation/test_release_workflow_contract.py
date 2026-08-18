from __future__ import annotations

import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RELEASE = ROOT / ".github" / "workflows" / "release.yml"
SECURITY = ROOT / ".github" / "workflows" / "security.yml"
EVIDENCE_VERIFY = ROOT / ".github" / "workflows" / "evidence-verify.yml"
SECURITY_REQUIREMENTS = ROOT / "validation" / "security-tools-requirements.txt"
ACTION_PIN = re.compile(r"^[ \t]*uses:\s*[^\s@]+@[0-9a-f]{40}\s*$", re.MULTILINE)
ANY_ACTION = re.compile(r"^[ \t]*uses:\s*(\S+)\s*$", re.MULTILINE)
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class ReleaseWorkflowContractTests(unittest.TestCase):
    def text(self, path: Path) -> str:
        return path.read_text(encoding="utf-8")

    def test_commercial_workflow_actions_are_commit_pinned(self) -> None:
        for workflow in (RELEASE, SECURITY, EVIDENCE_VERIFY):
            with self.subTest(workflow=workflow.name):
                content = self.text(workflow)
                actions = ANY_ACTION.findall(content)
                self.assertTrue(actions)
                self.assertEqual(len(actions), len(ACTION_PIN.findall(content)))

    def test_release_is_manual_review_artifact_assembly_not_publication(self) -> None:
        content = self.text(RELEASE)
        self.assertRegex(content, r"(?m)^on:\s*\n\s+workflow_dispatch:")
        self.assertIn("contents: read", content)
        self.assertNotIn("contents: write", content)
        self.assertNotIn("releases: write", content)
        lowered = content.lower()
        for forbidden in (
            "gh release",
            "actions/create-release",
            "softprops/action-gh-release",
            "api.github.com/repos",
            "/releases",
        ):
            with self.subTest(forbidden=forbidden):
                self.assertNotIn(forbidden, lowered)
        self.assertIn("actions/upload-artifact@", content)
        self.assertIn("without publishing", lowered)

    def test_scanner_bootstrap_is_hash_and_version_pinned(self) -> None:
        expected_checksum = "700c2b240f7fd330c24b675fe429f73a5b676531fcc6300400b2b67f155ba12a"
        for workflow in (RELEASE, SECURITY):
            with self.subTest(workflow=workflow.name):
                content = self.text(workflow)
                self.assertIn(
                    "python -m pip install --require-hashes -r validation/security-tools-requirements.txt",
                    content,
                )
                self.assertNotRegex(content, r"pip install\s+(?:pip-audit|detect-secrets)")
                self.assertIn('CARGO_AUDIT_VERSION: "0.22.2"', content)
                self.assertIn(f'CARGO_AUDIT_CRATE_SHA256: "{expected_checksum}"', content)
                self.assertIn("cargo install cargo-audit --locked --version $env:CARGO_AUDIT_VERSION", content)
                self.assertIn("cargo-audit --version", content)
                self.assertIn("Get-FileHash -Algorithm SHA256", content)

    def test_every_python_scan_tool_requirement_has_a_sha256_hash(self) -> None:
        content = self.text(SECURITY_REQUIREMENTS)
        requirement_count = 0
        current: str | None = None
        hashes: list[str] = []
        for line in content.splitlines():
            if line and not line[0].isspace() and not line.startswith("#") and "==" in line:
                if current is not None:
                    self.assertTrue(hashes, f"missing hashes for {current}")
                    self.assertTrue(all(SHA256.fullmatch(value) for value in hashes))
                current = line.split("\\", 1)[0].strip()
                hashes = []
                requirement_count += 1
            match = re.search(r"--hash=sha256:([0-9a-f]+)", line)
            if match:
                hashes.append(match.group(1))
        self.assertIsNotNone(current)
        self.assertTrue(hashes, f"missing hashes for {current}")
        self.assertTrue(all(SHA256.fullmatch(value) for value in hashes))
        self.assertGreaterEqual(requirement_count, 10)
        self.assertIn("pip-audit==2.10.1", content)
        self.assertIn("detect-secrets==1.5.0", content)

    def test_pending_then_stable_gate_order_is_fail_closed(self) -> None:
        content = self.text(RELEASE)
        pending_gate = content.index("competitor_ready -ne $false")
        signing = content.index("Sign desktop and CLI with approved managed identity")
        candidate = content.index("Assemble and independently verify signed candidate")
        stable_gate = content.index("quickpls_3_competitor_program.py --require-ready")
        self.assertLess(pending_gate, signing)
        self.assertLess(signing, candidate)
        self.assertLess(candidate, stable_gate)
        self.assertEqual(content.count("quickpls_3_competitor_program.py --require-ready"), 1)
        self.assertIn("Pre-sign validation must pass structurally while competitor_ready remains false.", content)

    def test_evidence_pack_requires_explicit_commit_and_tracked_byte_verification(self) -> None:
        source = self.text(ROOT / "validation" / "release_evidence_bundle.py")
        self.assertIn('pack.add_argument("--expected-commit", required=True)', source)
        self.assertIn("Git HEAD does not match the explicitly expected source commit", source)
        self.assertIn("tracked source differs from its identity report", source)


if __name__ == "__main__":
    unittest.main()
