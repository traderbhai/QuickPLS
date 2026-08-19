import fs from "node:fs/promises";
import { createHash } from "node:crypto";
import path from "node:path";

export const PACKAGED_TAURI_ORIGIN = "http://tauri.localhost";
export const PACKAGED_TAURI_IPC_ORIGIN = "http://ipc.localhost";

export async function inspectQuickPlsCdpPage(candidate, index) {
  const fallbackUrl = candidate.url();
  try {
    const inspected = await candidate.evaluate(() => {
      const shell = document.querySelector(".nd-app[data-native-desktop-shell='true']");
      const shellStyle = shell ? getComputedStyle(shell) : null;
      return {
        title: document.title,
        shellVisible: Boolean(shell
          && shellStyle?.display !== "none"
          && shellStyle?.visibility !== "hidden"
          && shell.getClientRects().length > 0),
        tauriRuntime: Boolean(window.__TAURI_INTERNALS__),
      };
    });
    const url = candidate.url();
    let origin = null;
    try { origin = new URL(url).origin; } catch { origin = null; }
    return { index, url, origin, ...inspected };
  } catch {
    let origin = null;
    try { origin = new URL(fallbackUrl).origin; } catch { origin = null; }
    return { index, url: fallbackUrl, origin, title: "", shellVisible: false, tauriRuntime: false };
  }
}

export async function enumerateQuickPlsCdpPages(browserInstance) {
  const pages = browserInstance.contexts().flatMap((context) => context.pages());
  const inspected = await Promise.all(
    pages.map((candidate, index) => inspectQuickPlsCdpPage(candidate, index)),
  );
  return pages.map((candidate, index) => ({ candidate, state: inspected[index] }));
}

export async function connectToSingleQuickPlsPage({
  chromium,
  endpoint,
  expectedOrigin = PACKAGED_TAURI_ORIGIN,
  attempts = 60,
  intervalMilliseconds = 250,
}) {
  const browser = await chromium.connectOverCDP(endpoint);
  try {
    let entries = [];
    for (let attempt = 0; attempt < attempts; attempt += 1) {
      entries = await enumerateQuickPlsCdpPages(browser);
      if (entries.some(({ state }) => state.shellVisible && state.tauriRuntime)) break;
      await new Promise((resolve) => setTimeout(resolve, intervalMilliseconds));
    }
    const qualifying = entries.filter(({ state }) => state.shellVisible && state.tauriRuntime);
    if (qualifying.length !== 1) {
      throw new Error(`Expected exactly one QuickPLS shell+Tauri CDP page; found ${qualifying.length}: ${JSON.stringify(entries.map(({ state }) => state))}`);
    }
    if (qualifying[0].state.origin !== expectedOrigin) {
      throw new Error(`QuickPLS packaged preflight expected ${expectedOrigin}; received ${qualifying[0].state.origin ?? "invalid"}.`);
    }
    return { browser, page: qualifying[0].candidate, pageState: qualifying[0].state, enumeratedPages: entries.map(({ state }) => state) };
  } catch (error) {
    await browser.close().catch(() => undefined);
    throw error;
  }
}

export async function setActualTauriClientViewport(page, viewport, reason) {
  const playwrightViewport = page.viewportSize();
  if (playwrightViewport !== null) {
    throw new Error(`${reason} cannot resize the actual Tauri window while Playwright viewport emulation is active: ${JSON.stringify(playwrightViewport)}`);
  }
  const cdp = await page.context().newCDPSession(page);
  try {
    const target = await cdp.send("Target.getTargetInfo");
    const targetId = target?.targetInfo?.targetId ?? null;
    if (!targetId) throw new Error(`${reason} could not resolve the actual WebView2 target identity.`);
    const window = await cdp.send("Browser.getWindowForTarget", { targetId });
    if (!Number.isInteger(window?.windowId) || !window?.bounds) {
      throw new Error(`${reason} could not bind the WebView2 target to an actual desktop window.`);
    }
    await cdp.send("Emulation.clearDeviceMetricsOverride");
    if (window.bounds.windowState !== "normal") {
      await cdp.send("Browser.setWindowBounds", { windowId: window.windowId, bounds: { windowState: "normal" } });
      await page.waitForTimeout(250);
    }
    const anchor = (await cdp.send("Browser.getWindowBounds", { windowId: window.windowId })).bounds;
    let requestedLeft = Number.isInteger(anchor.left) ? anchor.left : null;
    let requestedTop = Number.isInteger(anchor.top) ? anchor.top : null;
    const attempts = [];
    for (let attempt = 1; attempt <= 8; attempt += 1) {
      const dom = await page.evaluate(() => ({ innerWidth, innerHeight }));
      const current = (await cdp.send("Browser.getWindowBounds", { windowId: window.windowId })).bounds;
      if (dom.innerWidth === viewport.width && dom.innerHeight === viewport.height) {
        return { targetId, windowId: window.windowId, anchor, outerBounds: current, domInnerDimensions: dom, attempts };
      }
      const requestedOuterBounds = {
        width: Math.max(300, current.width + viewport.width - dom.innerWidth),
        height: Math.max(300, current.height + viewport.height - dom.innerHeight),
      };
      if (requestedLeft !== null) requestedOuterBounds.left = requestedLeft;
      if (requestedTop !== null) requestedOuterBounds.top = requestedTop;
      await cdp.send("Browser.setWindowBounds", { windowId: window.windowId, bounds: requestedOuterBounds });
      await page.waitForFunction(
        ([width, height]) => innerWidth === width && innerHeight === height,
        [viewport.width, viewport.height],
        { timeout: 1_500 },
      ).catch(() => undefined);
      await page.waitForTimeout(150);
      const observedDom = await page.evaluate(() => ({ innerWidth, innerHeight }));
      const observedOuter = (await cdp.send("Browser.getWindowBounds", { windowId: window.windowId })).bounds;
      attempts.push({ attempt, requestedOuterBounds, observedOuterBounds: observedOuter, observedDomInnerDimensions: observedDom });
      if (requestedLeft !== null && Number.isInteger(observedOuter.left)) requestedLeft += anchor.left - observedOuter.left;
      if (requestedTop !== null && Number.isInteger(observedOuter.top)) requestedTop += anchor.top - observedOuter.top;
      if (observedDom.innerWidth === viewport.width && observedDom.innerHeight === viewport.height) {
        return { targetId, windowId: window.windowId, anchor, outerBounds: observedOuter, domInnerDimensions: observedDom, attempts };
      }
    }
    throw new Error(`${reason} could not reach the exact ${viewport.width}x${viewport.height} actual client viewport: ${JSON.stringify(attempts)}`);
  } finally {
    await cdp.detach().catch(() => undefined);
  }
}

export function observeFunctionalOfflineRequests(page) {
  const requests = [];
  const listener = (request) => {
    const url = request.url();
    let origin = null;
    try { origin = new URL(url).origin; } catch { origin = null; }
    requests.push({ method: request.method(), resourceType: request.resourceType(), url, origin });
  };
  page.on("request", listener);
  return {
    requests,
    stop() { page.off("request", listener); },
    summary() {
      const internal = new Set([PACKAGED_TAURI_ORIGIN, PACKAGED_TAURI_IPC_ORIGIN, "null"]);
      const external = requests.filter(({ origin }) => origin && !internal.has(origin));
      return {
        passed: requests.length > 0 && external.length === 0,
        observedRequestCount: requests.length,
        externalRequestCount: external.length,
        origins: [...new Set(requests.map(({ origin }) => origin))].sort(),
        externalRequests: external,
      };
    },
  };
}

export async function fileArtifact(repositoryRoot, filePath, kind) {
  const absolute = path.resolve(filePath);
  const relative = path.relative(path.resolve(repositoryRoot), absolute);
  if (!relative || relative === ".." || relative.startsWith(`..${path.sep}`) || path.isAbsolute(relative)) {
    throw new Error(`Evidence artifact must remain inside the repository: ${absolute}`);
  }
  const bytes = await fs.readFile(absolute);
  return {
    kind,
    path: relative.split(path.sep).join("/"),
    size: bytes.length,
    sha256: createHash("sha256").update(bytes).digest("hex"),
  };
}
