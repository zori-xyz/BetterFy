import { chromium } from "playwright";

const origin = process.env.BETTERFY_SITE_URL ?? "http://127.0.0.1:4174/BetterFy/";
const browser = await chromium.launch();
const cases = [
  { language: "ru", width: 1440, height: 1000 },
  { language: "en", width: 1180, height: 820 },
  { language: "ru", width: 390, height: 844 },
  { language: "en", width: 390, height: 844 },
  { language: "ru", width: 768, height: 1024 },
  { language: "en", width: 1920, height: 1080 },
  { language: "ru", width: 2560, height: 1080 },
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
    Object.defineProperty(window, "sessionStorage", {
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

async function assertSuccessfulAuthWithoutSessionStorage() {
  const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
  await page.addInitScript(() => {
    const original = window.sessionStorage;
    Object.defineProperty(window, "sessionStorage", {
      configurable: true,
      get() {
        return new Proxy(original, {
          get(target, property) {
            if (property === "setItem") return () => { throw new DOMException("Storage denied", "SecurityError"); };
            const value = Reflect.get(target, property, target);
            return typeof value === "function" ? value.bind(target) : value;
          },
        });
      },
    });
  });
  await page.route("**/v1/auth/telegram/code", (route) => route.fulfill({
    status: 200,
    contentType: "application/json",
    body: JSON.stringify({ userId: "test-user", displayName: "Test Player", accessTier: "early-access", avatarAvailable: false, sessionToken: "test-session" }),
  }));
  await page.route("**/v1/session/profile", (route) => route.fulfill({
    status: 200,
    contentType: "application/json",
    body: JSON.stringify({ userId: "test-user", displayName: "Test Player", accessTier: "early-access", avatarAvailable: false }),
  }));
  await page.goto(origin, { waitUntil: "networkidle" });
  await page.getByRole("button", { name: "Профиль" }).first().click();
  await page.getByRole("button", { name: "У меня уже есть код" }).click();
  await page.getByLabel("Код подтверждения").fill("123456");
  await page.getByRole("button", { name: "Войти", exact: true }).click();
  await page.getByRole("heading", { name: "Test Player" }).waitFor();
  await page.getByText(/только для этой вкладки/i).waitFor();
  await page.close();
}

async function assertIdSignInFlow() {
  const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
  await page.route("**/v1/auth/id/login", (route) => route.fulfill({
    status: 200, contentType: "application/json",
    body: JSON.stringify({ userId: "test-user", displayName: "Test Player", accessTier: "early-access", avatarAvailable: false, sessionToken: "test-id-session" }),
  }));
  await page.route("**/v1/session/profile", (route) => route.fulfill({
    status: 200, contentType: "application/json",
    body: JSON.stringify({ userId: "test-user", displayName: "Test Player", accessTier: "early-access", avatarAvailable: false }),
  }));
  await page.goto(origin, { waitUntil: "networkidle" });
  await page.getByRole("button", { name: "Профиль" }).first().click();
  await page.getByRole("button", { name: "Войти через BetterFy ID" }).click();
  await page.getByRole("textbox", { name: "Почта или никнейм" }).fill("player_07");
  await page.getByLabel("Пароль — от 12 символов").fill("test password 2026");
  await page.getByRole("button", { name: "Войти", exact: true }).click();
  await page.getByRole("heading", { name: "Test Player" }).waitFor();
  await page.close();
}

async function assertReleaseStates() {
  for (const state of ["missing", "unavailable"]) {
    const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
    await page.route(/^https:\/\/api\.github\.com\/repos\/zori-xyz\/BetterFy\/releases\?per_page=20$/, (route) => state === "missing"
      ? route.fulfill({ status: 200, contentType: "application/json", body: "[]" })
      : route.fulfill({ status: 503, contentType: "application/json", body: "{}" }));
    await page.goto(origin, { waitUntil: "networkidle" });
    await page.getByRole("button", { name: "Скачать для Windows" }).first().click();
    const expected = state === "missing" ? /ещё не выпущена/i : /не удалось связаться/i;
    await page.getByRole("dialog").getByText(expected).waitFor();
    if (state === "unavailable") await page.getByRole("button", { name: "Повторить" }).waitFor();
    await page.close();
  }
}

async function assertReducedMotionAndObserverFallback() {
  const reducedPage = await browser.newPage({ viewport: { width: 390, height: 844 }, reducedMotion: "reduce" });
  await reducedPage.goto(origin, { waitUntil: "networkidle" });
  const hiddenRevealCount = await reducedPage.locator(".reveal").evaluateAll((nodes) => nodes.filter((node) => {
    const style = getComputedStyle(node);
    return style.opacity === "0" || style.visibility === "hidden";
  }).length);
  if (hiddenRevealCount > 0) throw new Error(`Reduced motion left ${hiddenRevealCount} reveal surfaces hidden`);
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
    const botHref = await page.locator('.site-footer a[href^="https://t.me/BeterFyBot"]').getAttribute("href");
    if (!botHref?.startsWith("https://t.me/BeterFyBot")) throw new Error(`Unexpected Telegram link: ${botHref}`);

    const journeyTabs = page.getByRole("tab");
    await journeyTabs.first().focus();
    await page.keyboard.press("ArrowRight");
    if (await journeyTabs.nth(1).getAttribute("aria-selected") !== "true") throw new Error(`${testCase.language} ${testCase.width}px journey tabs do not support arrow keys`);

    if (testCase.width <= 1100) {
      const menuButton = page.getByRole("button", { name: testCase.language === "ru" ? "Меню" : "Menu" });
      await menuButton.click();
      await page.locator("#mobile-navigation").waitFor();
      await page.keyboard.press("Escape");
      if (await page.locator("#mobile-navigation").count()) throw new Error(`${testCase.language} ${testCase.width}px mobile menu did not close with Escape`);
      await menuButton.locator(":scope:focus").waitFor();
    }

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
  await assertSuccessfulAuthWithoutSessionStorage();
  await assertIdSignInFlow();
  await assertReleaseStates();
  await assertReducedMotionAndObserverFallback();
  console.log(`BetterFy website: ${cases.length} responsive checks, mobile navigation, modal focus, auth/release failures, journey keyboard controls, reduced motion, and storage fallbacks passed.`);
} finally {
  await browser.close();
}
