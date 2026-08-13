import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { DiagnosticStagedContents } from "../services/projectService";
import { useWorkspace } from "../store";
import { DiagnosticStagedContentsPreview, SettingsWorkspace } from "./SettingsWorkspace";

describe("SettingsWorkspace diagnostic bundle controls", () => {
  beforeEach(() => {
    useWorkspace.getState().resetProject();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("renders a native-only preview-first, local-only support workflow", () => {
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });

    const html = renderToStaticMarkup(<SettingsWorkspace />);

    expect(html).toContain("Diagnostics and support");
    expect(html).toContain('data-diagnostic-bundle-panel="live"');
    expect(html).toContain("Preview bundle");
    expect(html).toContain("Save new ZIP");
    expect(html).toContain("Cancel preview");
    expect(html).toContain("never uploads or attaches it automatically");
    expect(html).toContain('role="status"');
    expect(html).toContain('aria-live="polite"');
    expect(html).not.toContain(">Upload<");
  });

  it("explains that diagnostic creation is unavailable outside the native desktop", () => {
    vi.stubGlobal("window", {});

    const html = renderToStaticMarkup(<SettingsWorkspace />);

    expect(html).toContain("Native desktop required");
    expect(html).toContain("installed QuickPLS desktop application");
    expect(html).toContain('disabled=""');
  });

  it("renders the inspectable redacted system, event, and manifest staging payload accessibly", () => {
    const contents: DiagnosticStagedContents = {
      system: {
        schemaVersion: 1,
        quickplsVersion: "2.46.0",
        releaseChannel: "unsigned-preview",
        sourceRevision: "abcdef1",
        osFamily: "windows",
        architecture: "x86_64",
        desktopRuntime: "Tauri 2",
        locale: "not_collected",
        webview2Version: "not_collected",
        userDataIncluded: false,
        networkAccessed: false,
      },
      events: [{ timestamp: "2026-08-13T09:00:00.000Z", sequence: 2, severity: "info", code: "diagnostic.preview.requested" }],
      manifest: {
        schemaVersion: 1,
        policyVersion: "quickpls-diagnostics-v1",
        createdAt: "2026-08-13T09:00:00.000Z",
        quickplsVersion: "2.46.0",
        entries: [{ name: "metadata/system.json", sha256: "a".repeat(64), bytes: 420 }],
        redactionCounts: { windowsPaths: 1, emailAddresses: 0, urlQueriesOrFragments: 0, bearerTokens: 0 },
        redactionTotal: 1,
        archiveLimits: {
          maximumEntries: 3,
          maximumEntryBytes: 262144,
          maximumUncompressedBytes: 524288,
          maximumArchiveBytes: 532480,
          compression: "stored",
        },
        localOnly: true,
        networkAccessed: false,
      },
    };

    const html = renderToStaticMarkup(<DiagnosticStagedContentsPreview contents={contents} />);

    expect(html).toContain('aria-label="Redacted staged diagnostic contents"');
    expect(html).toContain('aria-label="Redacted diagnostic event rows"');
    expect(html).toContain('aria-label="Diagnostic manifest payload descriptors"');
    expect(html).toContain("diagnostic.preview.requested");
    expect(html).toContain("metadata/system.json");
    expect(html).toContain("unsigned-preview");
    expect(html).toContain("User data included</dt><dd>no");
    expect(html).toContain("Network accessed</dt><dd>no");
  });
});
