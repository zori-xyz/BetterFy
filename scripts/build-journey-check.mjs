import { mkdir } from "node:fs/promises";
import { chromium } from "playwright";
import { fileURLToPath } from "node:url";

const base = "http://127.0.0.1:1420";
const output = fileURLToPath(new URL("../.impeccable/screens/", import.meta.url));
await mkdir(output, { recursive: true });

const browser = await chromium.launch({ headless: true });
const errors = [];

async function reachBuild(page) {
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(error.message));

  await page.goto(base, { waitUntil: "networkidle" });
  await page.evaluate(() => localStorage.clear());
  await page.reload({ waitUntil: "networkidle" });
  await page.getByRole("link", { name: /Открыть BetterFy Bot|Open BetterFy Bot/ }).waitFor();
  await page.getByRole("button", { name: /У меня уже есть код|I already have a code/ }).click();
  await page.locator(".otp-field input").fill("123456");
  await page.getByRole("button", { name: /Подтвердить|Confirm/ }).click();
  await page.getByRole("button", { name: /Найти автоматически|Find automatically/ }).waitFor();
  await page.getByRole("button", { name: /Продолжить в preview-режиме|Continue in preview mode/ }).click();
  await page.getByRole("button", { name: /Перейти на главную|Continue to Home/ }).click();
  await page.getByRole("button", { name: /Открыть сборку|Open build/ }).click();
  await page.getByRole("heading", { name: /Собери свою версию Dota|Compose your own Dota/ }).waitFor();
}

async function capture(viewport, suffix) {
  const page = await browser.newPage({ viewport });
  await reachBuild(page);

  await page.screenshot({ path: `${output}/polish-build-review-${suffix}.png` });
  await page.getByRole("button", { name: /Проверить тестовый план|Inspect fixture plan/ }).click();
  await page.getByRole("heading", { name: /Найден конфликт|Fixture conflict detected/ }).waitFor();
  await page.screenshot({ path: `${output}/polish-build-conflict-${suffix}.png` });

  await page.getByRole("button", { name: /Оставить Violet|Keep Violet/ }).click();
  await page.getByRole("heading", { name: /Решения приняты|Every decision is resolved/ }).waitFor();
  await page.screenshot({ path: `${output}/polish-build-ready-${suffix}.png` });

  await page.getByRole("button", { name: /Запустить preview-сборку|Start preview build/ }).click();
  await page.getByRole("heading", { name: /Сборка обретает форму|Your build is taking shape/ }).waitFor();
  await page.waitForTimeout(1150);
  await page.screenshot({ path: `${output}/polish-build-progress-${suffix}.png` });

  await page.getByRole("button", { name: /Смоделировать ошибку|Simulate an error/ }).click();
  await page.getByRole("heading", { name: /Вернём всё|Return to a calm state/ }).waitFor();
  await page.locator(".recovery-art img").evaluate((image) => {
    if (image instanceof HTMLImageElement && !image.complete) {
      return new Promise((resolve) => image.addEventListener("load", resolve, { once: true }));
    }
  });
  await page.waitForTimeout(320);
  await page.screenshot({ path: `${output}/polish-build-recovery-${suffix}.png` });
  await page.getByRole("button", { name: /Восстановить staging|Restore staging/ }).click();
  await page.getByRole("heading", { name: /Временное состояние очищено|Temporary state cleared/ }).waitFor();
  await page.screenshot({ path: `${output}/polish-build-restored-${suffix}.png` });

  await page.getByRole("button", { name: /Вернуться к плану|Return to plan/ }).click();
  await page.getByRole("button", { name: /Проверить тестовый план|Inspect fixture plan/ }).click();
  await page.getByRole("button", { name: /Оставить Violet|Keep Violet/ }).click();
  await page.getByRole("button", { name: /Запустить preview-сборку|Start preview build/ }).click();
  await page.getByRole("heading", { name: /Можно отправляться в игру|Ready for the match/ }).waitFor({ timeout: 10000 });
  await page.screenshot({ path: `${output}/polish-build-success-${suffix}.png` });

  const result = await page.evaluate(() => {
    const stage = document.querySelector(".build-result");
    const art = document.querySelector(".result-art img");
    const stageBox = stage?.getBoundingClientRect();
    const artBox = art?.getBoundingClientRect();
    return {
      horizontalOverflow: document.documentElement.scrollWidth > innerWidth,
      successVisible: Boolean(stageBox && stageBox.top < innerHeight && stageBox.bottom > 0),
      characterSceneRatio: stageBox && artBox ? Number((artBox.height / stageBox.height).toFixed(2)) : null,
      playDisabled: document.querySelector(".play-action")?.hasAttribute("disabled") ?? false,
    };
  });

  await page.close();
  return result;
}

const desktop = await capture({ width: 1440, height: 900 }, "1440");
const minimum = await capture({ width: 980, height: 660 }, "980");
console.log(JSON.stringify({ errors, desktop, minimum }, null, 2));
await browser.close();
