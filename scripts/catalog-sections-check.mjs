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
  await page.getByRole("heading", { name: /Убери лишнее|Remove the noise/ }).waitFor();
}

async function capture(viewport, suffix) {
  const page = await browser.newPage({ viewport });
  await reachCatalog(page);
  await page.waitForTimeout(500);
  await page.screenshot({ path: `${output}/catalog-minify-${suffix}.png` });

  const minify = await page.evaluate(() => ({
    documentOverflow: document.documentElement.scrollWidth > innerWidth,
    routeOverflow: (document.querySelector(".catalog-hub")?.scrollWidth ?? 0) >
      (document.querySelector(".catalog-hub")?.clientWidth ?? 0),
    cards: document.querySelectorAll(".minify-grid article").length,
    previews: document.querySelectorAll(".minify-grid img").length,
    switchVisible: Boolean(document.querySelector(".catalog-section-switch")),
    categoryTabs: document.querySelectorAll(".minify-controls [role='tab']").length,
  }));
  await page.locator(".minify-grid article").first().scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${output}/catalog-minify-cards-${suffix}.png` });
  await page.locator(".minify-grid article .minify-card-toggle").first().click();
  await page.getByRole("button", { name: /Только выбранные|Selected only/ }).click();
  minify.selectedFilterCards = await page.locator(".minify-grid article").count();
  minify.selectedCardVisible = await page.locator(".minify-grid article.selected").count() === 1;

  await page.getByRole("button", { name: /Гардероб|Wardrobe/ }).click();
  await page.getByRole("heading", { name: /Визуальная коллекция|visual collection/ }).waitFor();
  await page.waitForTimeout(350);
  await page.screenshot({ path: `${output}/catalog-wardrobe-${suffix}.png` });
  const wardrobe = await page.evaluate(() => ({
    documentOverflow: document.documentElement.scrollWidth > innerWidth,
    switchVisible: Boolean(document.querySelector(".catalog-section-switch")),
    featureVisible: Boolean(document.querySelector(".catalog-feature")),
    groupTabs: document.querySelectorAll(".catalog-groups [role='tab']").length,
    fineCategories: document.querySelectorAll(".catalog-fine-categories button").length,
  }));
  await page.locator(".catalog-grid article").first().scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${output}/catalog-wardrobe-cards-${suffix}.png` });
  await page.getByRole("button", { name: /Моды|Mods/ }).click();
  await page.getByRole("button", { name: "EN", exact: true }).click();
  await page.getByRole("heading", { name: /Remove the noise/ }).waitFor();
  await page.screenshot({ path: `${output}/catalog-minify-en-${suffix}.png` });
  const english = await page.evaluate(() => ({
    documentOverflow: document.documentElement.scrollWidth > innerWidth,
    routeOverflow: (document.querySelector(".catalog-hub")?.scrollWidth ?? 0) >
      (document.querySelector(".catalog-hub")?.clientWidth ?? 0),
    titleVisible: Boolean(document.querySelector(".minify-heading h1")),
  }));
  await page.close();
  return { minify, wardrobe, english };
}

const desktop = await capture({ width: 1440, height: 900 }, "1440");
const minimum = await capture({ width: 980, height: 660 }, "980");
console.log(JSON.stringify({ errors, desktop, minimum }, null, 2));
await browser.close();
