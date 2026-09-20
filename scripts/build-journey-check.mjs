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
  await page.getByRole("button", { name: /Открыть BetterFy Bot|Open BetterFy Bot/ }).waitFor();
  await page.locator(".auth-preview-button").click();
  await page.getByRole("button", { name: /Выбрать моды|Выбрать ещё|Choose mods|Choose more/ }).waitFor();
  await page.getByRole("button", { name: /Сборка|Build/, exact: true }).click();
  await page.getByRole("heading", { name: /Начни с первого мода|Start with your first mod|Всё выбранное|Everything you chose/ }).waitFor();
}

async function capture(viewport, suffix) {
  const page = await browser.newPage({ viewport });
  await reachBuild(page);

  await page.screenshot({ path: `${output}/polish-build-review-${suffix}.png` });
  await page.getByRole("button", { name: /Посмотреть, как работает проверка|See how checking works/ }).click();
  await page.getByRole("heading", { name: /Один ресурс|One resource/ }).waitFor();
  await page.screenshot({ path: `${output}/polish-build-conflict-${suffix}.png` });

  await page.getByRole("button", { name: /Выбрать Violet|Choose Violet/ }).click();
  await page.getByRole("heading", { name: /Решения приняты|Every decision is resolved/ }).waitFor();
  await page.screenshot({ path: `${output}/polish-build-ready-${suffix}.png` });

  await page.getByRole("button", { name: /Запустить preview-сборку|Start preview build/ }).click();
  await page.getByRole("heading", { name: /Сборка обретает форму|Your build is taking shape/ }).waitFor();
  await page.waitForTimeout(1150);
  await page.screenshot({ path: `${output}/polish-build-progress-${suffix}.png` });

  await page.getByRole("button", { name: /Смоделировать ошибку|Simulate an error/ }).click();
  await page.getByRole("heading", { name: /Очистим временную сборку|Clear the temporary build/ }).waitFor();
  await page.waitForTimeout(320);
  await page.screenshot({ path: `${output}/polish-build-recovery-${suffix}.png` });
  await page.getByRole("button", { name: /Восстановить staging|Restore staging/ }).click();
  await page.getByRole("heading", { name: /Временное состояние очищено|Temporary state cleared/ }).waitFor();
  await page.screenshot({ path: `${output}/polish-build-restored-${suffix}.png` });

  await page.getByRole("button", { name: /Вернуться к плану|Return to plan/ }).click();
  await page.getByRole("button", { name: /Посмотреть, как работает проверка|See how checking works/ }).click();
  await page.getByRole("button", { name: /Выбрать Violet|Choose Violet/ }).click();
  await page.getByRole("button", { name: /Запустить preview-сборку|Start preview build/ }).click();
  await page.getByRole("heading", { name: /Preview завершён|Preview complete/ }).waitFor({ timeout: 10000 });
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
