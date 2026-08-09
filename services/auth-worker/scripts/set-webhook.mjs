const token = process.env.TELEGRAM_BOT_TOKEN;
const secret = process.env.TELEGRAM_WEBHOOK_SECRET;
const workerUrl = process.env.BETTERFY_AUTH_WORKER_URL;

if (!token || !secret || !workerUrl) {
  console.error("Set TELEGRAM_BOT_TOKEN, TELEGRAM_WEBHOOK_SECRET, and BETTERFY_AUTH_WORKER_URL.");
  process.exit(1);
}

const endpoint = new URL(`/bot${token}/setWebhook`, "https://api.telegram.org");
const response = await fetch(endpoint, {
  method: "POST",
  headers: { "content-type": "application/json" },
  body: JSON.stringify({
    url: new URL("/v1/telegram/webhook", workerUrl).href,
    secret_token: secret,
    allowed_updates: ["message", "callback_query", "pre_checkout_query"],
    drop_pending_updates: true,
  }),
});

const result = await response.json();
if (!response.ok || !result.ok) {
  console.error("Telegram rejected the webhook configuration.");
  process.exit(1);
}
console.log("BetterFy Telegram webhook configured.");

async function setCommands(commands, languageCode) {
  const commandEndpoint = new URL(`/bot${token}/setMyCommands`, "https://api.telegram.org");
  const commandResponse = await fetch(commandEndpoint, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ commands, ...(languageCode ? { language_code: languageCode } : {}) }),
  });
  const commandResult = await commandResponse.json();
  if (!commandResponse.ok || !commandResult.ok) {
    console.error("Telegram rejected the command menu configuration.");
    process.exit(1);
  }
}

await setCommands([
  { command: "start", description: "Open BetterFy" },
  { command: "code", description: "Get a one-time sign-in code" },
  { command: "subscribe", description: "Open a Stars subscription" },
  { command: "subscription", description: "View or stop your subscription" },
  { command: "paysupport", description: "Payment support" },
  { command: "help", description: "How sign-in works" },
  { command: "privacy", description: "What the bot stores" },
]);
await setCommands([
  { command: "start", description: "Открыть BetterFy" },
  { command: "code", description: "Получить одноразовый код" },
  { command: "subscribe", description: "Оформить подписку за Stars" },
  { command: "subscription", description: "Статус и продление подписки" },
  { command: "paysupport", description: "Помощь с оплатой" },
  { command: "help", description: "Как работает вход" },
  { command: "privacy", description: "Что хранит бот" },
], "ru");
console.log("BetterFy Telegram command menus configured.");
