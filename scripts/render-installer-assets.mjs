import { chromium } from "playwright";
import { execFileSync } from "node:child_process";
import { mkdirSync } from "node:fs";
import { fileURLToPath, pathToFileURL } from "node:url";
import path from "node:path";

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const source = path.join(projectRoot, "scripts", "installer-preview.html");
const assetDir = path.join(projectRoot, "src-tauri", "windows", "installer");
const previewDir = path.join(projectRoot, "artifacts", "installer-preview");

mkdirSync(assetDir, { recursive: true });
mkdirSync(previewDir, { recursive: true });

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1400, height: 980 }, deviceScaleFactor: 1 });

try {
  await page.goto(pathToFileURL(source).href);
  await page.evaluate(() => document.fonts.ready);

  const captures = [
    ["#sidebar-image", path.join(assetDir, "sidebar.png")],
    ["#header-image", path.join(assetDir, "header.png")],
    ["#preview-welcome-ru", path.join(previewDir, "01-welcome-ru.png")],
    ["#preview-progress-ru", path.join(previewDir, "02-progress-ru.png")],
    ["#preview-finish-ru", path.join(previewDir, "03-finish-ru.png")],
    ["#preview-welcome-en", path.join(previewDir, "04-welcome-en.png")],
  ];

  for (const [selector, output] of captures) {
    await page.locator(selector).screenshot({ path: output });
  }
} finally {
  await browser.close();
}

if (process.platform !== "darwin") {
  throw new Error("BMP regeneration currently requires macOS sips; committed BMP files remain portable.");
}

for (const name of ["sidebar", "header"]) {
  execFileSync("sips", ["-s", "format", "bmp", path.join(assetDir, `${name}.png`), "--out", path.join(assetDir, `${name}.bmp`)], {
    stdio: "inherit",
  });
}

console.log(`Installer assets: ${assetDir}`);
console.log(`Installer previews: ${previewDir}`);
