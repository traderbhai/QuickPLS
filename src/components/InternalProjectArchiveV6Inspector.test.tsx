import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { InternalProjectArchiveV6ReadOutcomeV1 } from "../domain/internalProjectArchiveV6Read";
import {
  createInternalProjectArchiveV6InspectorState,
  InternalProjectArchiveV6Inspector,
  InternalProjectArchiveV6InspectorView,
  internalProjectArchiveV6InspectorReducer,
  resolveInternalProjectArchiveV6Inspection,
  type InternalProjectArchiveV6InspectionSummary,
  type InternalProjectArchiveV6InspectorState,
} from "./InternalProjectArchiveV6Inspector";

const summary: InternalProjectArchiveV6InspectionSummary = {
  access: "read_only",
  archivePath: "D:\\projects\\customer-study-v6.qpls",
  archiveSha256: "a".repeat(64),
  archiveBytes: 12_345,
  projectName: "Customer study",
  projectId: "00000000-0000-0000-0000-000000000601",
  createdAt: "2026-08-15T10:00:00Z",
  modifiedAt: "2026-08-15T10:01:00Z",
  engineVersion: "quickpls-test",
  counts: {
    datasets: 2,
    models: 3,
    recipes: 4,
    historicalRecipes: 5,
    historicalResults: 6,
    canonicalResultDocuments: 7,
  },
  sourceRecheckedUnchanged: true,
};

const noop = () => undefined;

function view(state: InternalProjectArchiveV6InspectorState, nativeDesktop = true) {
  return renderToStaticMarkup(<InternalProjectArchiveV6InspectorView
    nativeDesktop={nativeDesktop}
    state={state}
    onArchivePathChange={noop}
    onBrowse={noop}
    onInspect={noop}
  />);
}

describe("Internal/Labs schema-6 archive inspector", () => {
  it("fails closed unless the existing Experimental Labs preference is enabled", () => {
    const services = {
      chooseAndInspect: vi.fn(),
      inspectAt: vi.fn(),
    };

    const html = renderToStaticMarkup(<InternalProjectArchiveV6Inspector
      experimentalLabsEnabled={false}
      nativeDesktopOverride
      services={services}
    />);

    expect(html).toBe("");
    expect(services.chooseAndInspect).not.toHaveBeenCalled();
    expect(services.inspectAt).not.toHaveBeenCalled();
  });

  it("renders the workflow when the caller supplies the enabled Labs preference", () => {
    const html = renderToStaticMarkup(<InternalProjectArchiveV6Inspector
      experimentalLabsEnabled
      nativeDesktopOverride
    />);

    expect(html).toContain('data-internal-schema6-archive-inspector="read-only"');
    expect(html).toContain("Labs · read-only");
  });

  it("renders a labelled native path workflow and explicit non-activation limits", () => {
    const html = view(createInternalProjectArchiveV6InspectorState());

    expect(html).toContain('role="region"');
    expect(html).toContain('aria-labelledby="internal-schema6-archive-inspector-heading"');
    expect(html).toContain('<label for="internal-schema6-archive-path"');
    expect(html).toContain('aria-describedby="internal-schema6-archive-path-help internal-schema6-read-only-limits"');
    expect(html).toContain("Choose and inspect…");
    expect(html).toContain("Inspect provided path");
    expect(html).toContain('role="status"');
    expect(html).toContain('aria-live="polite"');
    expect(html).toContain("not active, cannot be edited, and cannot be saved");
    expect(html).toContain("never replaces the current project");
  });

  it("disables native controls and explains the missing desktop boundary", () => {
    const html = view(createInternalProjectArchiveV6InspectorState("D:\\study.qpls"), false);

    expect(html).toContain("Native desktop required");
    expect(html).toContain("No browser fallback activates or reads a project");
    expect(html).toContain('id="internal-schema6-archive-path"');
    expect(html.match(/disabled=""/g)?.length).toBeGreaterThanOrEqual(3);
  });

  it("announces loading and errors while keeping the path controls unavailable", () => {
    const loading = internalProjectArchiveV6InspectorReducer(
      createInternalProjectArchiveV6InspectorState("D:\\study.qpls"),
      { type: "started", statusMessage: "Validating locally…" },
    );
    const loadingHtml = view(loading);
    expect(loadingHtml).toContain('aria-busy="true"');
    expect(loadingHtml).toContain('aria-label="Archive inspection in progress"');
    expect(loadingHtml).toContain("Validating schema-6 ZIP");
    expect(loadingHtml.match(/disabled=""/g)?.length).toBeGreaterThanOrEqual(3);

    const failed = internalProjectArchiveV6InspectorReducer(loading, {
      type: "failed",
      failure: {
        code: "schema6_archive_read.invalid_archive",
        message: "Archive validation failed.",
        correctiveAction: "Restore a trusted schema-6 ZIP.",
      },
    });
    const failedHtml = view(failed);
    expect(failedHtml).toContain('role="alert"');
    expect(failedHtml).toContain("Archive validation failed.");
    expect(failedHtml).toContain("Restore a trusted schema-6 ZIP.");
    expect(failedHtml).toContain("schema6_archive_read.invalid_archive");
    expect(failedHtml).not.toContain("Inspected project identity");
  });

  it("shows verified identity and every exact dataset, model, recipe, and result count", () => {
    const ready = internalProjectArchiveV6InspectorReducer(
      createInternalProjectArchiveV6InspectorState(),
      { type: "succeeded", summary },
    );
    const html = view(ready);

    expect(html).toContain('data-inspection-state="ready"');
    expect(html).toContain("Read-only snapshot");
    expect(html).toContain("Customer study");
    expect(html).toContain(summary.projectId);
    expect(html).toContain("customer-study-v6.qpls");
    expect(html).toContain("12,345");
    expect(html).toContain("Exact validated content counts");
    expect(html).toContain("Datasets</span><strong>2</strong>");
    expect(html).toContain("Models</span><strong>3</strong>");
    expect(html).toContain("Recipes</span><strong>4</strong>");
    expect(html).toContain("Historical recipes</span><strong>5</strong>");
    expect(html).toContain("Historical results</span><strong>6</strong>");
    expect(html).toContain("Canonical result documents</span><strong>7</strong>");
    expect(html).toContain("No active project, workspace selection, editable model, save target, autosave state, or recovery state was changed.");
    expect(html).not.toContain("Save project");
  });

  it("maps cancellation, strict-reader blocks, success, and thrown boundary errors into terminal UI actions", async () => {
    await expect(resolveInternalProjectArchiveV6Inspection(async () => null))
      .resolves.toEqual({ type: "cancelled" });

    const blocked: InternalProjectArchiveV6ReadOutcomeV1 = {
      status: "blocked",
      diagnostic: {
        code: "schema6_archive_read.invalid_archive",
        message: "Archive validation failed.",
        correctiveAction: "Restore a trusted schema-6 ZIP.",
      },
    };
    await expect(resolveInternalProjectArchiveV6Inspection(async () => blocked))
      .resolves.toEqual({ type: "failed", failure: blocked.diagnostic });

    const ok = {
      status: "ok",
      value: {
        access: "read_only",
        archivePath: summary.archivePath,
        archiveSha256: summary.archiveSha256,
        archiveBytes: summary.archiveBytes,
        manifest: {
          name: summary.projectName,
          project_id: summary.projectId,
          created_at: summary.createdAt,
          modified_at: summary.modifiedAt,
          engine_version: summary.engineVersion,
        },
        counts: summary.counts,
        sourceRecheckedUnchanged: true,
      },
    } as unknown as InternalProjectArchiveV6ReadOutcomeV1;
    await expect(resolveInternalProjectArchiveV6Inspection(async () => ok))
      .resolves.toEqual({ type: "succeeded", summary });

    const boundaryError = Object.assign(new Error("Malformed native response."), {
      code: "schema6_archive_read.field_missing",
    });
    await expect(resolveInternalProjectArchiveV6Inspection(async () => {
      throw boundaryError;
    })).resolves.toMatchObject({
      type: "failed",
      failure: {
        code: "schema6_archive_read.field_missing",
        message: "Malformed native response.",
      },
    });
  });
});
