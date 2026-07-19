# QuickPLS 1.0.0 Quick Start

## 1. Install

Download `QuickPLS_1.0.0_x64-setup.exe` from the GitHub Release and run it.

The installer is unsigned. Windows may show a SmartScreen warning until a code-signing certificate is added.

## 2. Start A Project

Open QuickPLS and choose `New`.

## 3. Import Data

Use `Import` to load a CSV, TSV, XLSX, SAV, covariance, or correlation file supported by the documented v1.0 scope.

## 4. Build The SEM Diagram

In the SEM designer:

- Drag variables to the canvas to create constructs and indicators.
- Drag variables onto existing constructs to assign indicators.
- Move constructs and indicators directly on the canvas.
- Use the path tool to draw structural paths.
- Use the covariance tool only where the visual/model scope supports it.

## 5. Run Analysis

Choose an analysis method and click `Run`.

Before a run exists, diagrams show model structure only. After a compatible saved run is selected, QuickPLS can show loadings, path coefficients, and R² values on the diagram.

## 6. Review Results

Use:

- Saved Runs for run history and method payloads.
- Reports for tables, comparison, SVG diagram export, and browser print-to-PDF.
- Warnings and provenance to confirm the run is inside the documented supported scope.

## 7. Export

Supported v1.0 export surfaces:

- CSV
- HTML
- XLSX
- SVG publication diagram
- Browser print-to-PDF path

Native CLI PDF/PNG export is not part of v1.0.0.

