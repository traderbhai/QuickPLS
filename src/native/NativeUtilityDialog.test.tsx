import { beforeEach, describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { useWorkspace } from "../store";
import NativeUtilityDialog from "./NativeUtilityDialog";
import { completedGscaRun } from "./nativeGsca.testFixture";

describe("NativeUtilityDialog", () => {
  beforeEach(() => {
    const state = useWorkspace.getState();
    useWorkspace.setState({
      analysisSettings: {
        ...state.analysisSettings,
        method: "pls_pm",
        bootstrapSamples: 0,
        cbsemBootstrapSamples: 0,
        cbsemGroupColumn: null,
      },
      uiPreferences: { ...state.uiPreferences, experimentalLabsEnabled: false },
    });
  });

  it("mounts the live preview-first diagnostic workflow in production Preferences", () => {
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
    const html = renderToStaticMarkup(<NativeUtilityDialog kind="settings" close={() => undefined} />);
    vi.unstubAllGlobals();

    expect(html).toContain('data-live-preferences-dialog="true"');
    expect(html).toContain('data-diagnostic-bundle-panel="live"');
    expect(html).toContain("Interface density");
    expect(html).toContain("Diagnostics and support");
    expect(html).toContain("Preview bundle");
    expect(html).toContain("Save new ZIP");
    expect(html).toContain("Cancel preview");
    expect(html).toContain("never uploads or attaches it automatically");
    expect(html).not.toContain("Native desktop required");
    expect(html).not.toContain(">Upload<");
  });

  it("renders all nine Method Details sections for only the selected exact option cells", () => {
    vi.stubGlobal("window", {});
    const html = renderToStaticMarkup(<NativeUtilityDialog kind="trust" close={() => undefined} />);
    vi.unstubAllGlobals();

    expect(html).toContain('data-method-details-v2="true"');
    expect(html).toContain("PLS-SEM Algorithm");
    expect(html).toContain("What this method answers");
    expect(html).toContain("When to use it");
    expect(html).toContain("Required model and data");
    expect(html).toContain("Main settings and defaults");
    expect(html).toContain("Outputs");
    expect(html).toContain("Assumptions and limitations");
    expect(html).toContain('data-method-guidance-home="true"');
    expect(html).not.toContain("Available in Standard.");
    expect(html).toContain("Interpretation guidance");
    expect(html).toContain("Method references");
    expect(html).toContain("Advanced technical details");
    expect(html).not.toContain("Necessary Condition Analysis");
  });

  it("explains how to enable a Labs method without exposing internal product-governance labels", () => {
    const state = useWorkspace.getState();
    vi.stubGlobal("window", {});
    const html = renderToStaticMarkup(<NativeUtilityDialog
      kind="trust"
      close={() => undefined}
      methodDetailsSettings={{ ...state.analysisSettings, method: "nonlinear_effects" }}
      experimentalLabsEnabledOverride={false}
    />);
    vi.unstubAllGlobals();

    expect(html).toContain("Nonlinear Relationships");
    expect(html).toContain("Turn on Experimental Labs in Preferences to use this option.");
    expect(html).toContain(">Unavailable<");
    expect(html).not.toMatch(/native-qualified|release-qualified|promotion evidence|packaged evidence|candidate\s*[/,;]\s*unqualified/i);
  });

  it("binds Method Details opened from Results to the selected completed run", () => {
    vi.stubGlobal("window", {});
    const run = completedGscaRun();
    const html = renderToStaticMarkup(<NativeUtilityDialog kind="trust" close={() => undefined} run={run} />);
    vi.unstubAllGlobals();

    expect(html).toContain('data-method-details-context="completed-run"');
    expect(html).toContain("Generalized Structured Component Analysis");
    expect(html).not.toContain("PLS-SEM Algorithm");
    expect(html).toContain("Selected completed run");
    expect(html).toContain("GSCA run");
    expect(html).toContain(run.provenance?.dataset_fingerprint);
  });

});
