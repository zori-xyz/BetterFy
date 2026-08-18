import { chromium } from "playwright";

const origin = process.env.BETTERFY_SITE_URL ?? "http://127.0.0.1:4174/BetterFy/";
const browser = await chromium.launch();
const cases = [
  { language: "ru", width: 1440, height: 1000 },
  { language: "en", width: 1180, height: 820 },
  { language: "ru", width: 390, height: 844 },
  { language: "en", width: 390, height: 844 },
  { language: "ru", width: 320, height: 568 },
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

async function assertReducedMotionAndObserverFallback() {
  const reducedPage = await browser.newPage({ viewport: { width: 390, height: 844 }, reducedMotion: "reduce" });
  await reducedPage.goto(origin, { waitUntil: "networkidle" });
  const hiddenRevealCount = await reducedPage.locator(".reveal").evaluateAll((nodes) => nodes.filter((node) => {
    const style = getComputedStyle(node);
    return style.opacity === "0" || style.visibility === "hidden";
  }).length);
  if (hiddenRevealCount > 0) throw new Error(`Reduced motion left ${hiddenRevealCount} reveal surfaces hidden`);
  const transitionAnimation = await reducedPage.locator(".catalog-transition img").evaluate((node) => getComputedStyle(node).animationName);
  if (transitionAnimation !== "none") throw new Error(`Reduced motion left catalog animation enabled: ${transitionAnimation}`);
  await reducedPage.close();

  const fallbackPage = await browser.newPage({ viewport: { width: 390, height: 844 } });
  await fallbackPage.addInitScript(() => { Object.defineProperty(window, "IntersectionObserver", { configurable: true, value: undefined }); });
  await fallbackPage.goto(origin, { waitUntil: "networkidle" });
  if (await fallbackPage.locator(".reveal:not(.is-visible)").count()) throw new Error("IntersectionObserver fallback left reveal surfaces hidden");
  await fallbackPage.close();
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
    const brokenImages = await page.locator("img").evaluateAll((images) => images
      .filter((image) => image.loading !== "lazy" && (!image.complete || image.naturalWidth === 0))
      .map((image) => image.getAttribute("src")));
    if (brokenImages.length) throw new Error(`${testCase.language} ${testCase.width}px has broken images: ${brokenImages.join(", ")}`);
    const transition = page.locator(".catalog-transition");
    if (await transition.count() !== 1) throw new Error(`${testCase.language} ${testCase.width}px catalog transition is missing`);
    const transitionGeometry = await transition.evaluate((node) => ({ width: node.getBoundingClientRect().width, height: node.getBoundingClientRect().height }));
    if (transitionGeometry.width < 280 || transitionGeometry.height < 220) throw new Error(`${testCase.language} ${testCase.width}px transition is too small: ${JSON.stringify(transitionGeometry)}`);
    const eagerBelowFoldImages = await page.locator('.product img:not([loading="lazy"]):not([fetchpriority="high"])').count();
    if (eagerBelowFoldImages) throw new Error(`${testCase.language} ${testCase.width}px has ${eagerBelowFoldImages} eager product images`);
    const visibleCopy = await page.locator("body").innerText();
    const forbiddenMatch = forbiddenCopy.find((pattern) => pattern.test(visibleCopy));
    if (forbiddenMatch) throw new Error(`${testCase.language} contains forbidden generic copy: ${forbiddenMatch}`);
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    if (overflow > 1) throw new Error(`${testCase.language} ${testCase.width}px overflows horizontally by ${overflow}px`);

    const footerWordmark = page.locator(".site-footer .wordmark");
    await footerWordmark.scrollIntoViewIfNeeded();
    for (const image of await page.locator("img").all()) {
      await image.scrollIntoViewIfNeeded();
      await image.evaluate((node) => node.decode?.().catch(() => {}));
    }
    const unloadedImages = await page.locator("img").evaluateAll((images) => images
      .filter((image) => !image.complete || image.naturalWidth === 0)
      .map((image) => image.getAttribute("src")));
    if (unloadedImages.length) throw new Error(`${testCase.language} ${testCase.width}px has unloaded images: ${unloadedImages.join(", ")}`);
    const wordmarkGeometry = await footerWordmark.evaluate((node) => {
      const better = node.querySelector(".wordmark-better")?.getBoundingClientRect();
      const fy = node.querySelector(".wordmark-fy")?.getBoundingClientRect();
      return better && fy ? { gap: fy.left - better.right, width: node.getBoundingClientRect().width } : null;
    });
    if (!wordmarkGeometry || wordmarkGeometry.gap > 12 || wordmarkGeometry.width > 180) {
      throw new Error(`${testCase.language} ${testCase.width}px footer wordmark geometry is invalid: ${JSON.stringify(wordmarkGeometry)}`);
    }
    const botHref = await page.locator('.site-footer a[href^="https://t.me/BeterFyBot"]').getAttribute("href");
    if (!botHref?.startsWith("https://t.me/BeterFyBot")) throw new Error(`Unexpected Telegram link: ${botHref}`);

    const journeyTabs = page.getByRole("tab");
    await journeyTabs.first().focus();
    await page.keyboard.press("ArrowRight");
    if (await journeyTabs.nth(1).getAttribute("aria-selected") !== "true") throw new Error(`${testCase.language} ${testCase.width}px journey tabs do not support arrow keys`);
    if (await journeyTabs.first().getAttribute("data-complete") !== "true") throw new Error(`${testCase.language} ${testCase.width}px journey completion state is missing`);

    const profileTrigger = page.getByRole("button", { name: testCase.language === "ru" ? "Профиль" : "Profile" }).first();
    await profileTrigger.click();
    const dialog = page.getByRole("dialog");
    if (!(await dialog.isVisible())) throw new Error(`${testCase.language} ${testCase.width}px account dialog did not open`);
    if (!(await page.locator("main").evaluate((node) => node.inert))) throw new Error(`${testCase.language} ${testCase.width}px dialog background is not inert`);
    await dialog.locator(".modal-close:focus").waitFor();
    await page.keyboard.press("Shift+Tab");
    if (!(await dialog.locator(":focus").count())) throw new Error(`${testCase.language} ${testCase.width}px focus escaped the dialog`);
    await dialog.getByRole("button", { name: testCase.language === "ru" ? "У меня уже есть код" : "I already have a code" }).click();
    if (!(await dialog.getByLabel(testCase.language === "ru" ? "Код подтверждения" : "Confirmation code").isVisible())) {
      throw new Error(`${testCase.language} ${testCase.width}px one-time code entry did not open`);
    }
    await page.keyboard.press("Escape");
    if (await dialog.isVisible()) throw new Error(`${testCase.language} ${testCase.width}px account dialog did not close`);
    await profileTrigger.locator(":scope:focus").waitFor();
    if (browserErrors.length > 0) throw new Error(`${testCase.language} ${testCase.width}px browser errors: ${browserErrors.join(" | ")}`);
    await page.close();
  }
  await assertStorageDeniedFallback();
  await assertReducedMotionAndObserverFallback();
  console.log(`BetterFy website: ${cases.length} responsive checks, modal focus, journey keyboard controls, reduced motion, and fallbacks passed.`);
} finally {
  await browser.close();
}
