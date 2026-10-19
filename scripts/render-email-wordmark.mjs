// Renders the BetterFy wordmark for emails as a PNG. Mail clients do not load
// web fonts reliably (Gmail and Proton ignore them), so the brand's two faces,
// Days One and Mrs Sheppards, only reach an inbox as a picture. The background
// is the email's own (#0d0d12) and is baked in, so a client that forces a
// light theme still shows the light "Better" on dark.
//
// Usage: node scripts/render-email-wordmark.mjs
// Output: website/public/bot/email-wordmark.png (served by the auth Worker).

import { chromium } from "playwright";
import { readFile } from "node:fs/promises";

const root = new URL("..", import.meta.url);
const font = async (name) =>
  (await readFile(new URL(`src/assets/fonts/${name}`, root))).toString("base64");

// The same sizes and offsets as the app's .wordmark rules (studio.css), at
// the email's 26 px "Better".
const html = `<!doctype html><html><head><style>
@font-face{font-family:"Days One BetterFy";src:url(data:font/woff2;base64,${await font("days-one-latin.woff2")}) format("woff2")}
@font-face{font-family:"Mrs Sheppards BetterFy";src:url(data:font/woff2;base64,${await font("mrs-sheppards-latin.woff2")}) format("woff2")}
html,body{margin:0;background:#0d0d12}
.mark{display:inline-flex;align-items:center;padding:14px 18px 16px 4px;white-space:nowrap}
.better{font:400 26px/1 "Days One BetterFy";letter-spacing:-.065em;color:#f7f5fb}
.fy{display:inline-block;margin-left:1px;color:#cf4bc9;font:400 40px/.72 "Mrs Sheppards BetterFy";transform:translateY(3px) rotate(-4deg);transform-origin:left center}
</style></head><body><span class="mark"><span class="better">Better</span><span class="fy">Fy</span></span></body></html>`;

const browser = await chromium.launch();
try {
  const page = await browser.newPage({ deviceScaleFactor: 3 });
  await page.setContent(html);
  await page.evaluate(() => document.fonts.ready);
  const mark = page.locator(".mark");
  const box = await mark.boundingBox();
  await mark.screenshot({ path: new URL("website/public/bot/email-wordmark.png", root).pathname });
  console.log(`email-wordmark.png: ${Math.round(box.width)}×${Math.round(box.height)} CSS px at 3x`);
} finally {
  await browser.close();
}
