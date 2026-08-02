import { chromium } from "playwright";
import { fileURLToPath } from "node:url";

const base = "http://127.0.0.1:1420";
const output = fileURLToPath(new URL("../.impeccable/screens/", import.meta.url));
const executablePath =
  "/Users/zori/Library/Caches/ms-playwright/chromium_headless_shell-1228/chrome-headless-shell-mac-arm64/chrome-headless-shell";
const browser = await chromium.launch({ headless: true, executablePath });
const errors = [];
const layoutIssues = [];
const contrastIssues = [];

const auditLayout = async (page, label) => {
  const issues = await page.evaluate((screenLabel) => {
    const candidates = document.querySelectorAll(
      "h1, h2, h3, p, button, a, .section-label, .scene-copy > span, .scene-copy > p",
    );
    return [...candidates].flatMap((element) => {
      const style = getComputedStyle(element);
      const rect = element.getBoundingClientRect();
      if (
        style.display === "none" ||
        style.visibility === "hidden" ||
        element.matches(".minify-card-main") ||
        rect.width === 0 ||
        rect.height === 0
      ) return [];
      const overflows =
        element.scrollWidth > element.clientWidth + 2 ||
        element.scrollHeight > element.clientHeight + 2;
      const canClip =
        ["hidden", "clip"].includes(style.overflowX) ||
        ["hidden", "clip"].includes(style.overflowY);
      return overflows && canClip
        ? [{
            screen: screenLabel,
            tag: element.tagName.toLowerCase(),
            className: element.className,
            text: (element.textContent ?? "").trim().slice(0, 80),
            client: [element.clientWidth, element.clientHeight],
            scroll: [element.scrollWidth, element.scrollHeight],
          }]
        : [];
    });
  }, label);
  layoutIssues.push(...issues);

  const spacing = await page.evaluate((screenLabel) => {
    const pairs = [
      [".auth-view .section-label", ".auth-view h1"],
      [".auth-view h1", ".auth-view > p"],
      [".scene-copy > span", ".scene-copy h2"],
      [".scene-copy h2", ".scene-copy > p"],
      [".setup-copy > span", ".setup-copy h1"],
      [".setup-copy h1", ".setup-copy > p"],
      [".route-heading > span", ".route-heading h1"],
      [".route-heading h1", ".route-heading > p"],
    ];
    return pairs.flatMap(([fromSelector, toSelector]) => {
      const from = document.querySelector(fromSelector);
      const to = document.querySelector(toSelector);
      if (!from || !to) return [];
      const fromStyle = getComputedStyle(from);
      const toStyle = getComputedStyle(to);
      if (
        fromStyle.display === "none" ||
        toStyle.display === "none" ||
        from.getBoundingClientRect().height === 0 ||
        to.getBoundingClientRect().height === 0
      ) return [];
      const gap = to.getBoundingClientRect().top - from.getBoundingClientRect().bottom;
      return gap < 10
        ? [{ screen: screenLabel, kind: "spacing", pair: [fromSelector, toSelector], gap }]
        : [];
    });
  }, label);
  layoutIssues.push(...spacing);

  const sceneLineSpacing = await page.evaluate((screenLabel) => {
    const heading = document.querySelector(".scene-copy h2");
    if (!heading || heading.getBoundingClientRect().height === 0) return [];
    const style = getComputedStyle(heading);
    const fontSize = Number.parseFloat(style.fontSize);
    const lineHeight = Number.parseFloat(style.lineHeight);
    const ratio = lineHeight / fontSize;
    return ratio < 1.03
      ? [{ screen: screenLabel, kind: "scene-line-height", ratio }]
      : [];
  }, label);
  layoutIssues.push(...sceneLineSpacing);
};

const auditContrast = async (page, label) => {
  const issues = await page.evaluate((screenLabel) => {
    const parse = (value) => {
      const match = value.match(/rgba?\(([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)(?:[,\s/]+([\d.]+))?\)/);
      return match
        ? [Number(match[1]), Number(match[2]), Number(match[3]), match[4] === undefined ? 1 : Number(match[4])]
        : null;
    };
    const composite = (front, back) => {
      const alpha = front[3] + back[3] * (1 - front[3]);
      if (!alpha) return [0, 0, 0, 0];
      return [
        (front[0] * front[3] + back[0] * back[3] * (1 - front[3])) / alpha,
        (front[1] * front[3] + back[1] * back[3] * (1 - front[3])) / alpha,
        (front[2] * front[3] + back[2] * back[3] * (1 - front[3])) / alpha,
        alpha,
      ];
    };
    const luminance = (color) => {
      const channels = color.slice(0, 3).map((value) => {
        const channel = value / 255;
        return channel <= .04045 ? channel / 12.92 : ((channel + .055) / 1.055) ** 2.4;
      });
      return channels[0] * .2126 + channels[1] * .7152 + channels[2] * .0722;
    };
    const contrast = (a, b) => {
      const light = Math.max(luminance(a), luminance(b));
      const dark = Math.min(luminance(a), luminance(b));
      return (light + .05) / (dark + .05);
    };
    const sceneSelector = [
      ".auth-scene", ".setup-scene", ".hero-art-slot", ".build-character-scene",
      ".operation-world", ".result-art", ".catalog-feature-visual",
      ".minify-feature-visual", ".catalog-card-preview", ".minify-card-preview",
    ].join(",");
    const modal = document.querySelector('[aria-modal="true"]');

    return [...document.querySelectorAll("body *")].flatMap((element) => {
      if (!(element instanceof HTMLElement)) return [];
      const directText = [...element.childNodes]
        .filter((node) => node.nodeType === Node.TEXT_NODE)
        .map((node) => node.textContent?.trim() ?? "")
        .join(" ")
        .trim();
      if (
        !directText ||
        element.closest(sceneSelector) ||
        (modal && !modal.contains(element))
      ) return [];
      const style = getComputedStyle(element);
      const rect = element.getBoundingClientRect();
      const gradientControl = element.closest("button, a");
      const gradientText =
        style.backgroundClip === "text" ||
        style.webkitBackgroundClip === "text" ||
        style.webkitTextFillColor === "transparent";
      if (
        style.display === "none" ||
        style.visibility === "hidden" ||
        Number(style.opacity) === 0 ||
        rect.width < 2 ||
        rect.height < 2 ||
        gradientText ||
        (gradientControl && getComputedStyle(gradientControl).backgroundImage !== "none")
      ) return [];

      const chain = [];
      let current = element;
      while (current) {
        chain.unshift(current);
        current = current.parentElement;
      }
      let background = [255, 255, 255, 1];
      for (const node of chain) {
        const layer = parse(getComputedStyle(node).backgroundColor);
        if (layer && layer[3] > 0) background = composite(layer, background);
      }
      const foreground = parse(style.color);
      if (!foreground) return [];
      let opacity = 1;
      let opacityNode = element;
      while (opacityNode) {
        opacity *= Number(getComputedStyle(opacityNode).opacity || 1);
        opacityNode = opacityNode.parentElement;
      }
      foreground[3] *= opacity;
      const painted = composite(foreground, background);
      const ratio = contrast(painted, background);
      const fontSize = Number.parseFloat(style.fontSize);
      const fontWeight = Number.parseInt(style.fontWeight, 10) || 400;
      const large = fontSize >= 24 || (fontSize >= 18.66 && fontWeight >= 700);
      const disabled = Boolean(element.closest(":disabled"));
      const threshold = disabled ? 2.8 : large ? 3 : 4.5;
      return ratio + .05 < threshold
        ? [{
            screen: screenLabel,
            selector: `${element.tagName.toLowerCase()}.${element.className}`.slice(0, 120),
            text: directText.slice(0, 90),
            ratio: Number(ratio.toFixed(2)),
            threshold,
            color: style.color,
            background: background.slice(0, 3).map((value) => Math.round(value)),
            fontSize,
          }]
        : [];
    });
  }, label);
  contrastIssues.push(...issues);
};

const capture = async (viewport, suffix) => {
  const page = await browser.newPage({ viewport });
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(error.message));

  await page.goto(base, { waitUntil: "networkidle" });
  await page.evaluate(() => localStorage.clear());
  await page.reload({ waitUntil: "networkidle" });
  await page.screenshot({ path: `${output}/new-loading-${suffix}.png` });
  await page.waitForTimeout(1250);
  await page.screenshot({ path: `${output}/new-loading-complete-${suffix}.png` });

  await page.getByRole("link", { name: /Открыть BetterFy Bot|Open BetterFy Bot/ }).waitFor();
  await page.waitForTimeout(420);
  await auditLayout(page, `auth-${suffix}`);
  await page.screenshot({ path: `${output}/new-auth-${suffix}.png` });
  await page.getByRole("button", { name: "EN" }).click();
  await page.waitForTimeout(420);
  await auditLayout(page, `auth-en-${suffix}`);
  await page.screenshot({ path: `${output}/new-auth-en-${suffix}.png` });
  await page.getByRole("button", { name: /У меня уже есть код|I already have a code/ }).click();
  await page.waitForTimeout(420);
  await page.screenshot({ path: `${output}/new-code-${suffix}.png` });
  await page.locator(".otp-field input").fill("123456");
  await page.getByRole("button", { name: /Подтвердить|Confirm/ }).click();
  await page.locator(".confirmed-view").waitFor();
  await auditLayout(page, `confirmed-${suffix}`);
  await page.screenshot({ path: `${output}/new-confirmed-${suffix}.png` });
  await page.getByRole("button", { name: /Find automatically/ }).waitFor();
  await page.waitForTimeout(420);
  await auditLayout(page, `setup-${suffix}`);
  await page.screenshot({ path: `${output}/new-setup-${suffix}.png` });
  await page.getByRole("button", { name: /Find automatically/ }).click();
  await page.getByRole("button", { name: /Continue to Home/ }).waitFor();
  await page.waitForTimeout(360);
  await page.screenshot({ path: `${output}/new-setup-found-${suffix}.png` });
  await page.getByRole("button", { name: /Continue to Home/ }).click();
  await page.getByRole("button", { name: /Open build/ }).waitFor();
  await page.waitForTimeout(520);
  await auditLayout(page, `home-${suffix}`);
  await page.screenshot({ path: `${output}/new-home-${suffix}.png` });
  await page.getByRole("button", { name: /^Discover$/ }).click();
  await page.locator(".minify-heading").waitFor();
  await page.waitForTimeout(360);
  await auditLayout(page, `catalog-top-${suffix}`);
  await page.screenshot({ path: `${output}/new-catalog-top-${suffix}.png` });
  await page.mouse.move(Math.round(viewport.width * .72), Math.round(viewport.height * .46));
  await page.locator(".catalog-hub").evaluate((element) => element.scrollTo({ top: 520 }));
  await page.waitForTimeout(520);
  await auditLayout(page, `catalog-compact-${suffix}`);
  await page.screenshot({ path: `${output}/new-catalog-compact-${suffix}.png` });
  const catalogCompact = await page.locator(".app-frame").evaluate((element) =>
    element.classList.contains("catalog-focus-mode"));
  if (!catalogCompact) errors.push(`Catalog compact mode did not activate at ${suffix}`);
  await page.getByRole("button", { name: /^Wardrobe/ }).click();
  await page.locator(".catalog-heading").waitFor();
  await page.waitForTimeout(420);
  await auditLayout(page, `wardrobe-compact-${suffix}`);
  await page.screenshot({ path: `${output}/new-wardrobe-compact-${suffix}.png` });
  await page.locator(".app-rail").hover();
  await page.getByRole("button", { name: /^Build$/ }).click();
  await page.getByRole("button", { name: /Inspect fixture plan/ }).click();
  await page.getByRole("heading", { name: /Fixture conflict detected/ }).waitFor();
  await page.waitForTimeout(260);
  await auditLayout(page, `build-${suffix}`);
  await page.screenshot({ path: `${output}/new-build-${suffix}.png` });
  await page.getByRole("button", { name: /^Library$/ }).click();
  await page.waitForTimeout(360);
  await auditLayout(page, `library-${suffix}`);
  await page.screenshot({ path: `${output}/new-library-${suffix}.png` });
  await page.getByRole("tab", { name: /BetterFy Workshop|Мастерская BetterFy/ }).click();
  await page.waitForTimeout(280);
  await auditLayout(page, `workshop-${suffix}`);
  await page.screenshot({ path: `${output}/new-workshop-${suffix}.png` });
  await page.getByRole("button", { name: /Settings/ }).click();
  await page.waitForTimeout(360);
  await auditLayout(page, `settings-dark-${suffix}`);
  await page.screenshot({ path: `${output}/new-settings-${suffix}.png` });
  await page.locator(".theme-segment button").first().click();
  await page.waitForTimeout(420);
  await auditLayout(page, `settings-light-${suffix}`);
  await page.screenshot({ path: `${output}/new-settings-light-${suffix}.png` });
  await page.getByRole("button", { name: /Закрыть|Close/ }).click();
  await page.waitForTimeout(260);
  await auditLayout(page, `library-light-${suffix}`);
  await page.screenshot({ path: `${output}/new-library-light-${suffix}.png` });
  await page.getByRole("button", { name: /Профиль|Profile/ }).click();
  await page.waitForTimeout(360);
  await page.screenshot({ path: `${output}/new-profile-${suffix}.png` });

  const metrics = await page.evaluate(() => ({
    viewport: { width: innerWidth, height: innerHeight },
    document: {
      width: document.documentElement.scrollWidth,
      height: document.documentElement.scrollHeight,
    },
    horizontalOverflow: document.documentElement.scrollWidth > innerWidth,
    buttons: document.querySelectorAll("button").length,
  }));
  await page.close();
  return metrics;
};

const captureMissingGame = async () => {
  const page = await browser.newPage({ viewport: { width: 980, height: 660 } });
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(error.message));

  await page.goto(`${base}/?game-missing=1`, { waitUntil: "networkidle" });
  await page.evaluate(() => {
    localStorage.clear();
    localStorage.setItem("betterfy:theme", "light");
  });
  await page.reload({ waitUntil: "networkidle" });
  await page.getByRole("link", { name: /Открыть BetterFy Bot|Open BetterFy Bot/ }).waitFor();
  await page.getByRole("button", { name: /У меня уже есть код|I already have a code/ }).click();
  await page.locator(".otp-field input").fill("123456");
  await page.getByRole("button", { name: /Подтвердить|Confirm/ }).click();
  await page.getByRole("button", { name: /Найти автоматически|Find automatically/ }).click();
  await page.getByRole("heading", { name: /Dota 2 не найдена|Dota 2 wasn't found/ }).waitFor();
  await page.waitForTimeout(520);
  await auditLayout(page, "setup-missing-light-980");
  await auditContrast(page, "setup-missing-light-980");
  await page.screenshot({ path: `${output}/new-setup-missing-980.png` });

  const state = await page.evaluate(() => {
    const title = document.querySelector(".setup-error-title h1 em");
    const icon = document.querySelector(".setup-alert-icon");
    const titleBox = title?.getBoundingClientRect();
    const iconBox = icon?.getBoundingClientRect();
    return {
      warningOnRight: Boolean(
        titleBox &&
          iconBox &&
          iconBox.left >= titleBox.right &&
          iconBox.left - titleBox.right <= 20 &&
          Math.abs(iconBox.top + iconBox.height / 2 - (titleBox.top + titleBox.height / 2)) < 24,
      ),
      previewAvailable: Boolean(
        [...document.querySelectorAll("button")].find((button) =>
          /Продолжить в (?:демо|preview)-режиме|Continue in preview mode/.test(button.textContent ?? ""),
        ),
      ),
    };
  });

  await page.getByRole("button", {
    name: /Продолжить в (?:демо|preview)-режиме|Continue in preview mode/,
  }).click();
  await page.getByRole("button", { name: /Перейти на главную|Continue to Home/ }).waitFor();
  state.previewReachesFound = true;
  await page.close();
  return state;
};

const auditReducedMotion = async () => {
  const page = await browser.newPage({ viewport: { width: 980, height: 660 } });
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(base, { waitUntil: "networkidle" });
  await page.waitForTimeout(80);
  const result = await page.evaluate(() => {
    const activeAnimations = document.getAnimations().filter((animation) => {
      const duration = Number(animation.effect?.getTiming().duration ?? 0);
      return duration > 1 && animation.playState !== "finished";
    });
    const transitionSamples = [
      document.querySelector(".launch-screen"),
      document.querySelector(".wordmark-better"),
      document.querySelector(".wordmark-fy"),
    ].filter(Boolean).map((element) => getComputedStyle(element).transitionDuration);
    return {
      activeAnimations: activeAnimations.length,
      transitionSamples,
      reduced: matchMedia("(prefers-reduced-motion: reduce)").matches,
    };
  });
  await page.close();
  return result;
};

const captureLightEntry = async (viewport, suffix, full = false) => {
  const page = await browser.newPage({ viewport });
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(base, { waitUntil: "networkidle" });
  await page.evaluate(() => {
    localStorage.clear();
    localStorage.setItem("betterfy:theme", "light");
  });
  await page.reload({ waitUntil: "networkidle" });
  const snapshot = async (name) => {
    const label = `${name}-light-${suffix}`;
    await auditLayout(page, label);
    await auditContrast(page, label);
    await page.screenshot({ path: `${output}/new-${name}-light-${suffix}.png` });
  };
  await snapshot("loading");
  await page.getByRole("link", { name: /Открыть BetterFy Bot|Open BetterFy Bot/ }).waitFor();
  await page.waitForTimeout(420);
  await snapshot("auth");
  if (!full) {
    await page.getByRole("button", { name: "EN" }).click();
    await page.waitForTimeout(300);
    await snapshot("auth-en");
  }
  await page.getByRole("button", { name: /У меня уже есть код|I already have a code/ }).click();
  await page.waitForTimeout(320);
  await snapshot("code");
  await page.locator(".otp-field input").fill("123456");
  await page.getByRole("button", { name: /Подтвердить|Confirm/ }).click();
  await page.locator(".confirmed-view").waitFor();
  await page.waitForTimeout(300);
  await snapshot("confirmed");
  await page.getByRole("button", { name: /Найти автоматически|Find automatically/ }).waitFor();
  await page.waitForTimeout(420);
  await snapshot("setup");
  if (full) {
    await page.getByRole("button", { name: /Найти автоматически|Find automatically/ }).click();
    await page.getByRole("button", { name: /Перейти на главную|Continue to Home/ }).waitFor();
    await page.waitForTimeout(320);
    await snapshot("setup-found");
    await page.getByRole("button", { name: /Перейти на главную|Continue to Home/ }).click();
    await page.getByRole("button", { name: /Открыть сборку|Open build/ }).waitFor();
    await page.waitForTimeout(420);
    await snapshot("home");

    await page.getByRole("button", { name: /Каталог|Discover/ }).click();
    await page.locator(".minify-heading").waitFor();
    await page.waitForTimeout(360);
    await snapshot("catalog-top");
    await page.locator(".catalog-hub").evaluate((element) => element.scrollTo({ top: 520 }));
    await page.waitForTimeout(460);
    await snapshot("catalog-compact");
    await page.getByRole("button", { name: /Гардероб|Wardrobe/ }).click();
    await page.locator(".catalog-heading").waitFor();
    await page.waitForTimeout(360);
    await snapshot("wardrobe");

    await page.locator(".app-rail").hover();
    await page.getByRole("button", { name: /Сборка|Build/ }).click();
    await page.waitForTimeout(300);
    await snapshot("build-review");
    await page.getByRole("button", { name: /Проверить тестовый план|Inspect fixture plan/ }).click();
    await page.getByRole("heading", { name: /Найден конфликт|Fixture conflict detected/ }).waitFor();
    await snapshot("build-conflict");
    await page.getByRole("button", { name: /Оставить Violet|Keep Violet/ }).click();
    await page.waitForTimeout(260);
    await snapshot("build-ready");
    await page.getByRole("button", { name: /Запустить preview-сборку|Start preview build/ }).click();
    await page.waitForTimeout(900);
    await snapshot("build-progress");
    await page.getByRole("button", { name: /Смоделировать ошибку|Simulate an error/ }).click();
    await page.waitForTimeout(300);
    await snapshot("build-recovery");
    await page.getByRole("button", { name: /Восстановить staging|Restore staging/ }).click();
    await page.getByRole("heading", { name: /Временное состояние очищено|Temporary state cleared/ }).waitFor();
    await page.waitForTimeout(240);
    await snapshot("build-restored");
    await page.getByRole("button", { name: /Вернуться к плану|Return to plan/ }).click();
    await page.getByRole("button", { name: /Проверить тестовый план|Inspect fixture plan/ }).click();
    await page.getByRole("button", { name: /Оставить Violet|Keep Violet/ }).click();
    await page.getByRole("button", { name: /Запустить preview-сборку|Start preview build/ }).click();
    await page.locator(".success-result").waitFor({ timeout: 7000 });
    await page.waitForTimeout(300);
    await snapshot("build-success");

    await page.getByRole("button", { name: /Библиотека|Library/ }).click();
    await page.waitForTimeout(300);
    await snapshot("library");
    await page.getByRole("tab", { name: /Мастерская BetterFy|BetterFy Workshop/ }).click();
    await page.waitForTimeout(280);
    await snapshot("workshop");
    await page.getByRole("button", { name: /Настройки|Settings/ }).click();
    await page.waitForTimeout(520);
    await snapshot("settings");
    await page.getByRole("button", { name: /Закрыть|Close/ }).click();
    await page.locator(".side-panel-layer").waitFor({ state: "detached" });
    await page.getByRole("button", { name: /Профиль|Profile/ }).click();
    await page.waitForTimeout(520);
    await snapshot("profile");
  }
  const theme = await page.evaluate(() => document.documentElement.dataset.theme);
  await page.close();
  return theme;
};

const desktop = await capture({ width: 1440, height: 900 }, "1440");
const fullscreen = await capture({ width: 1920, height: 1080 }, "1920");
const minimum = await capture({ width: 980, height: 660 }, "980");
const missingGame = await captureMissingGame();
const reducedMotion = await auditReducedMotion();
const lightEntryTheme = await captureLightEntry({ width: 980, height: 660 }, "980");
const lightFullTheme = await captureLightEntry({ width: 1440, height: 900 }, "1440", true);
if (!reducedMotion.reduced || reducedMotion.activeAnimations > 0) {
  errors.push(`Reduced motion audit failed: ${JSON.stringify(reducedMotion)}`);
}
if (lightEntryTheme !== "light") errors.push("Light theme did not persist through entry flow");
if (lightFullTheme !== "light") errors.push("Light theme did not persist through full flow");
const uniqueContrastIssues = [...new Map(
  contrastIssues.map((issue) => [
    `${issue.screen}|${issue.selector}|${issue.text}|${issue.ratio}`,
    issue,
  ]),
).values()];
if (layoutIssues.length > 0) {
  errors.push(`Layout audit found ${layoutIssues.length} issue(s)`);
}
if (uniqueContrastIssues.length > 0) {
  errors.push(`Contrast audit found ${uniqueContrastIssues.length} issue(s)`);
}
console.log(JSON.stringify({
  errors,
  layoutIssues,
  contrastIssues: uniqueContrastIssues,
  contrastIssueCount: uniqueContrastIssues.length,
  desktop,
  fullscreen,
  minimum,
  missingGame,
  reducedMotion,
  lightEntryTheme,
  lightFullTheme,
}, null, 2));
await browser.close();
if (errors.length > 0) process.exitCode = 1;
