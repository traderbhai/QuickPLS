import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const read = (path: string) => readFileSync(path, "utf8");

describe("native desktop command surface contracts", () => {
  it("keeps launcher actions in the single native command bar", () => {
    const app = read("src/native/NativeDesktopApp.tsx");
    const styles = read("src/native/nativeDesktop.css");

    expect(app).toContain('className="nd-commandbar" role="toolbar"');
    expect(app).toContain('const hasUnsavedProject = projectName !== "No project open"');
    expect(app).toContain('<h2 id="recent-heading">Recent projects</h2>');
    expect(app).not.toContain('<button className="primary" onClick={onNew}');
    expect(app).not.toContain('<button onClick={onOpen}');
    expect(styles).not.toContain(".nd-launch-actions button");
  });

  it("gives model indicators and variable rows equivalent pointer and keyboard actions", () => {
    const app = read("src/native/NativeDesktopApp.tsx");
    const data = read("src/native/NativeDataSurface.tsx");

    expect(app).toContain('id="nd-indicator-drag-help"');
    expect(app).toContain('draggable={!isGroupingVariable}');
    expect(app).toContain('disabled={isGroupingVariable}');
    expect(app).toContain('"Grouping variable; unavailable as an indicator"');
    expect(app).toContain('onClick={() => activateIndicator(column)}');
    expect(app).toContain('setSelectedNode(owner.id)');
    expect(app).toContain('assignIndicator(selectedAssignableConstruct.id, variable)');
    expect(app).toContain('addConstruct(undefined, [variable])');
    expect(app).toContain('aria-label={isGroupingVariable ? `${column}. ${action}` : `${column}. ${action}; or drag to the model canvas or a construct`}');
    expect(app).toContain("Drag an indicator to the canvas or onto a construct.");
    expect(data).toContain('tabIndex={0} aria-selected={selectedColumn === column}');
    expect(data).toContain('event.key === "Enter" || event.key === " "');
  });

  it("does not duplicate calculation commands in properties or empty results", () => {
    const app = read("src/native/NativeDesktopApp.tsx");
    const results = read("src/native/NativeResultsSurface.tsx");

    expect(app).not.toContain('className="nd-properties-footer"');
    expect(results).not.toContain("onCalculate");
    expect(results).not.toContain("<Calculator");
    expect(results).toContain("No completed calculation");
  });

  it("clears terminal calculation state when its dialog closes", () => {
    const app = read("src/native/NativeDesktopApp.tsx");

    expect(app).toContain('const closingCalculation = currentDialogRef.current === "calculation"');
    expect(app).toContain("if (closingCalculation && !isNativeCalculationActive(useWorkspace.getState().runMonitor.status)) resetRunMonitor()");
  });

  it("keeps trust and provenance text out of the primary results diagram", () => {
    const results = read("src/native/NativeResultsSurface.tsx");

    expect(results).toContain("showValidationWatermark: false");
    expect(results).toContain("showUnsupportedWarning: false");
    expect(results).toContain("showRunProvenance: false");
  });

  it("collapses the optional properties pane at the compact desktop breakpoint", () => {
    const app = read("src/native/NativeDesktopApp.tsx");

    expect(app).toContain('const COMPACT_PANE_MEDIA_QUERY = "(max-width: 1100px)"');
    expect(app).toContain('!window.matchMedia(COMPACT_PANE_MEDIA_QUERY).matches');
    expect(app).toContain('if (event.matches) setPropertiesOpen(false)');
  });

  it("starts inside the Windows work area with a safe restored size", () => {
    const tauri = JSON.parse(read("src-tauri/tauri.conf.json"));
    const window = tauri.app.windows[0];

    expect(window).toMatchObject({
      width: 1280,
      height: 720,
      minWidth: 1024,
      minHeight: 700,
      maximized: true,
      resizable: true,
    });
  });

  it("provides truthful native data setup and versioned variable metadata editing", () => {
    const app = read("src/native/NativeDesktopApp.tsx");
    const dialog = read("src/native/NativeDataImportDialog.tsx");
    const data = read("src/native/NativeDataSurface.tsx");
    const tauri = read("src-tauri/src/lib.rs");

    expect(app).toContain('"import-data"');
    expect(dialog).toContain('name="data-kind"');
    expect(dialog).toContain("Study sample size");
    expect(dialog).toContain("Missing-value markers");
    expect(data).toContain("updateNativeColumnMetadata(dataset.id, selectedColumn, validation.metadata)");
    expect(data).toContain("Missing markers are applied when the file is imported.");
    expect(data).not.toContain('update("missingMarkers"');
    expect(tauri).toContain("fn version_column_metadata(");
    expect(tauri).toContain("commit_dataset_version(project, version, record)");
  });

  it("makes the post-import next step explicit without duplicating command behavior", () => {
    const app = read("src/native/NativeDesktopApp.tsx");
    const data = read("src/native/NativeDataSurface.tsx");
    const commands = read("src/native/nativeCommands.ts");

    expect(app).toContain('onNewModel={() => dispatchNativeAction({ id: "explorer.new-model" })}');
    expect(app).toContain('onAnalyze={() => dispatchNativeAction({ id: "calculation.open" })}');
    expect(data).toContain("Choose what to do next");
    expect(data).toContain("Build a path model, or analyze observed variables without creating a model.");
    expect(data).toContain(">New Model…</button>");
    expect(data).toContain(">Analyze…</button>");
    expect(commands).toMatch(/const canOpenCalculation:[\s\S]*context\.projectWritable/);
  });

  it("keeps dataset lineage authoritative and recode native-only", () => {
    const app = read("src/native/NativeDesktopApp.tsx");
    const data = read("src/native/NativeDataSurface.tsx");
    const dialog = read("src/native/NativeRecodeDialog.tsx");
    const service = read("src/services/projectService.ts");

    expect(data).toContain('role="list" aria-label="Dataset versions"');
    expect(data).toContain('className="nd-version-item" role="listitem"');
    expect(data).toContain('aria-label="Search variables"');
    expect(data).toContain("await activateNativeDataset(datasetId)");
    expect(app).toContain('case "data.recode": {');
    expect(app).toContain('openDialog("recode-data")');
    expect(app).toContain("commitDatasetVersion(mutation)");
    expect(dialog).toContain("Browser preview cannot write dataset versions.");
    expect(dialog).toContain("validateNativeRecodeDraft(dataset, draft)");
    expect(service).toContain('invoke<Dataset>("activate_dataset", { datasetId })');
    expect(service).toContain('invoke<DatasetVersionMutation>("recode_dataset_column", { datasetId, spec })');
    expect(dialog).not.toMatch(/dataset\.rows\s*(?:=|\.|\[)/);
  });

  it("targets Data context commands at the invoked variable and excludes blank/version targets from Recode", () => {
    const app = read("src/native/NativeDesktopApp.tsx");
    const data = read("src/native/NativeDataSurface.tsx");
    const context = read("src/native/nativeDataContext.ts");

    expect(data).toContain("dataContextTarget(event.target, event.currentTarget)");
    expect(data).toContain("setSelectedColumn(target.column)");
    expect(data).toContain("target,");
    expect(data).toContain("data-native-variable={column}");
    expect(data).toContain("data-native-dataset={item.dataset.id}");
    expect(context).toContain('if (target.kind === "variable") return { kind: "variable", count: 1 }');
    expect(context).toContain('if (target.kind === "dataset") return { kind: "dataset", count: 1 }');
    expect(context).toContain('return { kind: "none", count: 0 }');
    expect(app).toContain("nativeDataContextSelection(request.target)");
    expect(app).toContain("dispatchNativeAction(command.action, contextMenu?.target)");
    expect(app).toContain('target?.kind === "variable" ? target.column : selectedColumn');
  });

  it("guards Data mutations and scopes Recode completion to its originating dialog", () => {
    const app = read("src/native/NativeDesktopApp.tsx");
    const data = read("src/native/NativeDataSurface.tsx");
    const dialog = read("src/native/NativeRecodeDialog.tsx");

    expect(app).toContain("const dataMutationsLocked = isNativeCalculationActive(runMonitor.status)");
    expect(data).toContain('<DataPaneTitle title="Variables" />');
    expect(data).not.toContain('title="Variables" action=');
    expect(data).toContain("disabled={Boolean(activatingDatasetId) || mutationsLocked}");
    expect(data).toContain("if (datasetId === dataset.id || activatingDatasetId || mutationsLocked) return");
    expect(app).toContain('dialog === "recode-data" ? !recodeBusy');
    expect(app).toContain("scope !== dialogScopeRef.current");
    expect(dialog).toContain('disabled={status === "saving"} onClick={close}');
    expect(dialog).toContain("runNativeScopedSubmission");
    expect(dialog).toContain("complete(dialogScope)");
    expect(dialog).toContain('aria-busy={status === "saving"}');
    expect(dialog).toContain("nativeRecodeIssueFieldId(fieldPrefix, firstPath)");
    expect(dialog).toContain("aria-invalid={invalid(\"targetColumn\")}");
  });
});
