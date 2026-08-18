import { chromium } from "playwright";

const endpoint = process.env.QUICKPLS_CDP_ENDPOINT ?? "http://127.0.0.1:9222";
const browser = await chromium.connectOverCDP(endpoint);

try {
  const page = browser.contexts()[0]?.pages()[0];
  if (!page) throw new Error("QuickPLS close helper found no native page.");
  try {
    await page.evaluate(async () => {
      await window.__TAURI_INTERNALS__.invoke("exit_desktop_application");
    });
  } catch (error) {
    const cdpStillOpen = await fetch(`${endpoint}/json/version`).then((response) => response.ok).catch(() => false);
    if (cdpStillOpen) throw error;
  }
  let cdpClosed = false;
  for (let attempt = 0; attempt < 20; attempt += 1) {
    cdpClosed = !await fetch(`${endpoint}/json/version`).then((response) => response.ok).catch(() => false);
    if (cdpClosed) break;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  if (!cdpClosed) throw new Error("QuickPLS exit request left the native CDP endpoint open.");
} finally {
  await browser.close().catch(() => undefined);
}
