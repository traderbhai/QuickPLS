import path from "node:path";
import { chromium } from "playwright";
import { RESULTS, startPreview, stopPreview, waitForPreview, writeJson } from "./lib/v2_ui_smoke_harness.mjs";

const PORT = 57942;
const OUT = path.join(RESULTS, "v2422_native_command_wiring_audit.json");

const WORKSPACES = [
  "Home",
  "Data",
  "Model",
  "Setup",
  "Run",
  "Results",
  "Report",
  "Trust Center",
  "Settings",
];

function normalize(text) {
  return String(text ?? "").replace(/\s+/g, " ").trim();
}

async function collectButtons(page, selector, context) {
  return page.locator(selector).evaluateAll((nodes, ctx) => nodes.map((node) => ({
    context: ctx,
    label: node.textContent?.replace(/\s+/g, " ").trim() ?? "",
    disabled: node instanceof HTMLButtonElement ? node.disabled : false,
    commandStatus: node.getAttribute("data-command-status") ?? "",
    disabledReason: node.getAttribute("data-disabled-reason") ?? "",
    title: node.getAttribute("title") ?? "",
    ariaLabel: node.getAttribute("aria-label") ?? "",
  })), context);
}

function evaluateButton(button) {
  if (!button.label) {
    return { passed: false, issue: "command_label_missing" };
  }
  if (!button.commandStatus) {
    return { passed: false, issue: "command_status_missing" };
  }
  if (!["wired", "disabled"].includes(button.commandStatus)) {
    return { passed: false, issue: "command_status_invalid" };
  }
  if (button.commandStatus === "wired" && button.disabled) {
    return { passed: false, issue: "wired_command_disabled" };
  }
  if (button.commandStatus === "disabled" && !button.disabled) {
    return { passed: false, issue: "disabled_command_clickable" };
  }
  if (button.commandStatus === "disabled" && !normalize(button.disabledReason || button.title || button.ariaLabel)) {
    return { passed: false, issue: "disabled_reason_missing" };
  }
  return { passed: true, issue: "" };
}

async function selectWorkspace(page, workspace) {
  const button = page.locator(".np-rail button", { hasText: workspace }).first();
  await button.click();
  await page.waitForTimeout(120);
}

const url = `http://127.0.0.1:${PORT}/`;
const { server, logs } = startPreview(PORT);
let browser;

try {
  await waitForPreview(url, logs);
  browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });
  await page.goto(url, { waitUntil: "domcontentloaded", timeout: 45_000 });
  await page.waitForSelector(".np-shell[data-v241-mockup-parity='true']", { timeout: 15_000 });
  await page.waitForTimeout(300);

    const commandBarButtons = [];
    const menuButtons = [];

    for (const workspace of WORKSPACES) {
      await selectWorkspace(page, workspace);
      commandBarButtons.push(...await collectButtons(page, ".np-commandbar button", `${workspace} command bar`));
    }

    const menuLabels = await page.locator(".np-menu-slot > button").evaluateAll((nodes) => nodes.map((node) => node.textContent?.trim()).filter(Boolean));
    for (const menuLabel of menuLabels) {
      await page.locator(".np-menu-slot > button", { hasText: menuLabel }).first().click();
      await page.waitForTimeout(80);
      menuButtons.push(...await collectButtons(page, ".np-menu-popover button", `${menuLabel} menu`));
      await page.keyboard.press("Escape");
      await page.waitForTimeout(50);
    }

    const buttons = [...commandBarButtons, ...menuButtons];
    const issues = buttons
      .map((button) => ({ button, ...evaluateButton(button) }))
      .filter((item) => !item.passed)
      .map(({ button, issue }) => ({
        severity: "high",
        issue,
        context: button.context,
        label: button.label,
        commandStatus: button.commandStatus,
        disabled: button.disabled,
        disabledReason: button.disabledReason,
        title: button.title,
      }));

    const payload = {
      passed: issues.length === 0 && errors.length === 0,
      checked_at: new Date().toISOString(),
      viewport: "1440x900",
      command_bar_button_count: commandBarButtons.length,
      menu_button_count: menuButtons.length,
      errors,
      issues,
      sample_disabled_commands: buttons
        .filter((button) => button.commandStatus === "disabled")
        .slice(0, 20)
        .map((button) => ({
          context: button.context,
          label: button.label,
          reason: normalize(button.disabledReason || button.title || button.ariaLabel),
        })),
      sample_wired_commands: buttons
        .filter((button) => button.commandStatus === "wired")
        .slice(0, 20)
        .map((button) => ({ context: button.context, label: button.label })),
    };

    await writeJson(OUT, payload);
    if (!payload.passed) {
      throw new Error(`Native command wiring audit failed with ${issues.length} issue(s). See ${OUT}`);
    }
} finally {
  if (browser) await browser.close();
  stopPreview(server, PORT);
}
