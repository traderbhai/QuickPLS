import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import NativeUtilityDialog from "./NativeUtilityDialog";
import { NATIVE_NCA_SCOPE_NOTE } from "./nativeNca";

describe("NativeUtilityDialog", () => {
  it("discloses the bounded standalone NCA scope in the on-demand trust surface", () => {
    vi.stubGlobal("window", {});
    const html = renderToStaticMarkup(<NativeUtilityDialog kind="trust" close={() => undefined} />);
    vi.unstubAllGlobals();

    expect(html).toContain("Necessary Condition Analysis");
    expect(html).toContain(NATIVE_NCA_SCOPE_NOTE);
    expect(html).toContain("Numeric observed-variable");
    expect(html).not.toContain("latent-score NCA is supported");
  });

  it("describes the current joint MICOM and permutation-MGA v2 scope", () => {
    vi.stubGlobal("window", {});
    const html = renderToStaticMarkup(<NativeUtilityDialog kind="trust" close={() => undefined} />);
    vi.unstubAllGlobals();

    expect(html).toContain("MICOM and Two-Group Permutation MGA");
    expect(html).toContain("5,000–10,000 usable permutations");
    expect(html).toContain("MICOM Steps 1–3");
    expect(html).not.toContain("measurement invariance is not assessed");
  });

  it("labels Structural Path Randomization as candidate fixed-score inference, never validated", () => {
    vi.stubGlobal("window", {});
    const html = renderToStaticMarkup(<NativeUtilityDialog kind="trust" close={() => undefined} />);
    vi.unstubAllGlobals();

    expect(html).toContain("Structural Path Randomization");
    expect(html).toContain("Candidate single-model Freedman-Lane fixed-score inference");
    expect(html).toContain("exchangeable reduced-model residuals");
    expect(html).toContain("not a group comparison");
    expect(html).not.toContain("Structural Path Randomization</dt><dd>Validated");
  });

  it("describes indicator PLSpredict / CVPAT without a saved-model comparison claim", () => {
    vi.stubGlobal("window", {});
    const html = renderToStaticMarkup(<NativeUtilityDialog kind="trust" close={() => undefined} />);
    vi.unstubAllGlobals();

    expect(html).toContain("PLSpredict / CVPAT");
    expect(html).toContain("indicator-level scope with seeded 10-fold × 10-repeat cross-validation");
    expect(html).toContain("one-sided 95% CVPAT benchmark assessment");
    expect(html).not.toContain("saved-model comparison");
  });
});
