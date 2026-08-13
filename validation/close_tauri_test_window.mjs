import { chromium } from "playwright";

const endpoint = process.env.QUICKPLS_CDP_ENDPOINT ?? "http://127.0.0.1:9222";
const browser = await chromium.connectOverCDP(endpoint);

try {
  const page = browser.contexts()[0]?.pages()[0];
  if (page) {
    await page.evaluate(async () => {
      await window.__TAURI_INTERNALS__.invoke("plugin:window|destroy", { label: "main" });
    });
  }
} catch {
  // Destroying the native window closes the CDP target before evaluate resolves.
} finally {
  await browser.close().catch(() => undefined);
}
