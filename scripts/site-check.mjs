import { chromium } from "playwright";

const origin = process.env.BETTERFY_SITE_URL ?? "http://127.0.0.1:4174/BetterFy/";
const browser = await chromium.launch();
const cases = [
  { language: "ru", width: 1440, height: 1000 },
  { language: "en", width: 1180, height: 820 },
  { language: "ru", width: 390, height: 844 },
  { language: "en", width: 390, height: 844 },
];

try {
  for (const testCase of cases) {
    const page = await browser.newPage({ viewport: { width: testCase.width, height: testCase.height } });
    const browserErrors = [];
    page.on("pageerror", (error) => browserErrors.push(error.message));
    page.on("console", (message) => {
      if (message.type() === "error" && !message.text().startsWith("Failed to load resource:")) browserErrors.push(message.text());
    });
    page.on("response", (response) => {
      if (response.status() === 404 && response.url().endsWith("/repos/zori-xyz/BetterFy/releases/latest")) return;
      if (response.status() >= 400) browserErrors.push(`${response.status()} ${response.url()}`);
    });
    await page.addInitScript((language) => localStorage.setItem("betterfy-site-language", language), testCase.language);
    await page.goto(origin, { waitUntil: "networkidle" });
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    if (overflow > 1) throw new Error(`${testCase.language} ${testCase.width}px overflows horizontally by ${overflow}px`);

    await page.getByRole("button", { name: testCase.language === "ru" ? "Войти" : "Sign in" }).click();
    const dialog = page.getByRole("dialog");
    if (!(await dialog.isVisible())) throw new Error(`${testCase.language} ${testCase.width}px account dialog did not open`);
    await page.keyboard.press("Escape");
    if (await dialog.isVisible()) throw new Error(`${testCase.language} ${testCase.width}px account dialog did not close`);
    if (browserErrors.length > 0) throw new Error(`${testCase.language} ${testCase.width}px browser errors: ${browserErrors.join(" | ")}`);
    await page.close();
  }
  console.log(`BetterFy website: ${cases.length} responsive and account-flow checks passed.`);
} finally {
  await browser.close();
}
