import { mkdir } from "node:fs/promises";
import { chromium } from "playwright";
import { fileURLToPath } from "node:url";

const base = "http://127.0.0.1:1420";
const output = fileURLToPath(new URL("../.impeccable/screens/", import.meta.url));
await mkdir(output, { recursive: true });

const browser = await chromium.launch({ headless: true });
const errors = [];

async function reachCatalog(page) {
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(base, { waitUntil: "networkidle" });
  await page.evaluate(() => localStorage.clear());
  await page.reload({ waitUntil: "networkidle" });
  await page.getByRole("button", { name: /У меня уже есть код|I already have a code/ }).click();
  await page.locator(".otp-field input").fill("123456");
  await page.getByRole("button", { name: /Подтвердить|Confirm/ }).click();
  await page.getByRole("button", { name: /Найти автоматически|Find automatically/ }).waitFor();
  await page.getByRole("button", { name: /Продолжить в preview-режиме|Continue in preview mode/ }).click();
  await page.getByRole("button", { name: /Перейти на главную|Continue to Home/ }).click();
  await page.getByRole("button", { name: /Каталог|Discover/ }).click();
  await page.getByRole("heading", { name: /Моды, которые меняют|Mods that change/ }).waitFor();
  await page.locator(".catalog-feature").waitFor();
  await page.waitForTimeout(900);
}

async function capture(viewport, suffix) {
  const page = await browser.newPage({ viewport });
  await reachCatalog(page);
  await page.screenshot({ path: `${output}/catalog-mods-${suffix}.png`, fullPage: false });

  const firstAction = page.locator(".catalog-primary");
  await firstAction.click();
  await page.getByText(/В сборке|In build/, { exact: true }).waitFor();
  await page.getByRole("tab", { name: /Интерфейс|Interface/ }).click();
  await page.getByPlaceholder(/Найти мод|Find a mod/).fill("Monocraft");
  await page.getByRole("heading", { name: "Monocraft" }).waitFor();
  await page.waitForTimeout(400);
  await page.screenshot({ path: `${output}/catalog-search-${suffix}.png`, fullPage: false });
  await page.getByRole("button", { name: "EN" }).click();
  await page.getByRole("heading", { name: /Mods that change/ }).waitFor();
  await page.waitForTimeout(250);
  await page.screenshot({ path: `${output}/catalog-en-${suffix}.png`, fullPage: false });

  const metrics = await page.evaluate(() => ({
    horizontalOverflow: document.documentElement.scrollWidth > document.documentElement.clientWidth,
    resultCount: document.querySelectorAll(".catalog-grid article").length,
    failedPreviews: document.querySelectorAll(".catalog-image-fallback").length,
    selectedCount: document.querySelectorAll(".catalog-grid article.selected").length,
    featureVisible: Boolean(document.querySelector(".catalog-feature")),
  }));
  await page.close();
  return metrics;
}

const desktop = await capture({ width: 1440, height: 900 }, "1440");
const minimum = await capture({ width: 980, height: 660 }, "980");
await browser.close();

console.log(JSON.stringify({ errors, desktop, minimum }, null, 2));
