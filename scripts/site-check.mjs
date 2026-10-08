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

async function assertSeoSafetyAndStaticPages() {
  const base = new URL(origin);
  // Language comes from ?lang= first, and the head follows it.
  const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
  await page.goto(`${origin}?lang=en`, { waitUntil: "networkidle" });
  if (await page.evaluate(() => document.documentElement.lang) !== "en") throw new Error("?lang=en did not switch the page language");
  if (!/Dota 2 mods/.test(await page.title())) throw new Error(`English title is wrong: ${await page.title()}`);
  const description = await page.locator('meta[name="description"]').getAttribute("content");
  if (!/Windows app/.test(description ?? "")) throw new Error("English meta description was not applied");
  await page.getByRole("button", { name: "RU", exact: true }).click();
  if (!(await page.url()).includes("lang=ru")) throw new Error("Language switch did not keep ?lang= in sync");
  if (!/моды для Dota 2/.test(await page.title())) throw new Error("Russian title was not restored");
  const canonical = await page.locator('link[rel="canonical"]').getAttribute("href");
  if (canonical !== "https://zori-xyz.github.io/BetterFy/") throw new Error(`Unexpected canonical: ${canonical}`);
  for (const selector of ['meta[property="og:image"]', 'meta[name="twitter:image"]']) {
    const href = await page.locator(selector).getAttribute("content");
    const response = await fetch(new URL(new URL(href).pathname, base.origin));
    if (!response.ok) throw new Error(`${selector} points to a missing image: ${href}`);
  }
  // The safety section and the legal links.
  if (await page.locator("#safety .safety-card").count() !== 3) throw new Error("Safety section must list read / write / never cards");
  for (const name of ["privacy.html", "terms.html"]) {
    if (!(await page.locator(`.site-footer a[href$="${name}"]`).count())) throw new Error(`Footer has no link to ${name}`);
    const response = await fetch(new URL(name, origin));
    if (!response.ok) throw new Error(`${name} did not load: ${response.status}`);
  }
  await page.close();
  for (const name of ["robots.txt", "sitemap.xml", "site.webmanifest", "404.html", "favicon-32.png", "apple-touch-icon.png"]) {
    const response = await fetch(new URL(name, origin));
    if (!response.ok) throw new Error(`${name} did not load: ${response.status}`);
  }
  // The static pages switch language without the app bundle.
  const legal = await browser.newPage({ viewport: { width: 390, height: 844 } });
  await legal.goto(new URL("privacy.html?lang=en", origin).toString(), { waitUntil: "networkidle" });
  if (!(await legal.getByRole("heading", { name: "Privacy", level: 1 }).isVisible())) throw new Error("privacy.html did not show the English heading");
  const legalOverflow = await legal.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  if (legalOverflow > 1) throw new Error(`privacy.html overflows horizontally by ${legalOverflow}px`);
  await legal.close();
}

async function assertThemeAndStatus() {
  for (const colorScheme of ["dark", "light"]) {
    const page = await browser.newPage({ viewport: { width: 1440, height: 1000 }, colorScheme });
    await page.goto(origin, { waitUntil: "networkidle" });
    const initial = await page.evaluate(() => document.documentElement.dataset.theme);
    if (initial !== colorScheme) throw new Error(`System ${colorScheme} preference produced theme ${initial}`);
    const next = colorScheme === "dark" ? "light" : "dark";
    await page.getByRole("button", { name: next === "light" ? "Включить светлую тему" : "Включить тёмную тему" }).click();
    if (await page.evaluate(() => document.documentElement.dataset.theme) !== next) throw new Error(`Theme button did not switch to ${next}`);
    const rootColors = await page.evaluate(() => {
      const root = getComputedStyle(document.getElementById("root"));
      const body = getComputedStyle(document.body);
      return root.backgroundColor === body.backgroundColor && root.color === body.color;
    });
    if (!rootColors) throw new Error(`#root kept the pre-load colours after switching to ${next}`);
    await page.reload({ waitUntil: "networkidle" });
    if (await page.evaluate(() => document.documentElement.dataset.theme) !== next) throw new Error(`Theme ${next} was not remembered`);
    await page.close();
  }
  const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
  await page.goto(origin, { waitUntil: "networkidle" });
  const verified = await page.locator("#status .console-list li.is-ok").count();
  const unverified = await page.locator("#status .console-list li:not(.is-ok)").count();
  if (verified === 0 || unverified === 0) throw new Error("Status journal must list both verified and unverified facts");
  const mods = await page.locator("#mods .mod-group li").count();
  if (mods !== 25) throw new Error(`Mod catalog lists ${mods} packages, expected 25`);
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

    const faqToggle = page.locator(".faq-item > button").nth(1);
    await faqToggle.click();
    if (await faqToggle.getAttribute("aria-expanded") !== "true") throw new Error(`${testCase.language} ${testCase.width}px FAQ answers do not open`);

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
  await assertThemeAndStatus();
  await assertSeoSafetyAndStaticPages();
  console.log(`BetterFy website: ${cases.length} responsive checks, mobile navigation, modal focus, auth/release failures, FAQ disclosure, reduced motion, storage fallbacks, theme switching, the status journal, SEO head, the safety section and the static pages passed.`);
} finally {
  await browser.close();
}
