import { chromium } from "playwright";
import { existsSync, mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";

const base = "http://127.0.0.1:1420";
const output = fileURLToPath(new URL("../website/public/product/", import.meta.url));
const browserCandidates = [
  process.env.BETTERFY_CHROMIUM_PATH,
  chromium.executablePath(),
  "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
  "/Applications/Chromium.app/Contents/MacOS/Chromium",
].filter(Boolean);
const executablePath = browserCandidates.find((candidate) => existsSync(candidate));

if (!executablePath) throw new Error("capture_browser_missing");
mkdirSync(output, { recursive: true });

const browser = await chromium.launch({ headless: true, executablePath });
const page = await browser.newPage({ viewport: { width: 1600, height: 1000 }, deviceScaleFactor: 1 });

try {
  await page.goto(base, { waitUntil: "networkidle" });
  await page.evaluate(() => localStorage.clear());
  await page.reload({ waitUntil: "networkidle" });
  await page.getByRole("link", { name: /Открыть BetterFy Bot|Open BetterFy Bot/ }).waitFor();
  await page.getByRole("button", { name: "EN" }).click();
  await page.getByRole("button", { name: /I already have a code/ }).click();
  await page.locator(".otp-field input").fill("123456");
  await page.getByRole("button", { name: /Confirm/ }).click();
  await page.locator(".confirmed-view").waitFor();
  await page.getByRole("button", { name: /Find automatically/ }).waitFor();
  await page.getByRole("button", { name: /Find automatically/ }).click();
  await page.getByRole("button", { name: /Continue to Home/ }).click();
  await page.getByRole("button", { name: /Open build/ }).waitFor();
  await page.waitForTimeout(700);
  await page.screenshot({ path: `${output}/betterfy-home.png` });

  await page.getByRole("button", { name: /^Discover$/ }).click();
  await page.locator(".minify-heading").waitFor();
  await page.waitForTimeout(700);
  await page.screenshot({ path: `${output}/betterfy-discover.png` });

  await page.locator(".app-rail").hover();
  await page.getByRole("button", { name: /^Build$/ }).click();
  await page.waitForTimeout(700);
  await page.screenshot({ path: `${output}/betterfy-build.png` });
} finally {
  await browser.close();
}

console.log(`Captured BetterFy product screens in ${output}`);
