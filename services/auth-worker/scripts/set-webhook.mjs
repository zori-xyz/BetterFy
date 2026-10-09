// Applies the whole Telegram configuration (secret webhook, command menus in
// both languages, first-open descriptions) through the deployed Worker, so the
// lists live in one place: configureTelegram() in src/index.mjs.
const workerUrl = process.env.BETTERFY_AUTH_WORKER_URL;
const secret = process.env.DEPLOY_ADMIN_SECRET;

if (!workerUrl || !secret) {
  console.error("Set BETTERFY_AUTH_WORKER_URL and DEPLOY_ADMIN_SECRET.");
  process.exit(1);
}

const response = await fetch(new URL("/internal/configure-telegram", workerUrl), {
  method: "POST",
  headers: { "X-BetterFy-Deploy-Secret": secret },
});
const result = await response.json().catch(() => ({}));
if (!response.ok || !result.ok) {
  console.error(`The Worker rejected the Telegram configuration (HTTP ${response.status}).`);
  process.exit(1);
}
console.log("BetterFy Telegram bot configured: webhook, commands and descriptions.");
