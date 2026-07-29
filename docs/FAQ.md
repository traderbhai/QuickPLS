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

No. QuickPLS is designed for offline Windows desktop use.

## Does QuickPLS require R?

No. R/Rscript and R packages are validation-only development tools.

## Why does Windows warn about the installer?

The current installer is unsigned. A code-signing certificate is planned as a separate release-hardening step.

## What export formats are supported?

CSV, HTML, XLSX, SVG publication diagram, and browser print-to-PDF workflow.

Native PDF/PNG export is not promoted unless separately audited.

## Why are some methods unavailable?

QuickPLS evaluates each method against the current dataset, model, and settings. Setup shows whether a method is recommended, available after setup, not applicable, unsupported, or experimental, with the exact reason and next action.
