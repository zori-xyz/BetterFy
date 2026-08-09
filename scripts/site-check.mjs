import { chromium } from "playwright";

const origin = process.env.BETTERFY_SITE_URL ?? "http://127.0.0.1:4174/BetterFy/";
const browser = await chromium.launch();
const cases = [
  { language: "ru", width: 1440, height: 1000 },
  { language: "en", width: 1180, height: 820 },
  { language: "ru", width: 390, height: 844 },
  { language: "en", width: 390, height: 844 },
];
const forbiddenCopy = [
  /без технического шума/i,
  /без тумана/i,
  /спокойное пространство/i,
  /technical noise/i,
  /no fog/i,
  /calm workspace/i,
];

async function assertStorageDeniedFallback() {
  const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
  await page.addInitScript(() => {
    Object.defineProperty(window, "localStorage", {
      configurable: true,
      get() { throw new DOMException("Storage denied", "SecurityError"); },
    });
  });
  await page.goto(origin, { waitUntil: "networkidle" });
  if (!(await page.locator("h1").first().isVisible())) {
    throw new Error("Site did not render when localStorage was denied");
  }
  await page.close();
}

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
    const visibleCopy = await page.locator("body").innerText();
    const forbiddenMatch = forbiddenCopy.find((pattern) => pattern.test(visibleCopy));
    if (forbiddenMatch) throw new Error(`${testCase.language} contains forbidden generic copy: ${forbiddenMatch}`);
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    if (overflow > 1) throw new Error(`${testCase.language} ${testCase.width}px overflows horizontally by ${overflow}px`);

    const footerWordmark = page.locator(".site-footer .wordmark");
    await footerWordmark.scrollIntoViewIfNeeded();
    const wordmarkGeometry = await footerWordmark.evaluate((node) => {
      const better = node.querySelector(".wordmark-better")?.getBoundingClientRect();
      const fy = node.querySelector(".wordmark-fy")?.getBoundingClientRect();
      return better && fy ? { gap: fy.left - better.right, width: node.getBoundingClientRect().width } : null;
    });
    if (!wordmarkGeometry || wordmarkGeometry.gap > 12 || wordmarkGeometry.width > 180) {
      throw new Error(`${testCase.language} ${testCase.width}px footer wordmark geometry is invalid: ${JSON.stringify(wordmarkGeometry)}`);
    }
    const botHref = await page.getByRole("link", { name: "@BeterFyBot", exact: true }).getAttribute("href");
    if (!botHref?.startsWith("https://t.me/BeterFyBot")) throw new Error(`Unexpected Telegram link: ${botHref}`);

    await page.getByRole("button", { name: testCase.language === "ru" ? "Войти" : "Sign in" }).click();
    const dialog = page.getByRole("dialog");
    if (!(await dialog.isVisible())) throw new Error(`${testCase.language} ${testCase.width}px account dialog did not open`);
    await page.keyboard.press("Escape");
    if (await dialog.isVisible()) throw new Error(`${testCase.language} ${testCase.width}px account dialog did not close`);
    if (browserErrors.length > 0) throw new Error(`${testCase.language} ${testCase.width}px browser errors: ${browserErrors.join(" | ")}`);
    await page.close();
  }
  await assertStorageDeniedFallback();
  console.log(`BetterFy website: ${cases.length} responsive checks and the storage-denied fallback passed.`);
} finally {
  await browser.close();
}
