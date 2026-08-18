import { renderToStaticMarkup } from "react-dom/server";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useWorkspace } from "../store";
import { TrustCenterWorkspace } from "./TrustCenterWorkspace";

describe("TrustCenterWorkspace method truth", () => {
  beforeEach(() => {
    useWorkspace.getState().resetProject();
    useWorkspace.getState().setAnalysisSettings({ method: "permutation", permutationSamples: 999 });
    vi.stubGlobal("window", {});
  });

  it("uses the scoped Standard Registry V2 contract", () => {
    const html = renderToStaticMarkup(<TrustCenterWorkspace />);
    vi.unstubAllGlobals();

    expect(html).toContain("Freedman-Lane permutation");
    expect(html).toMatch(/Freedman-Lane permutation<\/td><td>PLS-SEM<\/td><td>Supported/);
    expect(html).toContain('<span class="status-text validated ui-status-badge">Supported setup</span>');
    expect(html).toContain("Method guidance");
    expect(html).toContain("docs/methods/PERMUTATION_ENGINE_V1.md");
    expect(html).not.toMatch(/Freedman-Lane permutation<\/td><td>PLS-SEM<\/td><td>(Experimental|Limited scope)/i);
    expect(html).not.toMatch(/validation evidence|promotion evidence/i);
  });
});
