# QuickPLS

QuickPLS is a free-to-use, proprietary Windows desktop application for PLS-SEM,
SEM diagrams, reproducible analysis workflows, moderation and heterogeneity
analysis, results review, and publication-oriented exports. Calculations run on
the local computer without an account, activation server, cloud storage, R, or
Python.

## QuickPLS 2.56.0 Beta (Unsigned)

Version 2.56.0 adds the **Moderation & heterogeneity** suite while preserving the
existing continuous-moderation V1 workflow and legacy project identities. This
Beta is not Authenticode-signed, is not a Stable release, does not claim
unrestricted SmartPLS parity, and must not be described as proof that every
method combination is supported.

- Release: [QuickPLS 2.56.0 Beta.1](https://github.com/traderbhai/QuickPLS/releases/tag/v2.56.0-beta.1)
- Source commit: `28939b73db8f2284f21ce184050eb3f04110bf94`
- Platform: Windows 10/11 x64

### Download

For normal use, choose the setup installer. The portable executable is useful
where installation is unavailable, but it requires a compatible Microsoft
WebView2 Runtime already installed.

| Asset | Size | SHA-256 |
| --- | ---: | --- |
| `QuickPLS_2.56.0_unsigned-preview_multimod_28939b73_x64_setup.exe` | 231,889,558 bytes (221.15 MiB) | `46b121d252ba236f4e8d72e5e618bbf4542d0cdb0819b834fb9b3d3677599523` |
| `QuickPLS_2.56.0_unsigned-preview_multimod_28939b73_x64_portable.exe` | 79,480,832 bytes (75.80 MiB) | `54abad31daca14dbad16588f85015fb39b030bac28e1cd475a10d4d4573cb2d1` |

Verify the downloaded file before running it:

```powershell
Get-FileHash .\QuickPLS_2.56.0_unsigned-preview_multimod_28939b73_x64_setup.exe -Algorithm SHA256
Get-FileHash .\QuickPLS_2.56.0_unsigned-preview_multimod_28939b73_x64_portable.exe -Algorithm SHA256
```

The executables are unsigned. Windows may identify the publisher as unknown or
show Microsoft SmartScreen. Download only from the release page above and
compare the complete SHA-256 value before continuing. Checksums detect changed
bytes; they do not establish publisher identity.

See the [installation guide](docs/INSTALLATION.md) for setup, portable use,
first launch, offline behavior, and uninstall instructions.

## What QuickPLS provides

- A native Windows workbench for data, editable SEM models, calculation, results,
  export, and strict project reopen.
- Raw, covariance, and correlation data import for workflows that admit those
  input types, with explicit missing-value and sample-size handling.
- PLS-SEM and bounded CB-SEM calculation routes governed by typed capability
  predicates rather than broad method-name claims.
- Diagram-native mediation, two-way and bounded three-way moderation,
  higher-order constructs, and supported moderated-mediation workflows.
- Deterministic seeds, immutable recipes, provenance-aware results, cancellation
  without partial scientific publication, and integrity-checked project data.
- CSV, XLSX, JSON, self-contained HTML, PDF, SVG, and PNG publication where the
  exact completed result family authorizes the format.
- Eight bundled projects, including Corporate Reputation and four
  Organizational Identification variants. Seven open as ordinary editable
  projects; the two-outcome moderation sample opens through the qualified,
  integrity-checked General SEM schema-6 workflow.

## Moderation and heterogeneity suite

Open or create a project, activate a calculation-ready model, and choose
**Moderation & heterogeneity…** from the Model toolbar. The suite contains four
separate workflows:

1. **Categorical moderation**: typed 2-20-group MGA, structured MICOM review,
   pairwise permutation comparisons, a max-spread omnibus test for three or more
   groups, multiplicity control, and separately bounded sensitivity procedures.
2. **Latent segmentation**: FIMIX-PLS V2 and separately identified PLS-POS V2
   workflows for analyst-inspected and explicitly locked `K=2…5`; QuickPLS does
   not auto-select K.
3. **Conditional process**: explicitly selected conditional indirect paths,
   multiple two-way moderation, bounded three-way, BCa or bounded studentized
   inference, and separately qualified grouped, HOC, case-weighted, and
   frequency-weighted profiles.
4. **Interventional mediation**: a separate observed-data parametric
   g-computation workflow with an explicit adjustment set, temporal-order and
   identification declarations, and positivity diagnostics. Output is labelled
   an assumption-dependent interventional estimate; it does not state that
   causality was established.

Every workflow is bounded by an exact profile. Unsupported intersections fail
closed instead of dropping an interaction, shortening a path, changing an
interval method, or silently removing groups or weights. Important exclusions
include arbitrary higher-order interactions, survey-design claims, causal
relabelling of ordinary PLS mediation, and quadratic/self-moderation in the new
MultiMod suite.

Read the exact contracts before reporting results:

- [MultiMod boundaries and qualification](docs/MULTIMOD_BOUNDARIES_AND_QUALIFICATION_V1.md)
- [Unsupported intersections and reason codes](docs/MULTIMOD_UNSUPPORTED_INTERSECTIONS_V1.md)
- [MGA Multigroup V1](docs/methods/MGA_MULTIGROUP_V1.md)
- [PLS Heterogeneity V2](docs/methods/PLS_HETEROGENEITY_V2.md)
- [General SEM Conditional Process V2](docs/methods/GENERAL_SEM_CONDITIONAL_PROCESS_V2.md)
- [Interventional Causal Mediation V1](docs/methods/INTERVENTIONAL_CAUSAL_MEDIATION_V1.md)

### Qualification authority

The Standard MultiMod surface is authorized only when an official executable
contains the release authority bound to its exact source commit and exact
profile/procedure cells. The result must visibly say **Standard ·
Release-qualified**. A locally compiled or altered executable without that
embedded authority remains **Experimental Labs · Unqualified**, even when the
source code is otherwise identical. A project file, environment variable, or UI
request cannot promote a Labs-only executable.

The permanent manifests committed to the repository intentionally remain
fail-closed Labs templates; they are not evidence that an arbitrary source build
is Standard.

## First workflow

1. Install QuickPLS or launch the portable executable.
2. Create a project, open a `.qpls` project, or choose a bundled sample.
3. Import and inspect data in **Data**.
4. Create or activate a model and author it on **Canvas**.
5. Use **Calculate** for the normal method catalogue, or choose **Moderation &
   heterogeneity…** for MultiMod.
6. Review eligibility and settings before calculation. After completion,
   QuickPLS opens the result with diagnostics, estimates, inference, exclusions,
   failures, provenance, and required evidence inventories.
7. Save the project and export only the formats enabled for the exact completed
   result.

## Offline and independence boundary

QuickPLS product code performs analysis locally and product telemetry is
disabled. The QuickPLS application/page is configured to make no external
requests during the documented offline workflow. The separately managed
Microsoft WebView2 Runtime may still perform its own background service
connections unless Windows or an independently verified OS-level network policy
blocks them; this Beta does not claim zero egress for the complete WebView2
process tree.

QuickPLS is independently implemented from published methods and permitted
documentation. It does not import SmartPLS project files, reverse-engineer
SmartPLS, claim undocumented SmartPLS equivalence, or claim affiliation with
SmartPLS GmbH or any referenced third-party project.

## Documentation

- [Installation](docs/INSTALLATION.md)
- [Quick Start](docs/QUICK_START.md)
- [User Guide](docs/USER_GUIDE.md)
- [QuickPLS 2.56.0 Features](docs/FEATURES_V2_56_0.md)
- [QuickPLS 2.56.0 Release Notes](docs/RELEASE_NOTES_V2_56_0.md)
- [Release checksums](docs/RELEASE_CHECKSUMS_V2_56_0.txt)
- [Machine-readable release manifest](docs/RELEASE_MANIFEST_V2_56_0.json)
- [GitHub release receipt](docs/RELEASE_RECEIPT_V2_56_0.json)
- [Method Compatibility](docs/METHOD_COMPATIBILITY.md)
- [Known Issues](docs/KNOWN_ISSUES.md)
- [Privacy Notice](docs/PRIVACY_NOTICE.md)
- [Support Policy](docs/SUPPORT_POLICY.md)
- [Security Policy](SECURITY.md)
- [Changelog](CHANGELOG.md)

## Source, license, and support

QuickPLS source is publicly visible for transparency, validation review,
learning, and contribution proposals under its proprietary source-available
license. It is not open-source software. Official QuickPLS releases are free to
use for research, teaching, learning, and internal analysis subject to the
[license](LICENSE.md) and [EULA](EULA.md). Redistribution and publication of
modified builds are restricted.

Report reproducible product defects through the
[issue tracker](https://github.com/traderbhai/QuickPLS/issues/new/choose).
Do not attach confidential datasets or unpublished identifiable research data.
Report suspected vulnerabilities privately through a
[GitHub security advisory](https://github.com/traderbhai/QuickPLS/security/advisories/new).
