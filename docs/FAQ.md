# QuickPLS FAQ

## Is QuickPLS free?

Yes. Official QuickPLS releases are free to use.

## Is QuickPLS open source?

No. QuickPLS is proprietary source-available software. The source can be inspected and contributions can be proposed, but redistribution and derived commercial use are restricted by `LICENSE.md` and `EULA.md`.

## Does QuickPLS import SmartPLS projects?

No.

## Does QuickPLS guarantee identical results to SmartPLS?

No. QuickPLS implements published methods independently and validates against documented formulas, fixtures, simulations, and reference engines where permitted.

## Does QuickPLS require internet?

No account, cloud service, or remote computation is required. QuickPLS analytical workflows run locally after download. The Microsoft-managed WebView2 runtime may make its own background service connections unless Windows network controls block them.

## Does QuickPLS require R?

No. R/Rscript and R packages are validation-only development tools.

## Why does Windows warn about the installer?

The public `v2.50.0` GitHub pre-release is unsigned. Download it only from the [official release page](https://github.com/traderbhai/QuickPLS/releases/tag/v2.50.0), compare its SHA-256 value with the attached checksum file, and expect Windows SmartScreen to identify the publisher as unknown. Code signing is planned as a separate release-hardening step.

## What export formats are supported?

Canonical General SEM results support CSV, XLSX, self-contained HTML, PDF, SVG, and PNG. Other completed methods expose only the formats appropriate to their result type, including SVG or Windows Print/PDF where documented.

## Why are some methods unavailable?

QuickPLS evaluates each method against the current dataset, model, and settings. Setup shows whether a method is recommended, available after setup, not applicable, unsupported, or experimental, with the exact reason and next action.

## What was added in Version 2.50?

Version 2.50 integrates bounded General SEM mediation and simultaneous moderation, higher-order PLS point/bootstrap, two-way moderated mediation, recursive common-factor CB-SEM ML, and recursive-SEM case bootstrap into the same Canvas, Calculate, Results, export, and schema-6 reopen workflow. See the [release notes](RELEASE_NOTES_V2_50_0.md).

## Is QuickPLS identical to SmartPLS?

No. QuickPLS offers a familiar graphical SEM workflow and independently implements documented methods. It does not import SmartPLS projects, copy proprietary implementation details, or promise identity for undocumented behavior.
