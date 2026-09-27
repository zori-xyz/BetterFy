import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { chromium } from "playwright";

const root = resolve(import.meta.dirname, "..");
const [symbol, daysOne, mrsSheppards] = await Promise.all([
  readFile(resolve(root, "brand/betterfy-symbol-dark.jpg")),
  readFile(resolve(root, "src/assets/fonts/days-one-latin.woff2")),
  readFile(resolve(root, "src/assets/fonts/mrs-sheppards-latin.woff2")),
]);

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1280, height: 640 } });
await page.setContent(`
  <style>
    @font-face { font-family: DaysOne; src: url(data:font/woff2;base64,${daysOne.toString("base64")}); }
    @font-face { font-family: MrsSheppards; src: url(data:font/woff2;base64,${mrsSheppards.toString("base64")}); }
    * { box-sizing: border-box; }
    html, body { width: 1280px; height: 640px; margin: 0; overflow: hidden; }
    body {
      display: grid;
      grid-template-columns: 430px 1fr;
      align-items: center;
      gap: 74px;
      padding: 72px 88px;
      color: #f7f5fb;
      background:
        radial-gradient(circle at 24% 50%, rgba(118, 55, 255, .16), transparent 33%),
        linear-gradient(135deg, #07070b, #0d0915 72%, #07070b);
    }
    .symbol {
      display: block;
      width: 390px;
      height: 390px;
      border-radius: 88px;
      box-shadow: 0 34px 90px rgba(0,0,0,.54), 0 0 80px rgba(113,57,255,.13);
    }
    .copy { align-self: center; padding-bottom: 4px; }
    .wordmark { display: flex; align-items: center; height: 126px; white-space: nowrap; }
    .better { font: 400 92px/1 DaysOne, sans-serif; letter-spacing: -7px; }
    .fy { margin-left: -3px; color: #c850f2; font: 400 132px/.7 MrsSheppards, cursive; transform: translateY(13px); }
    .rule { width: 100%; height: 1px; margin: 20px 0 26px; background: linear-gradient(90deg, rgba(255,255,255,.22), transparent); }
    .label { margin: 0; color: #b987e4; font: 600 15px/1.4 ui-monospace, monospace; letter-spacing: .2em; }
    .detail { margin: 15px 0 0; color: #85818f; font: 500 20px/1.5 system-ui, sans-serif; }
  </style>
  <img class="symbol" src="data:image/jpeg;base64,${symbol.toString("base64")}" alt="" />
  <main class="copy">
    <div class="wordmark"><span class="better">Better</span><span class="fy">Fy</span></div>
    <div class="rule"></div>
    <p class="label">DOTA 2 · MOD PLATFORM</p>
    <p class="detail">Windows · Early Access</p>
  </main>
`);
await page.evaluate(() => document.fonts.ready);
await page.screenshot({ path: resolve(root, "brand/betterfy-github-social.png") });
await browser.close();

console.log(resolve(root, "brand/betterfy-github-social.png"));
