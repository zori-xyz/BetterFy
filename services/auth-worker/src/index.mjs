import {
  CODE_TTL_SECONDS,
  SESSION_TTL_SECONDS,
  chooseLanguage,
  constantTimeEqual,
  formatCode,
  generateCode,
  generateSessionToken,
  keyedHash,
  normalizeCode,
} from "./security.mjs";
import { COPY } from "./copy.mjs";
import {
  ACCESS_PLANS,
  PREMIUM_ENTITLEMENT,
  invoicePayload,
  isEntitlementActive,
  paymentExpiry,
  planById,
  planBySku,
  priceForPlan,
  validateCheckout,
  validateSuccessfulPayment,
} from "./payments.mjs";

const TELEGRAM_HEADER = "X-Telegram-Bot-Api-Secret-Token";
const ISSUE_LIMIT = 3;
const ISSUE_WINDOW_SECONDS = 10 * 60;
const VERIFY_LIMIT = 10;
const VERIFY_WINDOW_SECONDS = 10 * 60;
const MAX_BODY_BYTES = 8 * 1024;
const AVATAR_REFRESH_SECONDS = 24 * 60 * 60;

function matchesConfiguredSecret(value, expected) {
  return typeof expected === "string" && expected.length >= 32 && constantTimeEqual(value, expected);
}

function json(body, status = 200, headers = {}) {
  return new Response(JSON.stringify(body), {
    status,
    headers: {
      "content-type": "application/json; charset=utf-8",
      "cache-control": "no-store",
      "x-content-type-options": "nosniff",
      "referrer-policy": "no-referrer",
      ...headers,
    },
  });
}

function allowedOrigin(request, env) {
  const origin = request.headers.get("Origin");
  if (!origin) return null;
  const allowed = String(env.ALLOWED_ORIGINS ?? "")
    .split(",")
    .map((entry) => entry.trim())
    .filter(Boolean);
  return allowed.includes(origin) ? origin : false;
}

function corsHeaders(origin) {
  return origin
    ? {
        "access-control-allow-origin": origin,
        "access-control-allow-methods": "GET, POST, OPTIONS",
        "access-control-allow-headers": "content-type, authorization",
        "access-control-max-age": "600",
        vary: "Origin",
      }
    : {};
}

async function readJson(request) {
  const length = Number(request.headers.get("content-length") ?? 0);
  if (length > MAX_BODY_BYTES) throw new Error("body_too_large");
  const text = await request.text();
  if (text.length > MAX_BODY_BYTES) throw new Error("body_too_large");
  return JSON.parse(text);
}

async function telegram(env, method, payload) {
  const response = await fetch(`https://api.telegram.org/bot${env.TELEGRAM_BOT_TOKEN}/${method}`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(payload),
  });
  if (!response.ok) throw new Error(`telegram_${method}_failed`);
  const result = await response.json();
  if (!result.ok) throw new Error(`telegram_${method}_rejected`);
  return result.result;
}

async function refreshAvatar(env, from, now) {
  const existing = await env.AUTH_DB.prepare(
    "SELECT avatar_checked_at FROM betterfy_users WHERE telegram_user_id = ?",
  ).bind(String(from.id)).first();
  if (Number(existing?.avatar_checked_at ?? 0) > now - AVATAR_REFRESH_SECONDS) return;
  try {
    const photos = await telegram(env, "getUserProfilePhotos", { user_id: from.id, limit: 1 });
    const sizes = Array.isArray(photos?.photos?.[0]) ? photos.photos[0] : [];
    const fileId = sizes.at(-1)?.file_id;
    await env.AUTH_DB.prepare(
      "UPDATE betterfy_users SET avatar_file_id = ?, avatar_checked_at = ? WHERE telegram_user_id = ?",
    ).bind(typeof fileId === "string" ? fileId : null, now, String(from.id)).run();
  } catch {
    // Avatar is optional. Authentication and payments must keep working when
    // Telegram profile photos are private or temporarily unavailable.
  }
}

async function issueSession(env, userId, now) {
  const token = generateSessionToken();
  const hash = await keyedHash(`session:${token}`, env.AUTH_CODE_PEPPER);
  await env.AUTH_DB.prepare(
    `INSERT INTO auth_sessions
      (session_hash, user_id, created_at, expires_at, last_used_at)
     VALUES (?, ?, ?, ?, ?)`,
  ).bind(hash, userId, now, now + SESSION_TTL_SECONDS, now).run();
  return token;
}

function bearerToken(request) {
  const value = request.headers.get("Authorization") ?? "";
  const match = /^Bearer ([A-Za-z0-9_-]{43})$/.exec(value);
  return match?.[1] ?? null;
}

async function authenticatedUser(request, env, now) {
  const token = bearerToken(request);
  if (!token) return null;
  const hash = await keyedHash(`session:${token}`, env.AUTH_CODE_PEPPER);
  const user = await env.AUTH_DB.prepare(
    `SELECT u.user_id, u.telegram_user_id, u.display_name, u.username, u.language,
            u.avatar_file_id, s.expires_at
     FROM auth_sessions s
     JOIN betterfy_users u ON u.user_id = s.user_id
     WHERE s.session_hash = ? AND s.revoked_at IS NULL AND s.expires_at > ?`,
  ).bind(hash, now).first();
  if (!user) return null;
  await env.AUTH_DB.prepare(
    "UPDATE auth_sessions SET last_used_at = ? WHERE session_hash = ?",
  ).bind(now, hash).run();
  return user;
}

async function profileAvatar(request, env, origin) {
  const headers = corsHeaders(origin);
  const user = await authenticatedUser(request, env, Math.floor(Date.now() / 1000));
  if (!user) return json({ error: "unauthorized" }, 401, headers);
  if (!user.avatar_file_id) return json({ error: "avatar_missing" }, 404, headers);
  const file = await telegram(env, "getFile", { file_id: user.avatar_file_id });
  const path = file?.file_path;
  if (typeof path !== "string" || path.length > 240 || path.includes("..") || !/^[A-Za-z0-9_./-]+$/.test(path)) {
    return json({ error: "avatar_unavailable" }, 404, headers);
  }
  const response = await fetch(`https://api.telegram.org/file/bot${env.TELEGRAM_BOT_TOKEN}/${path}`);
  if (!response.ok || !response.body) return json({ error: "avatar_unavailable" }, 404, headers);
  const declaredSize = Number(response.headers.get("content-length") ?? 0);
  if (declaredSize > 5 * 1024 * 1024) return json({ error: "avatar_too_large" }, 413, headers);
  const contentType = response.headers.get("content-type") ?? "image/jpeg";
  if (!contentType.startsWith("image/")) return json({ error: "avatar_unavailable" }, 404, headers);
  const bytes = await response.arrayBuffer();
  if (bytes.byteLength > 5 * 1024 * 1024) return json({ error: "avatar_too_large" }, 413, headers);
  return new Response(bytes, {
    headers: {
      ...headers,
      "content-type": contentType,
      "cache-control": "private, max-age=300",
      "x-content-type-options": "nosniff",
      "referrer-policy": "no-referrer",
    },
  });
}

async function sessionProfile(request, env, origin) {
  const headers = corsHeaders(origin);
  const now = Math.floor(Date.now() / 1000);
  const user = await authenticatedUser(request, env, now);
  if (!user) return json({ error: "unauthorized" }, 401, headers);
  const entitlement = await env.AUTH_DB.prepare(
    "SELECT active_until FROM entitlements WHERE user_id = ? AND entitlement_key = ?",
  ).bind(user.user_id, PREMIUM_ENTITLEMENT).first();
  return json({
    userId: user.user_id,
    displayName: user.display_name,
    username: user.username ?? undefined,
    accessTier: isEntitlementActive(entitlement, now) ? "premium" : "early-access",
    avatarAvailable: Boolean(user.avatar_file_id),
    sessionExpiresAt: user.expires_at,
  }, 200, headers);
}

async function revokeSession(request, env, origin) {
  const headers = corsHeaders(origin);
  const token = bearerToken(request);
  if (!token) return json({ error: "unauthorized" }, 401, headers);
  const hash = await keyedHash(`session:${token}`, env.AUTH_CODE_PEPPER);
  const result = await env.AUTH_DB.prepare(
    "UPDATE auth_sessions SET revoked_at = ? WHERE session_hash = ? AND revoked_at IS NULL",
  ).bind(Math.floor(Date.now() / 1000), hash).run();
  return json({ ok: Number(result.meta?.changes ?? 0) === 1 }, 200, headers);
}

function allowedReleaseUrl(value) {
  try {
    const url = new URL(value);
    return url.protocol === "https:"
      && url.hostname === "github.com"
      && url.pathname.startsWith("/zori-xyz/BetterFy/releases/download/")
      ? url.href
      : null;
  } catch {
    return null;
  }
}

async function latestRelease(request, env, origin) {
  const headers = corsHeaders(origin);
  const user = await authenticatedUser(request, env, Math.floor(Date.now() / 1000));
  if (!user) return json({ error: "unauthorized" }, 401, headers);
  const response = await fetch("https://api.github.com/repos/zori-xyz/BetterFy/releases/latest", {
    headers: { accept: "application/vnd.github+json", "user-agent": "BetterFy-Auth-Worker" },
  });
  if (!response.ok) return json({ error: "release_unavailable" }, 503, headers);
  const release = await response.json();
  const asset = Array.isArray(release.assets)
    ? release.assets.find((candidate) => typeof candidate?.name === "string" && candidate.name.toLowerCase().endsWith(".exe"))
    : null;
  const downloadUrl = allowedReleaseUrl(asset?.browser_download_url);
  if (!downloadUrl) return json({ error: "installer_unavailable" }, 404, headers);
  return json({
    version: typeof release.tag_name === "string" ? release.tag_name : "BetterFy",
    publishedAt: typeof release.published_at === "string" ? release.published_at : null,
    downloadUrl,
  }, 200, headers);
}

async function configureTelegram(env) {
  const webhookUrl = new URL("/v1/telegram/webhook", env.PUBLIC_WORKER_URL);
  if (webhookUrl.protocol !== "https:") throw new Error("PUBLIC_WORKER_URL must use HTTPS");
  await telegram(env, "setWebhook", {
    url: webhookUrl.href,
    secret_token: env.TELEGRAM_WEBHOOK_SECRET,
    allowed_updates: ["message", "callback_query", "pre_checkout_query"],
    drop_pending_updates: true,
  });
  await telegram(env, "setMyCommands", {
    commands: [
      { command: "start", description: "Open BetterFy" },
      { command: "code", description: "Get a one-time sign-in code" },
      { command: "subscribe", description: "Open a Stars subscription" },
      { command: "subscription", description: "View or stop your subscription" },
      { command: "paysupport", description: "Payment support" },
      { command: "help", description: "How sign-in works" },
      { command: "privacy", description: "What the bot stores" },
      { command: "terms", description: "Purchase terms" },
    ],
  });
  await telegram(env, "setMyCommands", {
    language_code: "ru",
    commands: [
      { command: "start", description: "Открыть BetterFy" },
      { command: "code", description: "Получить одноразовый код" },
      { command: "subscribe", description: "Оформить подписку за Stars" },
      { command: "subscription", description: "Статус и продление подписки" },
      { command: "paysupport", description: "Помощь с оплатой" },
      { command: "help", description: "Как работает вход" },
      { command: "privacy", description: "Что хранит бот" },
      { command: "terms", description: "Условия покупки" },
    ],
  });
}

function websiteUrl() {
  return "https://zori-xyz.github.io/BetterFy/";
}

function cardUrl(env, name) {
  const base = new URL(env.BOT_ASSET_BASE_URL);
  if (base.protocol !== "https:") throw new Error("BOT_ASSET_BASE_URL must use HTTPS");
  return `${base.href.replace(/\/$/, "")}/${name}`;
}

function welcomeKeyboard(language) {
  const copy = COPY[language];
  return {
    inline_keyboard: [
      [{ text: copy.issue, callback_data: "issue_code" }],
      [{ text: `⭐ ${copy.subscribe}`, callback_data: "plans" }],
      [
        { text: copy.website, url: websiteUrl() },
        { text: copy.language, callback_data: language === "ru" ? "lang_en" : "lang_ru" },
      ],
    ],
  };
}

function plansKeyboard(language) {
  const copy = COPY[language];
  return {
    inline_keyboard: ACCESS_PLANS.map((plan) => [
      { text: copy.plans[plan.id].button, callback_data: `buy_${plan.id}` },
    ]),
  };
}

async function sendPlans(env, chatId, language) {
  await telegram(env, "sendMessage", {
    chat_id: chatId,
    text: COPY[language].choosePlan,
    reply_markup: plansKeyboard(language),
  });
}

function formatExpiry(timestamp, language) {
  return new Intl.DateTimeFormat(language === "ru" ? "ru-RU" : "en-GB", {
    day: "2-digit",
    month: "long",
    year: "numeric",
    timeZone: "UTC",
  }).format(new Date(timestamp * 1000));
}

async function upsertUser(env, from, language, now) {
  const displayName = [from.first_name, from.last_name].filter(Boolean).join(" ").trim() || "BetterFy player";
  const existing = await env.AUTH_DB.prepare(
    "SELECT user_id FROM betterfy_users WHERE telegram_user_id = ?",
  ).bind(String(from.id)).first();
  const userId = existing?.user_id ?? crypto.randomUUID();
  await env.AUTH_DB.prepare(
    `INSERT INTO betterfy_users
      (user_id, telegram_user_id, display_name, username, language, created_at, updated_at)
     VALUES (?, ?, ?, ?, ?, ?, ?)
     ON CONFLICT(telegram_user_id) DO UPDATE SET
       display_name = excluded.display_name,
       username = excluded.username,
       language = excluded.language,
       updated_at = excluded.updated_at`,
  ).bind(userId, String(from.id), displayName, from.username ?? null, language, now, now).run();
  await refreshAvatar(env, from, now);
  return userId;
}

async function getLanguage(env, from) {
  const stored = await env.AUTH_DB.prepare(
    "SELECT language FROM betterfy_users WHERE telegram_user_id = ?",
  ).bind(String(from.id)).first();
  return stored?.language === "ru" || stored?.language === "en"
    ? stored.language
    : chooseLanguage(from.language_code);
}

async function sendWelcome(env, chatId, language) {
  await telegram(env, "sendPhoto", {
    chat_id: chatId,
    photo: cardUrl(env, "message-master.png"),
    caption: COPY[language].welcome,
    reply_markup: welcomeKeyboard(language),
  });
}

async function issueCode(env, chatId, from, language, now) {
  const userId = await upsertUser(env, from, language, now);
  const recent = await env.AUTH_DB.prepare(
    "SELECT COUNT(*) AS count FROM auth_codes WHERE user_id = ? AND created_at >= ?",
  ).bind(userId, now - ISSUE_WINDOW_SECONDS).first();
  if (Number(recent?.count ?? 0) >= ISSUE_LIMIT) {
    await telegram(env, "sendMessage", { chat_id: chatId, text: COPY[language].rateLimited });
    return;
  }

  let code;
  let hash;
  let inserted = false;
  for (let attempt = 0; attempt < 6 && !inserted; attempt += 1) {
    code = generateCode();
    hash = await keyedHash(`code:${code}`, env.AUTH_CODE_PEPPER);
    const result = await env.AUTH_DB.prepare(
      "INSERT OR IGNORE INTO auth_codes (code_hash, user_id, created_at, expires_at) VALUES (?, ?, ?, ?)",
    ).bind(hash, userId, now, now + CODE_TTL_SECONDS).run();
    inserted = Number(result.meta?.changes ?? 0) === 1;
  }
  if (!inserted) throw new Error("code_generation_collision");

  await telegram(env, "sendPhoto", {
    chat_id: chatId,
    photo: cardUrl(env, language === "ru" ? "code-ru.png" : "code-en.png"),
    parse_mode: "HTML",
    caption: `<b>${COPY[language].codeTitle}</b>\n\n<code>${formatCode(code)}</code>\n\n${COPY[language].codeBody}`,
    protect_content: true,
  });
}

async function subscriptionRecord(env, userId) {
  return env.AUTH_DB.prepare(
    `SELECT e.active_until, e.source_charge_id, p.canceled_at, o.sku
     FROM entitlements e
     LEFT JOIN star_payment_events p ON p.telegram_charge_id = e.source_charge_id
     LEFT JOIN payment_orders o ON o.order_id = p.order_id
     WHERE e.user_id = ? AND e.entitlement_key = ?`,
  ).bind(userId, PREMIUM_ENTITLEMENT).first();
}

async function activeRecurringSubscription(env, userId, now) {
  const recurringPlan = planById("30d");
  return env.AUTH_DB.prepare(
    `SELECT p.telegram_charge_id, p.canceled_at
     FROM star_payment_events p
     JOIN payment_orders o ON o.order_id = p.order_id
     WHERE p.user_id = ? AND o.sku = ? AND p.refunded_at IS NULL AND p.expires_at > ?
     ORDER BY p.paid_at DESC
     LIMIT 1`,
  ).bind(userId, recurringPlan.sku, now).first();
}

async function sendSubscriptionStatus(env, chatId, from, language, now) {
  const userId = await upsertUser(env, from, language, now);
  const subscription = await subscriptionRecord(env, userId);
  if (!isEntitlementActive(subscription, now)) {
    await telegram(env, "sendMessage", {
      chat_id: chatId,
      text: COPY[language].subscriptionInactive,
      reply_markup: { inline_keyboard: [[{ text: `⭐ ${COPY[language].subscribe}`, callback_data: "plans" }]] },
    });
    return false;
  }

  const recurring = await activeRecurringSubscription(env, userId, now);
  const buttons = recurring && recurring.canceled_at == null
    ? [[{ text: COPY[language].subscriptionCancel, callback_data: "cancel_subscription" }]]
    : [];
  await telegram(env, "sendMessage", {
    chat_id: chatId,
    text: `${COPY[language].subscriptionActive} ${formatExpiry(subscription.active_until, language)}.`,
    reply_markup: { inline_keyboard: buttons },
  });
  return true;
}

async function sendAccessInvoice(env, chatId, from, language, now, planId) {
  const plan = planById(planId);
  if (!plan) return;
  const userId = await upsertUser(env, from, language, now);
  const amount = priceForPlan(env, plan);
  if (!amount) {
    await telegram(env, "sendMessage", { chat_id: chatId, text: COPY[language].subscriptionUnavailable });
    return;
  }

  const orderId = crypto.randomUUID();
  const payload = invoicePayload(orderId);
  await env.AUTH_DB.prepare(
    `INSERT INTO payment_orders
      (order_id, user_id, sku, invoice_payload, currency, amount, status, created_at, updated_at)
     VALUES (?, ?, ?, ?, 'XTR', ?, 'created', ?, ?)`,
  ).bind(orderId, userId, plan.sku, payload, amount, now, now).run();

  const invoice = {
    chat_id: chatId,
    title: COPY[language].plans[plan.id].title,
    description: COPY[language].plans[plan.id].description,
    payload,
    currency: "XTR",
    prices: [{ label: COPY[language].plans[plan.id].title, amount }],
    start_parameter: `premium_${plan.id}_${orderId}`,
    photo_url: cardUrl(env, "message-master.png"),
    protect_content: true,
  };
  if (plan.recurring) invoice.subscription_period = plan.durationSeconds;
  await telegram(env, "sendInvoice", invoice);
}

async function handlePreCheckout(env, query, now) {
  const order = await env.AUTH_DB.prepare(
    `SELECT o.*, u.telegram_user_id
     FROM payment_orders o
     JOIN betterfy_users u ON u.user_id = o.user_id
     WHERE o.invoice_payload = ?`,
  ).bind(query.invoice_payload).first();
  const accepted = validateCheckout(query, order, order?.telegram_user_id);
  await telegram(env, "answerPreCheckoutQuery", {
    pre_checkout_query_id: query.id,
    ok: accepted,
    ...(!accepted ? { error_message: COPY[chooseLanguage(query.from?.language_code)].paymentRejected } : {}),
  });
  if (accepted) {
    await env.AUTH_DB.prepare(
      "UPDATE payment_orders SET status = 'precheckout', updated_at = ? WHERE order_id = ? AND status = 'created'",
    ).bind(now, order.order_id).run();
  }
}

async function handleSuccessfulPayment(env, message, language, now) {
  const payment = message.successful_payment;
  const order = await env.AUTH_DB.prepare(
    `SELECT o.*, u.telegram_user_id
     FROM payment_orders o
     JOIN betterfy_users u ON u.user_id = o.user_id
     WHERE o.invoice_payload = ?`,
  ).bind(payment.invoice_payload).first();
  if (!validateSuccessfulPayment(payment, order) || String(message.from.id) !== String(order.telegram_user_id)) {
    throw new Error("payment_validation_failed");
  }

  const plan = planBySku(order.sku);
  if (!plan) throw new Error("payment_plan_missing");
  const duplicate = await env.AUTH_DB.prepare(
    "SELECT telegram_charge_id FROM star_payment_events WHERE telegram_charge_id = ?",
  ).bind(payment.telegram_payment_charge_id).first();
  if (duplicate) return;
  const currentEntitlement = await env.AUTH_DB.prepare(
    "SELECT active_until FROM entitlements WHERE user_id = ? AND entitlement_key = ?",
  ).bind(order.user_id, PREMIUM_ENTITLEMENT).first();

  const expiresAt = paymentExpiry(payment, now, plan, currentEntitlement?.active_until);
  const results = await env.AUTH_DB.batch([
    env.AUTH_DB.prepare(
      `INSERT OR IGNORE INTO star_payment_events
        (telegram_charge_id, order_id, user_id, amount, currency, paid_at, expires_at,
         is_recurring, is_first_recurring)
       VALUES (?, ?, ?, ?, 'XTR', ?, ?, ?, ?)`,
    ).bind(
      payment.telegram_payment_charge_id,
      order.order_id,
      order.user_id,
      payment.total_amount,
      now,
      expiresAt,
      payment.is_recurring ? 1 : 0,
      payment.is_first_recurring ? 1 : 0,
    ),
    env.AUTH_DB.prepare(
      "UPDATE payment_orders SET status = 'paid', updated_at = ? WHERE order_id = ?",
    ).bind(now, order.order_id),
    env.AUTH_DB.prepare(
      `INSERT INTO entitlements
        (user_id, entitlement_key, active_until, source, source_charge_id, updated_at)
       VALUES (?, ?, ?, 'telegram_stars', ?, ?)
       ON CONFLICT(user_id, entitlement_key) DO UPDATE SET
         active_until = MAX(entitlements.active_until, excluded.active_until),
         source_charge_id = CASE
           WHEN excluded.active_until >= entitlements.active_until THEN excluded.source_charge_id
           ELSE entitlements.source_charge_id
         END,
         updated_at = excluded.updated_at`,
    ).bind(order.user_id, PREMIUM_ENTITLEMENT, expiresAt, payment.telegram_payment_charge_id, now),
  ]);

  if (Number(results[0]?.meta?.changes ?? 0) === 1) {
    await telegram(env, "sendMessage", {
      chat_id: message.chat.id,
      text: `${COPY[language].subscriptionPaid}\n${COPY[language].subscriptionActive} ${formatExpiry(expiresAt, language)}.`,
    });
  }
}

async function cancelSubscription(env, chatId, from, language, now) {
  const userId = await upsertUser(env, from, language, now);
  const subscription = await activeRecurringSubscription(env, userId, now);
  if (!subscription?.telegram_charge_id) {
    await telegram(env, "sendMessage", { chat_id: chatId, text: COPY[language].subscriptionInactive });
    return;
  }
  await telegram(env, "editUserStarSubscription", {
    user_id: from.id,
    telegram_payment_charge_id: subscription.telegram_charge_id,
    is_canceled: true,
  });
  await env.AUTH_DB.prepare(
    "UPDATE star_payment_events SET canceled_at = COALESCE(canceled_at, ?) WHERE telegram_charge_id = ?",
  ).bind(now, subscription.telegram_charge_id).run();
  await telegram(env, "sendMessage", { chat_id: chatId, text: COPY[language].subscriptionCanceled });
}

async function handleRefundedPayment(env, message, language, now) {
  const payment = message.refunded_payment;
  const event = await env.AUTH_DB.prepare(
    "SELECT user_id, order_id FROM star_payment_events WHERE telegram_charge_id = ?",
  ).bind(payment.telegram_payment_charge_id).first();
  if (!event) return;
  await env.AUTH_DB.prepare(
    "UPDATE star_payment_events SET refunded_at = COALESCE(refunded_at, ?) WHERE telegram_charge_id = ?",
  ).bind(now, payment.telegram_payment_charge_id).run();
  const remaining = await env.AUTH_DB.prepare(
    `SELECT expires_at AS active_until, telegram_charge_id
     FROM star_payment_events
     WHERE user_id = ? AND refunded_at IS NULL AND expires_at > ?
     ORDER BY expires_at DESC
     LIMIT 1`,
  ).bind(event.user_id, now).first();
  const activeUntil = Number(remaining?.active_until ?? now);
  await env.AUTH_DB.batch([
    env.AUTH_DB.prepare(
      "UPDATE payment_orders SET status = 'refunded', updated_at = ? WHERE order_id = ?",
    ).bind(now, event.order_id),
    env.AUTH_DB.prepare(
      `UPDATE entitlements SET active_until = ?, source_charge_id = ?, updated_at = ?
       WHERE user_id = ? AND entitlement_key = ?`,
    ).bind(activeUntil, remaining?.telegram_charge_id ?? payment.telegram_payment_charge_id, now, event.user_id, PREMIUM_ENTITLEMENT),
  ]);
  await telegram(env, "sendMessage", { chat_id: message.chat.id, text: COPY[language].paymentRefunded });
}

async function handleMessage(env, message, now) {
  if (!message?.from || !message?.chat) return;
  const language = await getLanguage(env, message.from);
  if (message.chat.type !== "private") {
    await telegram(env, "sendMessage", { chat_id: message.chat.id, text: COPY[language].privateOnly });
    return;
  }
  if (message.successful_payment) return handleSuccessfulPayment(env, message, language, now);
  if (message.refunded_payment) return handleRefundedPayment(env, message, language, now);
  const command = String(message.text ?? "").trim().split(/\s+/)[0].split("@")[0].toLowerCase();
  if (command === "/code") return issueCode(env, message.chat.id, message.from, language, now);
  if (command === "/subscribe") return sendPlans(env, message.chat.id, language);
  if (command === "/subscription") return sendSubscriptionStatus(env, message.chat.id, message.from, language, now);
  if (command === "/help") {
    return telegram(env, "sendMessage", { chat_id: message.chat.id, text: COPY[language].help });
  }
  if (command === "/privacy") {
    return telegram(env, "sendMessage", { chat_id: message.chat.id, text: COPY[language].privacy });
  }
  if (command === "/terms") {
    return telegram(env, "sendMessage", { chat_id: message.chat.id, text: COPY[language].terms });
  }
  if (command === "/paysupport") {
    return telegram(env, "sendMessage", { chat_id: message.chat.id, text: COPY[language].paySupport });
  }
  await upsertUser(env, message.from, language, now);
  return sendWelcome(env, message.chat.id, language);
}

async function handleCallback(env, callback, now) {
  if (!callback?.from || !callback?.message?.chat) return;
  await telegram(env, "answerCallbackQuery", { callback_query_id: callback.id });
  const chatId = callback.message.chat.id;
  let language = await getLanguage(env, callback.from);
  if (callback.data === "issue_code") return issueCode(env, chatId, callback.from, language, now);
  if (callback.data === "plans") return sendPlans(env, chatId, language);
  if (typeof callback.data === "string" && callback.data.startsWith("buy_")) {
    return sendAccessInvoice(env, chatId, callback.from, language, now, callback.data.slice(4));
  }
  if (callback.data === "cancel_subscription") return cancelSubscription(env, chatId, callback.from, language, now);
  if (callback.data === "lang_ru" || callback.data === "lang_en") {
    language = callback.data.endsWith("ru") ? "ru" : "en";
    await upsertUser(env, callback.from, language, now);
    await telegram(env, "sendMessage", { chat_id: chatId, text: COPY[language].languageChanged });
    return sendWelcome(env, chatId, language);
  }
}

async function handleWebhook(request, env) {
  if (!matchesConfiguredSecret(request.headers.get(TELEGRAM_HEADER), env.TELEGRAM_WEBHOOK_SECRET)) {
    return json({ error: "not_found" }, 404);
  }
  let update;
  try {
    update = await readJson(request);
  } catch {
    return json({ error: "invalid_update" }, 400);
  }
  const now = Math.floor(Date.now() / 1000);
  if (update.pre_checkout_query) await handlePreCheckout(env, update.pre_checkout_query, now);
  if (update.message) await handleMessage(env, update.message, now);
  if (update.callback_query) await handleCallback(env, update.callback_query, now);
  return json({ ok: true });
}

async function consumeRateLimit(env, request, now) {
  const ip = request.headers.get("CF-Connecting-IP") ?? "unknown";
  const bucket = await keyedHash(`verify:${ip}`, env.AUTH_CODE_PEPPER);
  const resetBefore = now - VERIFY_WINDOW_SECONDS;
  const row = await env.AUTH_DB.prepare(
    `INSERT INTO auth_rate_limits (bucket_hash, window_started_at, request_count)
     VALUES (?, ?, 1)
     ON CONFLICT(bucket_hash) DO UPDATE SET
       request_count = CASE WHEN window_started_at < ? THEN 1 ELSE request_count + 1 END,
       window_started_at = CASE WHEN window_started_at < ? THEN ? ELSE window_started_at END
     RETURNING request_count`,
  ).bind(bucket, now, resetBefore, resetBefore, now).first();
  return Number(row?.request_count ?? VERIFY_LIMIT + 1) <= VERIFY_LIMIT;
}

async function verifyCode(request, env, origin) {
  const headers = corsHeaders(origin);
  const now = Math.floor(Date.now() / 1000);
  if (!(await consumeRateLimit(env, request, now))) {
    return json({ error: "rate_limited" }, 429, { ...headers, "retry-after": "600" });
  }
  let payload;
  try {
    payload = await readJson(request);
  } catch {
    return json({ error: "invalid_request" }, 400, headers);
  }
  const code = normalizeCode(payload?.code);
  if (!code) return json({ error: "invalid_code" }, 400, headers);
  const hash = await keyedHash(`code:${code}`, env.AUTH_CODE_PEPPER);
  const consumed = await env.AUTH_DB.prepare(
    `UPDATE auth_codes SET consumed_at = ?
     WHERE code_hash = ? AND consumed_at IS NULL AND expires_at > ?
     RETURNING user_id`,
  ).bind(now, hash, now).first();
  if (!consumed?.user_id) return json({ error: "code_invalid_or_expired" }, 401, headers);

  const user = await env.AUTH_DB.prepare(
    "SELECT user_id, telegram_user_id, display_name, username, language, avatar_file_id FROM betterfy_users WHERE user_id = ?",
  ).bind(consumed.user_id).first();
  if (!user) return json({ error: "profile_missing" }, 500, headers);
  const entitlement = await env.AUTH_DB.prepare(
    "SELECT active_until FROM entitlements WHERE user_id = ? AND entitlement_key = ?",
  ).bind(user.user_id, PREMIUM_ENTITLEMENT).first();

  const approvedCard = user.language === "ru" ? "approved-ru.png" : "approved-en.png";
  const copy = COPY[user.language === "ru" ? "ru" : "en"];
  const sessionToken = await issueSession(env, user.user_id, now);
  try {
    await telegram(env, "sendPhoto", {
      chat_id: user.telegram_user_id,
      photo: cardUrl(env, approvedCard),
      caption: copy.approved,
      protect_content: true,
    });
  } catch {
    // Approval is already committed. Telegram delivery is best effort only.
  }

  return json({
    userId: user.user_id,
    displayName: user.display_name,
    username: user.username ?? undefined,
    accessTier: isEntitlementActive(entitlement, now) ? "premium" : "early-access",
    sessionToken,
    avatarAvailable: Boolean(user.avatar_file_id),
  }, 200, headers);
}

export async function route(request, env) {
  const url = new URL(request.url);
  if (request.method === "GET" && url.pathname === "/health") {
    return json({ ok: true, service: "betterfy-auth" });
  }
  if (request.method === "POST" && url.pathname === "/v1/telegram/webhook") {
    return handleWebhook(request, env);
  }
  if (request.method === "POST" && url.pathname === "/internal/configure-telegram") {
    if (!matchesConfiguredSecret(request.headers.get("X-BetterFy-Deploy-Secret"), env.DEPLOY_ADMIN_SECRET)) {
      return json({ error: "not_found" }, 404);
    }
    await configureTelegram(env);
    return json({ ok: true, configured: "telegram" });
  }
  if (url.pathname.startsWith("/v1/session/") || url.pathname === "/v1/releases/latest") {
    const origin = allowedOrigin(request, env);
    if (origin === false) return json({ error: "origin_not_allowed" }, 403);
    if (request.method === "OPTIONS") return new Response(null, { status: 204, headers: corsHeaders(origin) });
    if (request.method === "GET" && url.pathname === "/v1/session/profile") return sessionProfile(request, env, origin);
    if (request.method === "GET" && url.pathname === "/v1/session/avatar") return profileAvatar(request, env, origin);
    if (request.method === "POST" && url.pathname === "/v1/session/logout") return revokeSession(request, env, origin);
    if (request.method === "GET" && url.pathname === "/v1/releases/latest") return latestRelease(request, env, origin);
    return json({ error: "method_not_allowed" }, 405, corsHeaders(origin));
  }
  if (url.pathname === "/v1/auth/telegram/code") {
    const origin = allowedOrigin(request, env);
    if (origin === false) return json({ error: "origin_not_allowed" }, 403);
    if (request.method === "OPTIONS") return new Response(null, { status: 204, headers: corsHeaders(origin) });
    if (request.method === "POST") return verifyCode(request, env, origin);
  }
  return json({ error: "not_found" }, 404);
}

export default {
  async fetch(request, env) {
    try {
      return await route(request, env);
    } catch (error) {
      console.error("request_failed", { name: error?.name, message: String(error?.message ?? "unknown") });
      return json({ error: "internal_error" }, 500);
    }
  },
};
