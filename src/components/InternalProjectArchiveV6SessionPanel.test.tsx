import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { InternalProjectArchiveV6ReadSnapshotV1 } from "../domain/internalProjectArchiveV6Read";
import type { InternalProjectArchiveV6Wire } from "../domain/internalProjectArchiveV6Wire";
import type { InternalProjectArchiveV6ReadOnlySession } from "../internalProjectArchiveV6SessionStore";
import {
  InternalProjectArchiveV6SessionPanel,
  InternalProjectArchiveV6SessionView,
} from "./InternalProjectArchiveV6SessionPanel";

const project = {
  models: [0, 1, 2].map((index) => ({
    model_id: `legacy:${index}`,
    payload: { kind: "legacy_estimand_unspecified" },
  })),
} as unknown as InternalProjectArchiveV6Wire;

const snapshot = {
  access: "read_only",
  archivePath: "D:\\projects\\customer-study-v6.qpls",
  archiveSha256: "a".repeat(64),
  manifest: {
    name: "Customer study",
    project_id: "00000000-0000-0000-0000-000000000601",
  },
  counts: {
    datasets: 2,
    models: 3,
    recipes: 4,
    historicalResults: 5,
    canonicalResultDocuments: 6,
  },
  sourceRecheckedUnchanged: true,
  project,
} as unknown as InternalProjectArchiveV6ReadSnapshotV1;

const session: InternalProjectArchiveV6ReadOnlySession = {
  kind: "internal_schema6_read_only",
  access: "read_only",
  snapshot,
  originSnapshot: snapshot,
  project,
  standardActivation: null,
  capabilities: {
    edit: false,
    ephemeralModelAuthorityMutation: false,
    compile: false,
    run: false,
    save: false,
    saveAs: false,
    saveCopy: "new_destination_only",
    autosave: false,
    recovery: false,
  },
};

const noop = () => undefined;

function view(overrides: Partial<Parameters<typeof InternalProjectArchiveV6SessionView>[0]> = {}) {
  return renderToStaticMarkup(<InternalProjectArchiveV6SessionView
    nativeDesktop
    archivePath=""
    state={{
      phase: "inactive",
      session: null,
      failure: null,
      statusMessage: "No schema-6 read-only session is active.",
      dirty: false,
      persistence: null,
      modelMutationPending: false,
      standardActivationPending: false,
      standardActivationFailure: null,
      standardActivationStatusMessage: "Not activated.",
      saveCopyPending: false,
      saveCopyFailure: null,
      saveCopyStatusMessage: "No schema-6 copy has been saved from this session.",
    }}
    onArchivePathChange={noop}
    onBrowseAndOpen={noop}
    onOpenAtPath={noop}
    onSaveCopy={noop}
    onActivateStandard={noop}
    onForkRevision={noop}
    onCloseStandardProject={noop}
    onClose={noop}
    {...overrides}
  />);
}

describe("Internal/Labs schema-6 read-only session panel", () => {
  it("fails closed when Experimental Labs is disabled", () => {
    const services = { chooseAndRead: vi.fn(), readAt: vi.fn(), chooseAndSaveCopy: vi.fn() };
    const html = renderToStaticMarkup(<InternalProjectArchiveV6SessionPanel
      experimentalLabsEnabled={false}
      nativeDesktopOverride
      services={services}
    />);

    expect(html).toBe("");
    expect(services.chooseAndRead).not.toHaveBeenCalled();
    expect(services.readAt).not.toHaveBeenCalled();
    expect(services.chooseAndSaveCopy).not.toHaveBeenCalled();
  });

  it("renders explicit isolation and no-persistence limits", () => {
    const html = view();

    expect(html).toContain('data-internal-schema6-session="inactive"');
    expect(html).toContain("Open safely, then activate explicitly");
    expect(html).toContain("explicit activation action");
    expect(html).toContain("Schema-5 save, autosave, calculation, and recovery remain unavailable");
    expect(html).toContain("Choose and open read-only…");
    expect(html).toContain("Open provided path read-only");
    expect(html).toContain("Close read-only session");
    expect(html).not.toContain(">Save project<");
  });

  it("shows the isolated active project identity and validated counts", () => {
    const html = view({
      archivePath: snapshot.archivePath,
      state: {
        phase: "active",
        session,
        failure: null,
        statusMessage: "Schema-6 archive opened in the isolated read-only Labs memory session.",
        dirty: true,
        persistence: "not_persisted",
        modelMutationPending: false,
        standardActivationPending: false,
        standardActivationFailure: null,
        standardActivationStatusMessage: "Ready to activate.",
        saveCopyPending: false,
        saveCopyFailure: null,
        saveCopyStatusMessage: "Detached model changes are not persisted.",
      },
    });

    expect(html).toContain('data-internal-schema6-session="active"');
    expect(html).toContain("Active only in the Labs memory session");
    expect(html).toContain("Customer study");
    expect(html).toContain("customer-study-v6.qpls");
    expect(html).toContain("Datasets</span><strong>2</strong>");
    expect(html).toContain("Models</span><strong>3</strong>");
    expect(html).toContain("Recipes</span><strong>4</strong>");
    expect(html).toContain("Saved results</span><strong>11</strong>");
    expect(html).toContain("Dataset values are not exposed to the frontend");
    expect(html).toContain("Activate ready/draft models in Standard");
    expect(html).toContain("Save validated new copy…");
  });

  it("announces strict-reader loading and actionable failures", () => {
    const loading = view({
      archivePath: "D:\\projects\\study.qpls",
      state: {
        phase: "opening",
        session: null,
        failure: null,
        statusMessage: "Opening…",
        dirty: false,
        persistence: null,
        modelMutationPending: false,
        standardActivationPending: false,
        standardActivationFailure: null,
        standardActivationStatusMessage: "Not activated.",
        saveCopyPending: false,
        saveCopyFailure: null,
        saveCopyStatusMessage: "No copy saved.",
      },
    });
    expect(loading).toContain('aria-busy="true"');
    expect(loading).toContain("Opening through the strict reader");
    expect(loading).toContain("Cancel and close session");

    const failed = view({
      state: {
        phase: "error",
        session: null,
        failure: {
          code: "schema6_archive_read.invalid_archive",
          message: "Archive validation failed.",
          correctiveAction: "Restore a trusted schema-6 ZIP.",
        },
        statusMessage: "No session opened.",
        dirty: false,
        persistence: null,
        modelMutationPending: false,
        standardActivationPending: false,
        standardActivationFailure: null,
        standardActivationStatusMessage: "Not activated.",
        saveCopyPending: false,
        saveCopyFailure: null,
        saveCopyStatusMessage: "No copy saved.",
      },
    });
    expect(failed).toContain('role="alert"');
    expect(failed).toContain("Archive validation failed.");
    expect(failed).toContain("Restore a trusted schema-6 ZIP.");
    expect(failed).toContain("schema6_archive_read.invalid_archive");
  });

  it("disables archive opening outside the native desktop", () => {
    const html = view({ nativeDesktop: false, archivePath: "D:\\study.qpls" });

    expect(html).toContain("Native desktop required");
    expect(html).toContain("No browser fallback reads or activates an archive");
    expect(html.match(/disabled=""/g)?.length).toBeGreaterThanOrEqual(4);
  });

  it("offers an accessible Standard close only after a clean validated copy", () => {
    const activated = {
      ...session,
      standardActivation: { modelIds: ["model:draft:1"], sourceArchiveSha256: "a".repeat(64) },
    };
    const blocked = view({
      archivePath: snapshot.archivePath,
      state: {
        phase: "active",
        session: activated,
        failure: null,
        statusMessage: "Bound.",
        dirty: true,
        persistence: "not_persisted",
        modelMutationPending: false,
        standardActivationPending: false,
        standardActivationFailure: null,
        standardActivationStatusMessage: "Activated.",
        saveCopyPending: false,
        saveCopyFailure: null,
        saveCopyStatusMessage: "Save required.",
      },
    });
    expect(blocked).toContain("Close General SEM project");
    expect(blocked).toContain("Save a validated new copy before closing");
    expect(blocked).toContain("Edit active model as new revision");
    expect(blocked).toContain("old RecipeV4 and canonical results remain bound");
    expect(blocked).toContain('role="status"');

    const closable = view({
      archivePath: snapshot.archivePath,
      state: {
        phase: "active",
        session: activated,
        failure: null,
        statusMessage: "Bound.",
        dirty: false,
        persistence: "persisted_new_copy",
        modelMutationPending: false,
        standardActivationPending: false,
        standardActivationFailure: null,
        standardActivationStatusMessage: "Activated.",
        saveCopyPending: false,
        saveCopyFailure: null,
        saveCopyStatusMessage: "Saved.",
      },
    });
    expect(closable).toContain("Close Standard and release its schema-6 source binding.");
    expect(closable).toContain("Reopen the saved copy to continue.");
  });
});
