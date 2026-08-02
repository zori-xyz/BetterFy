import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { chromium } from "playwright";

const root = resolve(import.meta.dirname, "..");
const daysOne = await readFile(resolve(root, "src/assets/fonts/days-one-latin.woff2"));
const mrsSheppards = await readFile(resolve(root, "src/assets/fonts/mrs-sheppards-latin.woff2"));
const output = resolve(root, "src-tauri/icons/icon-source.png");

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({
  viewport: { width: 1024, height: 1024 },
  deviceScaleFactor: 1,
});

await page.setContent(`
  <style>
    @font-face {
      font-family: "Days One BetterFy";
      src: url(data:font/woff2;base64,${daysOne.toString("base64")}) format("woff2");
    }
    @font-face {
      font-family: "Mrs Sheppards BetterFy";
      src: url(data:font/woff2;base64,${mrsSheppards.toString("base64")}) format("woff2");
    }
    * { box-sizing: border-box; }
    html, body {
      width: 1024px;
      height: 1024px;
      margin: 0;
      overflow: hidden;
      background: transparent;
    }
    .icon {
      position: relative;
      display: grid;
      place-items: center;
      width: 1024px;
      height: 1024px;
      overflow: hidden;
      border: 4px solid rgba(255,255,255,.08);
      border-radius: 220px;
      background:
        radial-gradient(circle at 72% 52%, rgba(168,77,255,.16), transparent 29%),
        #050507;
    }
    .wordmark {
      display: flex;
      align-items: center;
      transform: translateY(-5px);
      white-space: nowrap;
    }
    .better {
      color: #f7f5fb;
      font: 400 134px/1 "Days One BetterFy", sans-serif;
      letter-spacing: -7px;
    }
    .fy {
      margin-left: -5px;
      color: #c84ff4;
      font: 400 207px/.7 "Mrs Sheppards BetterFy", cursive;
      transform: translateY(21px);
      text-shadow: 0 0 42px rgba(168,77,255,.23);
    }
  </style>
  <main class="icon" aria-label="BetterFy">
    <div class="wordmark"><span class="better">Better</span><span class="fy">Fy</span></div>
  </main>
`);

await page.evaluate(() => document.fonts.ready);
await page.locator(".icon").screenshot({ path: output, omitBackground: true });
await browser.close();

console.log(output);
