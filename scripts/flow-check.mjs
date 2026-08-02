import { chromium } from "playwright";

const executablePath =
  "/Users/zori/Library/Caches/ms-playwright/chromium_headless_shell-1228/chrome-headless-shell-mac-arm64/chrome-headless-shell";
const browser = await chromium.launch({ headless: true, executablePath });
const page = await browser.newPage({ viewport: { width: 1180, height: 760 } });
const errors = [];
page.on("console", (message) => {
  if (message.type() === "error") errors.push(message.text());
});
page.on("pageerror", (error) => errors.push(error.message));

await page.goto("http://127.0.0.1:1420", { waitUntil: "networkidle" });
await page.evaluate(() => localStorage.clear());
await page.reload({ waitUntil: "networkidle" });
await page.getByText(/Готовим твою Dota|Preparing your Dota/).waitFor();
const botLink = page.getByRole("link", { name: /Открыть BetterFy Bot|Open BetterFy Bot/ });
await botLink.waitFor();
const botHref = await botLink.getAttribute("href");

await page.getByRole("button", { name: /У меня уже есть код|I already have a code/ }).click();
await page.locator(".otp-field input").fill("000000");
await page.getByRole("button", { name: /Подтвердить|Confirm/ }).click();
await page.getByText(/Код недействителен|code is invalid/i).waitFor();

await page.locator(".otp-field input").fill("123456");
await page.getByRole("button", { name: /Подтвердить|Confirm/ }).click();
await page.getByText(/Проверяем код|Checking your code/).waitFor();
await page.getByRole("button", { name: /Найти автоматически|Find automatically/ }).waitFor();
await page.getByRole("button", { name: /Найти автоматически|Find automatically/ }).click();
await page.getByRole("button", { name: /Перейти на главную|Continue to Home/ }).waitFor();
const setupVerification = await page.getByText(/демо|demo/i).count();
await page.getByRole("button", { name: /Перейти на главную|Continue to Home/ }).click();
await page.getByRole("button", { name: /Открыть сборку|Open build/ }).waitFor();

await page.getByRole("button", { name: "EN" }).click();
const language = await page.evaluate(() => document.documentElement.lang);
const homeTitle = await page.getByRole("heading", { level: 1 }).innerText();
const primaryLabel = await page.getByRole("button", { name: /Open build/ }).innerText();
const communityHref = await page.getByRole("link", { name: /Open @BetterFyBot/ }).first().getAttribute("href");
await page.getByRole("button", { name: /^Build$/ }).click();
const buildRoute = await page.getByRole("heading", { name: /Compose your own Dota/ }).count();
await page.getByRole("button", { name: /Inspect fixture plan/ }).click();
await page.getByRole("heading", { name: /Fixture conflict detected/ }).waitFor();
const fixtureConflict = await page.getByText(/Two fixture mods target the same resource/).count();
await page.getByRole("button", { name: /^Library$/ }).click();
const libraryRoute = await page.getByRole("heading", { name: /Your library/ }).count();
await page.getByRole("button", { name: /Settings/ }).click();
const settingsPanel = await page.getByRole("dialog", { name: /Settings/ }).count();
await page.getByRole("switch", { name: /Interface motion/ }).click();
const motionEnabled = await page.getByRole("switch", { name: /Interface motion/ }).getAttribute("aria-checked");
await page.getByRole("switch", { name: /Launch with Windows/ }).click();
const startupPreference = await page.getByRole("switch", { name: /Launch with Windows/ }).getAttribute("aria-checked");
await page.getByRole("button", { name: /Run diagnostics/ }).click();
await page.getByText(/Demo check completed/).waitFor();
const diagnosticState = await page.getByText(/Demo check completed/).count();
await page.keyboard.press("Escape");
const settingsClosedByKeyboard = await page.getByRole("dialog", { name: /Settings/ }).count();
await page.getByRole("button", { name: /Profile/ }).click();
const profilePanel = await page.getByRole("dialog", { name: /Profile/ }).count();

const result = {
  errors,
  botHref,
  communityHref,
  language,
  homeTitle,
  primaryLabel,
  setupVerification,
  buildRoute,
  fixtureConflict,
  libraryRoute,
  horizontalOverflow: await page.evaluate(() => document.documentElement.scrollWidth > innerWidth),
  stage: await page.locator(".home-surface").count(),
  settingsPanel,
  profilePanel,
  motionEnabled,
  startupPreference,
  diagnosticState,
  settingsClosedByKeyboard,
};

console.log(JSON.stringify(result, null, 2));
await browser.close();
