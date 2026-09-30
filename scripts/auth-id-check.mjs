import { chromium } from "playwright";
import { mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";

const base = process.env.BETTERFY_APP_URL ?? "http://127.0.0.1:1420/";
const output = new URL("../.impeccable/screens/", import.meta.url);
mkdirSync(output, { recursive: true });
const browser = await chromium.launch();

try {
  for (const [width, height] of [[320, 568], [390, 844], [980, 660], [1180, 760], [1440, 900]]) {
    const page = await browser.newPage({ viewport: { width, height }, reducedMotion: "reduce" });
    await page.goto(base, { waitUntil: "networkidle" });
    await page.getByRole("button", { name: /BetterFy ID.*(?:пароль|password)/ }).waitFor();
    await page.screenshot({ path: fileURLToPath(new URL(`auth-id-${width}.png`, output)) });
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth > innerWidth);
    if (overflow) throw new Error(`Auth overflow at ${width} px`);
    await page.getByRole("button", { name: /BetterFy ID.*пароль/ }).click();
    await page.getByRole("textbox", { name: "Почта или никнейм" }).waitFor();
    await page.screenshot({ path: fileURLToPath(new URL(`auth-id-login-${width}.png`, output)) });
    await page.getByRole("button", { name: "Создать аккаунт" }).click();
    await page.getByRole("textbox", { name: "Никнейм" }).waitFor();
    await page.screenshot({ path: fileURLToPath(new URL(`auth-id-register-${width}.png`, output)) });
    await page.getByRole("button", { name: "Назад" }).click();
    await page.getByRole("button", { name: "Назад" }).click();
    await page.getByRole("button", { name: "EN" }).click();
    await page.getByRole("button", { name: /BetterFy ID.*password/ }).waitFor();
    await page.getByRole("button", { name: /BetterFy ID.*password/ }).click();
    await page.getByRole("textbox", { name: "Email or username" }).waitFor();
    const idOverflow = await page.evaluate(() => document.documentElement.scrollWidth > innerWidth);
    if (idOverflow) throw new Error(`English ID form overflow at ${width} px`);
    await page.close();
  }
  console.log("BetterFy ID: 5 viewports, RU/EN, choice, sign-in and registration forms passed.");
} finally {
  await browser.close();
}
