import {
  CODE_TTL_SECONDS,
  DEVICE_CHALLENGE_TTL_SECONDS,
  DESKTOP_SESSION_TTL_SECONDS,
  REFRESH_FAMILY_TTL_SECONDS,
  SESSION_TTL_SECONDS,
  chooseLanguage,
  constantTimeEqual,
  formatCode,
  generateCode,
  generateSessionToken,
  keyedHash,
  normalizeChallengeToken,
  normalizeCode,
  normalizeDeviceId,
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
const CHALLENGE_CREATE_LIMIT = 6;
const CHALLENGE_POLL_SECONDS = 2;
const MAX_BODY_BYTES = 8 * 1024;
const AVATAR_REFRESH_SECONDS = 24 * 60 * 60;
const AVATAR_RETRY_SECONDS = 5 * 60;

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

export function selectTelegramAvatarFileId(profilePhotos) {
  const sizes = Array.isArray(profilePhotos?.photos?.[0]) ? profilePhotos.photos[0] : [];
  const candidates = sizes.filter((photo) => typeof photo?.file_id === "string" && photo.file_id.length > 0);
  if (candidates.length === 0) return null;
  return candidates.reduce((largest, candidate) => {
    const largestScore = Number(largest.file_size ?? 0) || (Number(largest.width ?? 0) * Number(largest.height ?? 0));
    const candidateScore = Number(candidate.file_size ?? 0) || (Number(candidate.width ?? 0) * Number(candidate.height ?? 0));
    return candidateScore >= largestScore ? candidate : largest;
  }).file_id;
}

export function detectImageContentType(bytes) {
  const value = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes ?? 0);
  if (value.length >= 3 && value[0] === 0xff && value[1] === 0xd8 && value[2] === 0xff) return "image/jpeg";
  if (value.length >= 8
    && value[0] === 0x89 && value[1] === 0x50 && value[2] === 0x4e && value[3] === 0x47
    && value[4] === 0x0d && value[5] === 0x0a && value[6] === 0x1a && value[7] === 0x0a) return "image/png";
  if (value.length >= 12
    && String.fromCharCode(...value.slice(0, 4)) === "RIFF"
    && String.fromCharCode(...value.slice(8, 12)) === "WEBP") return "image/webp";
  return null;
}

async function refreshAvatar(env, telegramUserId, now, force = false) {
  const existing = await env.AUTH_DB.prepare(
    "SELECT avatar_file_id, avatar_checked_at FROM betterfy_users WHERE telegram_user_id = ?",
  ).bind(String(telegramUserId)).first();
  const refreshAfter = existing?.avatar_file_id ? AVATAR_REFRESH_SECONDS : AVATAR_RETRY_SECONDS;
  if (!force && Number(existing?.avatar_checked_at ?? 0) > now - refreshAfter) {
    return existing?.avatar_file_id ?? null;
  }
  try {
    const photos = await telegram(env, "getUserProfilePhotos", { user_id: telegramUserId, limit: 1 });
    const fileId = selectTelegramAvatarFileId(photos);
    await env.AUTH_DB.prepare(
      "UPDATE betterfy_users SET avatar_file_id = ?, avatar_checked_at = ? WHERE telegram_user_id = ?",
    ).bind(fileId, now, String(telegramUserId)).run();
    return fileId;
  } catch {
    // Avatar is optional. Authentication and payments must keep working when
    // Telegram profile photos are private or temporarily unavailable.
    return existing?.avatar_file_id ?? null;
  }
}

export function normalizeClientKind(value) {
  return value === "web" || value === "desktop" ? value : "unknown";
}

export function refreshCredentialState(credential, now) {
  if (!credential) return "unknown";
  if (credential.used_at != null) return "replayed";
  if (credential.revoked_at != null) return "revoked";
  if (Number(credential.expires_at) <= now) return "expired";
  return "active";
}

export function supportsRotatingDesktopCredentials(payload) {
  return normalizeClientKind(payload?.clientKind) === "desktop"
    && payload?.credentialMode === "rotating-v1";
}

export function refreshFamilyCompromised(row) {
  return Number(row?.revoked_count ?? 0) > 0;
}

export function parseDeviceStartPayload(value) {
  if (typeof value !== "string") return null;
  const match = /^\/start(?:@[A-Za-z0-9_]+)?\s+auth_([A-Za-z0-9_-]{43})$/.exec(value.trim());
  return normalizeChallengeToken(match?.[1]) ?? null;
}

export function deviceChallengeState(row, now) {
  if (!row) return "missing";
  if (Number(row.expires_at) <= now) return "expired";
  if (row.status === "pending" || row.status === "approved" || row.status === "denied" || row.status === "redeemed") {
    return row.status;
  }
  return "missing";
}

async function createSessionRecord(env, userId, now, clientKind, familyId = null, ttl = SESSION_TTL_SECONDS) {
  const token = generateSessionToken();
  const sessionId = crypto.randomUUID();
  const hash = await keyedHash(`session:${token}`, env.AUTH_CODE_PEPPER);
  const statement = env.AUTH_DB.prepare(
    `INSERT INTO auth_sessions
      (session_hash, user_id, created_at, expires_at, last_used_at, session_id, client_kind, family_id)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
  ).bind(hash, userId, now, now + ttl, now, sessionId, normalizeClientKind(clientKind), familyId);
  return { token, sessionId, hash, statement };
}

async function issueSession(env, userId, now, clientKind) {
  const session = await createSessionRecord(env, userId, now, clientKind);
  await session.statement.run();
  return session;
}

async function issueDesktopCredentials(env, userId, now) {
  const familyId = crypto.randomUUID();
  const familyExpiresAt = now + REFRESH_FAMILY_TTL_SECONDS;
  const refreshToken = generateSessionToken();
  const refreshHash = await keyedHash(`refresh:${refreshToken}`, env.AUTH_CODE_PEPPER);
  const session = await createSessionRecord(
    env,
    userId,
    now,
    "desktop",
    familyId,
    DESKTOP_SESSION_TTL_SECONDS,
  );
  await env.AUTH_DB.batch([
    env.AUTH_DB.prepare(
      `INSERT INTO auth_refresh_tokens
        (token_hash, family_id, user_id, generation, created_at, expires_at)
       VALUES (?, ?, ?, 0, ?, ?)`,
    ).bind(refreshHash, familyId, userId, now, familyExpiresAt),
    session.statement,
  ]);
  return { ...session, refreshToken };
}

async function revokeCredentialFamily(env, familyId, now) {
  if (!familyId) return;
  await env.AUTH_DB.batch([
    env.AUTH_DB.prepare(
      "UPDATE auth_refresh_tokens SET revoked_at = COALESCE(revoked_at, ?) WHERE family_id = ?",
    ).bind(now, familyId),
    env.AUTH_DB.prepare(
      "UPDATE auth_sessions SET revoked_at = COALESCE(revoked_at, ?) WHERE family_id = ?",
    ).bind(now, familyId),
  ]);
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
            u.avatar_file_id, u.avatar_checked_at, s.expires_at,
            s.session_id, s.client_kind, s.created_at, s.last_used_at
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
  const now = Math.floor(Date.now() / 1000);
  const user = await authenticatedUser(request, env, now);
  if (!user) return json({ error: "unauthorized" }, 401, headers);
  let fileId = user.avatar_file_id ?? await refreshAvatar(env, user.telegram_user_id, now);
  if (!fileId) return json({ error: "avatar_missing" }, 404, headers);
  let file;
  try {
    file = await telegram(env, "getFile", { file_id: fileId });
  } catch {
    fileId = await refreshAvatar(env, user.telegram_user_id, now, true);
    if (!fileId) return json({ error: "avatar_unavailable" }, 404, headers);
    try {
      file = await telegram(env, "getFile", { file_id: fileId });
    } catch {
      return json({ error: "avatar_unavailable" }, 404, headers);
    }
  }
  const path = file?.file_path;
  if (typeof path !== "string" || path.length > 240 || path.includes("..") || !/^[A-Za-z0-9_./-]+$/.test(path)) {
    return json({ error: "avatar_unavailable" }, 404, headers);
  }
  const response = await fetch(`https://api.telegram.org/file/bot${env.TELEGRAM_BOT_TOKEN}/${path}`);
  if (!response.ok || !response.body) return json({ error: "avatar_unavailable" }, 404, headers);
  const declaredSize = Number(response.headers.get("content-length") ?? 0);
  if (declaredSize > 5 * 1024 * 1024) return json({ error: "avatar_too_large" }, 413, headers);
  const bytes = await response.arrayBuffer();
  if (bytes.byteLength > 5 * 1024 * 1024) return json({ error: "avatar_too_large" }, 413, headers);
  const contentType = detectImageContentType(bytes);
  if (!contentType) return json({ error: "avatar_unavailable" }, 404, headers);
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
  const avatarFileId = await refreshAvatar(env, user.telegram_user_id, now);
  const entitlement = await subscriptionRecord(env, user.user_id);
  const plan = planBySku(entitlement?.sku);
  return json({
    userId: user.user_id,
    displayName: user.display_name,
    username: user.username ?? undefined,
    accessTier: isEntitlementActive(entitlement, now) ? "premium" : "early-access",
    avatarAvailable: Boolean(avatarFileId),
    accessExpiresAt: isEntitlementActive(entitlement, now) ? entitlement.active_until : undefined,
    accessPlan: isEntitlementActive(entitlement, now) ? plan?.id : undefined,
    accessRecurring: isEntitlementActive(entitlement, now) ? Boolean(plan?.recurring && entitlement.canceled_at == null) : false,
    sessionExpiresAt: user.expires_at,
    sessionId: user.session_id ?? undefined,
  }, 200, headers);
}

async function revokeSession(request, env, origin) {
  const headers = corsHeaders(origin);
  const token = bearerToken(request);
  if (!token) return json({ error: "unauthorized" }, 401, headers);
  const hash = await keyedHash(`session:${token}`, env.AUTH_CODE_PEPPER);
  const session = await env.AUTH_DB.prepare(
    "SELECT family_id FROM auth_sessions WHERE session_hash = ?",
  ).bind(hash).first();
  if (session?.family_id) {
    await revokeCredentialFamily(env, session.family_id, Math.floor(Date.now() / 1000));
    return json({ ok: true }, 200, headers);
  }
  const result = await env.AUTH_DB.prepare(
    "UPDATE auth_sessions SET revoked_at = ? WHERE session_hash = ? AND revoked_at IS NULL",
  ).bind(Math.floor(Date.now() / 1000), hash).run();
  return json({ ok: Number(result.meta?.changes ?? 0) === 1 }, 200, headers);
}

async function listDeviceSessions(request, env, origin) {
  const headers = corsHeaders(origin);
  const now = Math.floor(Date.now() / 1000);
  const user = await authenticatedUser(request, env, now);
  if (!user) return json({ error: "unauthorized" }, 401, headers);
  const result = await env.AUTH_DB.prepare(
    `SELECT session_id, client_kind, created_at, last_used_at, expires_at
     FROM auth_sessions
     WHERE user_id = ? AND revoked_at IS NULL AND expires_at > ? AND session_id IS NOT NULL
     ORDER BY last_used_at DESC
     LIMIT 20`,
  ).bind(user.user_id, now).all();
  return json({
    sessions: (result.results ?? []).map((session) => ({
      sessionId: session.session_id,
      clientKind: normalizeClientKind(session.client_kind),
      createdAt: session.created_at,
      lastUsedAt: session.last_used_at,
      expiresAt: session.expires_at,
      current: session.session_id === user.session_id,
    })),
  }, 200, headers);
}

async function revokeDeviceSession(request, env, origin) {
  const headers = corsHeaders(origin);
  const now = Math.floor(Date.now() / 1000);
  const user = await authenticatedUser(request, env, now);
  if (!user) return json({ error: "unauthorized" }, 401, headers);
  let payload;
  try {
    payload = await readJson(request);
  } catch {
    return json({ error: "invalid_request" }, 400, headers);
  }
  const sessionId = typeof payload?.sessionId === "string" ? payload.sessionId : "";
  if (!/^(?:[0-9a-f]{32}|[0-9a-f-]{36})$/i.test(sessionId)) {
    return json({ error: "invalid_session" }, 400, headers);
  }
  const owned = await env.AUTH_DB.prepare(
    "SELECT family_id FROM auth_sessions WHERE session_id = ? AND user_id = ? AND revoked_at IS NULL",
  ).bind(sessionId, user.user_id).first();
  if (!owned) return json({ ok: false, current: false }, 200, headers);
  if (owned.family_id) {
    await revokeCredentialFamily(env, owned.family_id, now);
    return json({ ok: true, current: sessionId === user.session_id }, 200, headers);
  }
  const result = await env.AUTH_DB.prepare(
    `UPDATE auth_sessions SET revoked_at = ?
     WHERE session_id = ? AND user_id = ? AND revoked_at IS NULL`,
  ).bind(now, sessionId, user.user_id).run();
  return json({
    ok: Number(result.meta?.changes ?? 0) === 1,
    current: sessionId === user.session_id,
  }, 200, headers);
}

function authPayload(user, entitlement, plan, session, extra = {}) {
  const now = Math.floor(Date.now() / 1000);
  const active = isEntitlementActive(entitlement, now);
  return {
    userId: user.user_id,
    displayName: user.display_name,
    username: user.username ?? undefined,
    accessTier: active ? "premium" : "early-access",
    sessionToken: session.token,
    sessionId: session.sessionId,
    avatarAvailable: Boolean(user.avatar_file_id),
    accessExpiresAt: active ? entitlement.active_until : undefined,
    accessPlan: active ? plan?.id : undefined,
    accessRecurring: active ? Boolean(plan?.recurring && entitlement.canceled_at == null) : false,
    ...extra,
  };
}

async function refreshDesktopSession(request, env, origin) {
  const headers = corsHeaders(origin);
  const now = Math.floor(Date.now() / 1000);
  let payload;
  try {
    payload = await readJson(request);
  } catch {
    return json({ error: "invalid_request" }, 400, headers);
  }
  const refreshToken = typeof payload?.refreshToken === "string" ? payload.refreshToken : "";
  if (!/^[A-Za-z0-9_-]{43}$/.test(refreshToken)) {
    return json({ error: "refresh_rejected" }, 401, headers);
  }
  const tokenHash = await keyedHash(`refresh:${refreshToken}`, env.AUTH_CODE_PEPPER);
  const credential = await env.AUTH_DB.prepare(
    `SELECT r.family_id, r.user_id, r.generation, r.expires_at, r.used_at, r.revoked_at,
            u.telegram_user_id, u.display_name, u.username, u.language, u.avatar_file_id
     FROM auth_refresh_tokens r
     JOIN betterfy_users u ON u.user_id = r.user_id
     WHERE r.token_hash = ?`,
  ).bind(tokenHash).first();
  const credentialState = refreshCredentialState(credential, now);
  if (credentialState === "unknown") return json({ error: "refresh_rejected" }, 401, headers);
  if (credentialState !== "active") {
    await revokeCredentialFamily(env, credential.family_id, now);
    return json({ error: "refresh_rejected" }, 401, headers);
  }

  const nextRefreshToken = generateSessionToken();
  const nextRefreshHash = await keyedHash(`refresh:${nextRefreshToken}`, env.AUTH_CODE_PEPPER);
  const consumed = await env.AUTH_DB.prepare(
    `UPDATE auth_refresh_tokens SET used_at = ?, replaced_by_hash = ?
     WHERE token_hash = ? AND used_at IS NULL AND revoked_at IS NULL AND expires_at > ?`,
  ).bind(now, nextRefreshHash, tokenHash, now).run();
  if (Number(consumed.meta?.changes ?? 0) !== 1) {
    await revokeCredentialFamily(env, credential.family_id, now);
    return json({ error: "refresh_rejected" }, 401, headers);
  }

  const session = await createSessionRecord(
    env,
    credential.user_id,
    now,
    "desktop",
    credential.family_id,
    DESKTOP_SESSION_TTL_SECONDS,
  );
  try {
    await env.AUTH_DB.batch([
      env.AUTH_DB.prepare(
        `INSERT INTO auth_refresh_tokens
          (token_hash, family_id, user_id, generation, created_at, expires_at)
         VALUES (?, ?, ?, ?, ?, ?)`,
      ).bind(
        nextRefreshHash,
        credential.family_id,
        credential.user_id,
        Number(credential.generation) + 1,
        now,
        credential.expires_at,
      ),
      env.AUTH_DB.prepare(
        "UPDATE auth_sessions SET revoked_at = COALESCE(revoked_at, ?) WHERE family_id = ?",
      ).bind(now, credential.family_id),
      session.statement,
    ]);
  } catch (error) {
    await revokeCredentialFamily(env, credential.family_id, now);
    throw error;
  }
  // A concurrent replay may revoke the family immediately before or after the
  // replacement batch. Checking after publication closes both interleavings:
  // the replay either already marked an older row or will revoke this row too.
  const familyState = await env.AUTH_DB.prepare(
    `SELECT COUNT(*) AS revoked_count
     FROM auth_refresh_tokens
     WHERE family_id = ? AND revoked_at IS NOT NULL`,
  ).bind(credential.family_id).first();
  if (refreshFamilyCompromised(familyState)) {
    await revokeCredentialFamily(env, credential.family_id, now);
    return json({ error: "refresh_rejected" }, 401, headers);
  }
  const entitlement = await subscriptionRecord(env, credential.user_id);
  const plan = planBySku(entitlement?.sku);
  return json(authPayload(credential, entitlement, plan, session, {
    refreshToken: nextRefreshToken,
    refreshExpiresAt: credential.expires_at,
  }), 200, headers);
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
  await refreshAvatar(env, from.id, now);
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

function deviceDecisionKeyboard(language, token) {
  const copy = COPY[language];
  return {
    inline_keyboard: [[
      { text: copy.confirmDevice, callback_data: `device_yes_${token}` },
      { text: copy.denyDevice, callback_data: `device_no_${token}` },
    ]],
  };
}

function primaryAuthDb(env) {
  // Device approval is a security-sensitive, cross-request state machine. A
  // challenge can be created by the desktop and read by the bot immediately,
  // so every decision must start from the latest primary D1 version. The
  // fallback keeps local test doubles and older Wrangler runtimes usable.
  return typeof env.AUTH_DB.withSession === "function"
    ? env.AUTH_DB.withSession("first-primary")
    : env.AUTH_DB;
}

async function sendDeviceChallenge(env, message, language, token, now) {
  const challengeHash = await keyedHash(`device-challenge:${token}`, env.AUTH_CODE_PEPPER);
  const challenge = await primaryAuthDb(env).prepare(
    "SELECT status, expires_at FROM auth_device_challenges WHERE challenge_hash = ?",
  ).bind(challengeHash).first();
  if (deviceChallengeState(challenge, now) !== "pending") {
    return telegram(env, "sendMessage", { chat_id: message.chat.id, text: COPY[language].deviceExpired });
  }
  await upsertUser(env, message.from, language, now);
  return telegram(env, "sendPhoto", {
    chat_id: message.chat.id,
    photo: cardUrl(env, "message-master.png"),
    caption: COPY[language].deviceRequest,
    reply_markup: deviceDecisionKeyboard(language, token),
    protect_content: true,
  });
}

async function decideDeviceChallenge(env, callback, language, token, approved, now) {
  const chatId = callback.message.chat.id;
  const userId = await upsertUser(env, callback.from, language, now);
  const challengeHash = await keyedHash(`device-challenge:${token}`, env.AUTH_CODE_PEPPER);
  const nextStatus = approved ? "approved" : "denied";
  const result = await primaryAuthDb(env).prepare(
    `UPDATE auth_device_challenges
     SET status = ?, user_id = ?, decided_at = ?
     WHERE challenge_hash = ? AND status = 'pending' AND expires_at > ?`,
  ).bind(nextStatus, userId, now, challengeHash, now).run();
  if (Number(result.meta?.changes ?? 0) !== 1) {
    return telegram(env, "sendMessage", { chat_id: chatId, text: COPY[language].deviceExpired });
  }
  try {
    await telegram(env, "editMessageReplyMarkup", {
      chat_id: chatId,
      message_id: callback.message.message_id,
      reply_markup: { inline_keyboard: [] },
    });
  } catch {
    // The decision is already committed; stale Telegram chrome is non-fatal.
  }
  if (!approved) {
    return telegram(env, "sendMessage", { chat_id: chatId, text: COPY[language].deviceDenied });
  }
  return telegram(env, "sendPhoto", {
    chat_id: chatId,
    photo: cardUrl(env, language === "ru" ? "approved-ru.png" : "approved-en.png"),
    caption: COPY[language].deviceApproved,
    protect_content: true,
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
  const challengeToken = parseDeviceStartPayload(message.text);
  if (challengeToken) return sendDeviceChallenge(env, message, language, challengeToken, now);
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
  const deviceDecision = /^(device_yes|device_no)_([A-Za-z0-9_-]{43})$/.exec(String(callback.data ?? ""));
  if (deviceDecision) {
    return decideDeviceChallenge(
      env,
      callback,
      language,
      deviceDecision[2],
      deviceDecision[1] === "device_yes",
      now,
    );
  }
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

async function consumeRequestLimit(env, request, now, namespace, limit, windowSeconds) {
  const ip = request.headers.get("CF-Connecting-IP") ?? "unknown";
  const bucket = await keyedHash(`${namespace}:${ip}`, env.AUTH_CODE_PEPPER);
  const resetBefore = now - windowSeconds;
  const row = await env.AUTH_DB.prepare(
    `INSERT INTO auth_rate_limits (bucket_hash, window_started_at, request_count)
     VALUES (?, ?, 1)
     ON CONFLICT(bucket_hash) DO UPDATE SET
       request_count = CASE WHEN window_started_at < ? THEN 1 ELSE request_count + 1 END,
       window_started_at = CASE WHEN window_started_at < ? THEN ? ELSE window_started_at END
     RETURNING request_count`,
  ).bind(bucket, now, resetBefore, resetBefore, now).first();
  return Number(row?.request_count ?? limit + 1) <= limit;
}

async function consumeRateLimit(env, request, now) {
  return consumeRequestLimit(env, request, now, "verify", VERIFY_LIMIT, VERIFY_WINDOW_SECONDS);
}

async function createDeviceChallenge(request, env, origin) {
  const headers = corsHeaders(origin);
  const now = Math.floor(Date.now() / 1000);
  let payload;
  try {
    payload = await readJson(request);
  } catch {
    return json({ error: "invalid_request" }, 400, headers);
  }
  const requestedDeviceId = payload?.deviceId === null ? null : normalizeDeviceId(payload?.deviceId);
  if ((payload?.deviceId !== null && !requestedDeviceId) || !supportsRotatingDesktopCredentials(payload)) {
    return json({ error: "invalid_request" }, 400, headers);
  }
  const deviceId = requestedDeviceId ?? generateSessionToken();
  if (!(await consumeRequestLimit(
    env,
    request,
    now,
    "device-challenge-create",
    CHALLENGE_CREATE_LIMIT,
    VERIFY_WINDOW_SECONDS,
  ))) {
    return json({ error: "rate_limited" }, 429, { ...headers, "retry-after": "600" });
  }
  const token = generateSessionToken();
  const challengeHash = await keyedHash(`device-challenge:${token}`, env.AUTH_CODE_PEPPER);
  const deviceHash = await keyedHash(`device:${deviceId}`, env.AUTH_CODE_PEPPER);
  const expiresAt = now + DEVICE_CHALLENGE_TTL_SECONDS;
  await env.AUTH_DB.prepare(
    `INSERT INTO auth_device_challenges
      (challenge_hash, device_hash, status, created_at, expires_at)
     VALUES (?, ?, 'pending', ?, ?)`,
  ).bind(challengeHash, deviceHash, now, expiresAt).run();
  const botUsername = /^[A-Za-z0-9_]{5,32}$/.test(String(env.BOT_USERNAME ?? ""))
    ? String(env.BOT_USERNAME)
    : "BeterFyBot";
  return json({
    challengeToken: token,
    deviceId,
    deepLink: `https://t.me/${botUsername}?start=auth_${token}`,
    expiresAt,
    pollAfterSeconds: CHALLENGE_POLL_SECONDS,
  }, 201, headers);
}

async function pollDeviceChallenge(request, env, origin) {
  const headers = corsHeaders(origin);
  const now = Math.floor(Date.now() / 1000);
  let payload;
  try {
    payload = await readJson(request);
  } catch {
    return json({ error: "invalid_request" }, 400, headers);
  }
  const token = normalizeChallengeToken(payload?.challengeToken);
  const deviceId = normalizeDeviceId(payload?.deviceId);
  if (!token || !deviceId) return json({ error: "invalid_request" }, 400, headers);
  const challengeHash = await keyedHash(`device-challenge:${token}`, env.AUTH_CODE_PEPPER);
  const deviceHash = await keyedHash(`device:${deviceId}`, env.AUTH_CODE_PEPPER);
  const challengeDb = primaryAuthDb(env);
  const challenge = await challengeDb.prepare(
    `SELECT status, user_id, expires_at
     FROM auth_device_challenges
     WHERE challenge_hash = ? AND device_hash = ?`,
  ).bind(challengeHash, deviceHash).first();
  const state = deviceChallengeState(challenge, now);
  if (state === "missing") return json({ error: "challenge_not_found" }, 404, headers);
  if (state === "pending") return json({ state: "pending", expiresAt: challenge.expires_at }, 202, headers);
  if (state === "denied") return json({ state: "denied" }, 403, headers);
  if (state === "expired") return json({ state: "expired" }, 410, headers);
  if (state === "redeemed") return json({ error: "challenge_redeemed" }, 409, headers);
  if (!challenge.user_id) return json({ error: "challenge_invalid" }, 409, headers);

  const redeemed = await challengeDb.prepare(
    `UPDATE auth_device_challenges SET status = 'redeemed', redeemed_at = ?
     WHERE challenge_hash = ? AND device_hash = ? AND status = 'approved' AND expires_at > ?`,
  ).bind(now, challengeHash, deviceHash, now).run();
  if (Number(redeemed.meta?.changes ?? 0) !== 1) {
    return json({ error: "challenge_redeemed" }, 409, headers);
  }
  // Redemption is committed before credentials are issued. If issuance fails,
  // the challenge stays closed and the user starts a fresh one; replay can
  // never mint a second credential family.
  const user = await env.AUTH_DB.prepare(
    "SELECT user_id, telegram_user_id, display_name, username, language, avatar_file_id FROM betterfy_users WHERE user_id = ?",
  ).bind(challenge.user_id).first();
  if (!user) return json({ error: "profile_missing" }, 500, headers);
  const session = await issueDesktopCredentials(env, user.user_id, now);
  const entitlement = await subscriptionRecord(env, user.user_id);
  const plan = planBySku(entitlement?.sku);
  return json(authPayload(user, entitlement, plan, session, {
    refreshToken: session.refreshToken,
    refreshExpiresAt: now + REFRESH_FAMILY_TTL_SECONDS,
  }), 200, headers);
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
  const entitlement = await subscriptionRecord(env, user.user_id);
  const plan = planBySku(entitlement?.sku);

  const approvedCard = user.language === "ru" ? "approved-ru.png" : "approved-en.png";
  const copy = COPY[user.language === "ru" ? "ru" : "en"];
  const clientKind = normalizeClientKind(payload?.clientKind);
  const rotatingDesktop = supportsRotatingDesktopCredentials(payload);
  const session = rotatingDesktop
    ? await issueDesktopCredentials(env, user.user_id, now)
    : await issueSession(env, user.user_id, now, clientKind);
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

  return json(authPayload(user, entitlement, plan, session, rotatingDesktop ? {
    refreshToken: session.refreshToken,
    refreshExpiresAt: now + REFRESH_FAMILY_TTL_SECONDS,
  } : {}), 200, headers);
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
    if (request.method === "GET" && url.pathname === "/v1/session/devices") return listDeviceSessions(request, env, origin);
    if (request.method === "POST" && url.pathname === "/v1/session/devices/revoke") return revokeDeviceSession(request, env, origin);
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
  if (url.pathname === "/v1/auth/device/challenges" || url.pathname === "/v1/auth/device/challenges/poll") {
    const origin = allowedOrigin(request, env);
    if (origin === false) return json({ error: "origin_not_allowed" }, 403);
    if (request.method === "OPTIONS") return new Response(null, { status: 204, headers: corsHeaders(origin) });
    if (request.method === "POST" && url.pathname.endsWith("/poll")) {
      return pollDeviceChallenge(request, env, origin);
    }
    if (request.method === "POST") return createDeviceChallenge(request, env, origin);
    return json({ error: "method_not_allowed" }, 405, corsHeaders(origin));
  }
  if (url.pathname === "/v1/auth/refresh") {
    const origin = allowedOrigin(request, env);
    if (origin === false) return json({ error: "origin_not_allowed" }, 403);
    if (request.method === "OPTIONS") return new Response(null, { status: 204, headers: corsHeaders(origin) });
    if (request.method === "POST") return refreshDesktopSession(request, env, origin);
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
