import test from "node:test";
import assert from "node:assert/strict";
import {
  detectImageContentType,
  deviceChallengeState,
  normalizeClientKind,
  parseDeviceStartPayload,
  refreshFamilyCompromised,
  refreshCredentialState,
  route,
  selectTelegramAvatarFileId,
  supportsRotatingDesktopCredentials,
} from "../src/index.mjs";

const env = {
  ALLOWED_ORIGINS: "https://zori-xyz.github.io,http://localhost:1420",
  TELEGRAM_WEBHOOK_SECRET: "webhook-secret",
};

test("avatar selection prefers the largest valid Telegram photo", () => {
  assert.equal(selectTelegramAvatarFileId({
    photos: [[
      { file_id: "small", width: 160, height: 160, file_size: 1_000 },
      { file_id: "large", width: 640, height: 640, file_size: 8_000 },
      { file_id: "medium", width: 320, height: 320, file_size: 4_000 },
    ]],
  }), "large");
  assert.equal(selectTelegramAvatarFileId({ photos: [] }), null);
  assert.equal(selectTelegramAvatarFileId(null), null);
});

test("avatar proxy identifies image bytes instead of trusting Telegram headers", () => {
  assert.equal(detectImageContentType(Uint8Array.from([0xff, 0xd8, 0xff, 0xe0])), "image/jpeg");
  assert.equal(detectImageContentType(Uint8Array.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])), "image/png");
  assert.equal(detectImageContentType(Uint8Array.from([
    0x52, 0x49, 0x46, 0x46, 0, 0, 0, 0, 0x57, 0x45, 0x42, 0x50,
  ])), "image/webp");
  assert.equal(detectImageContentType(Uint8Array.from([0x3c, 0x73, 0x76, 0x67])), null);
});

test("session clients are reduced to supported privacy-safe labels", () => {
  assert.equal(normalizeClientKind("web"), "web");
  assert.equal(normalizeClientKind("desktop"), "desktop");
  assert.equal(normalizeClientKind("Windows 11 · Chrome 139"), "unknown");
});

test("rotation is negotiated explicitly so older desktop builds keep working", () => {
  assert.equal(supportsRotatingDesktopCredentials({ clientKind: "desktop" }), false);
  assert.equal(supportsRotatingDesktopCredentials({ clientKind: "web", credentialMode: "rotating-v1" }), false);
  assert.equal(supportsRotatingDesktopCredentials({ clientKind: "desktop", credentialMode: "rotating-v1" }), true);
});

test("device deep links accept only an exact opaque challenge", () => {
  const token = "a".repeat(43);
  assert.equal(parseDeviceStartPayload(`/start auth_${token}`), token);
  assert.equal(parseDeviceStartPayload(`/start@BeterFyBot auth_${token}`), token);
  assert.equal(parseDeviceStartPayload(`/start auth_${"a".repeat(42)}`), null);
  assert.equal(parseDeviceStartPayload(`/start auth_${token} trailing`), null);
  assert.equal(parseDeviceStartPayload("/start"), null);
});

test("device challenges fail closed across terminal and expired states", () => {
  assert.equal(deviceChallengeState(null, 100), "missing");
  assert.equal(deviceChallengeState({ status: "pending", expires_at: 101 }, 100), "pending");
  assert.equal(deviceChallengeState({ status: "approved", expires_at: 101 }, 100), "approved");
  assert.equal(deviceChallengeState({ status: "denied", expires_at: 101 }, 100), "denied");
  assert.equal(deviceChallengeState({ status: "redeemed", expires_at: 101 }, 100), "redeemed");
  assert.equal(deviceChallengeState({ status: "approved", expires_at: 100 }, 100), "expired");
  assert.equal(deviceChallengeState({ status: "unknown", expires_at: 101 }, 100), "missing");
});

test("refresh credentials fail closed on expiry, revocation, and replay", () => {
  assert.equal(refreshCredentialState(null, 100), "unknown");
  assert.equal(refreshCredentialState({ expires_at: 101, used_at: null, revoked_at: null }, 100), "active");
  assert.equal(refreshCredentialState({ expires_at: 100, used_at: null, revoked_at: null }, 100), "expired");
  assert.equal(refreshCredentialState({ expires_at: 200, used_at: 99, revoked_at: null }, 100), "replayed");
  assert.equal(refreshCredentialState({ expires_at: 200, used_at: null, revoked_at: 99 }, 100), "revoked");
});

test("a replacement is rejected if any concurrent request revoked its family", () => {
  assert.equal(refreshFamilyCompromised({ revoked_count: 0 }), false);
  assert.equal(refreshFamilyCompromised({ revoked_count: 1 }), true);
  assert.equal(refreshFamilyCompromised({ revoked_count: "2" }), true);
});

test("health endpoint exposes no deployment details", async () => {
  const response = await route(new Request("https://auth.example/health"), env);
  assert.equal(response.status, 200);
  assert.deepEqual(await response.json(), { ok: true, service: "betterfy-auth" });
  assert.equal(response.headers.get("cache-control"), "no-store");
});

test("webhook rejects a missing or wrong Telegram secret before parsing", async () => {
  const missing = await route(new Request("https://auth.example/v1/telegram/webhook", {
    method: "POST",
    body: "not-json",
  }), env);
  assert.equal(missing.status, 404);

  const wrong = await route(new Request("https://auth.example/v1/telegram/webhook", {
    method: "POST",
    headers: { "X-Telegram-Bot-Api-Secret-Token": "wrong" },
    body: "not-json",
  }), env);
  assert.equal(wrong.status, 404);
});

test("webhook remains closed when the deployment secret is absent", async () => {
  const response = await route(new Request("https://auth.example/v1/telegram/webhook", {
    method: "POST",
    body: JSON.stringify({ update_id: 1 }),
  }), { ALLOWED_ORIGINS: env.ALLOWED_ORIGINS });
  assert.equal(response.status, 404);
});

test("verification denies unlisted browser origins", async () => {
  const response = await route(new Request("https://auth.example/v1/auth/telegram/code", {
    method: "POST",
    headers: { Origin: "https://attacker.example", "content-type": "application/json" },
    body: JSON.stringify({ code: "123456" }),
  }), env);
  assert.equal(response.status, 403);
  assert.equal(response.headers.get("access-control-allow-origin"), null);
});

test("device challenge endpoints reject malformed bindings before database access", async () => {
  const create = await route(new Request("https://auth.example/v1/auth/device/challenges", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ deviceId: "bad", clientKind: "desktop", credentialMode: "rotating-v1" }),
  }), env);
  assert.equal(create.status, 400);

  const poll = await route(new Request("https://auth.example/v1/auth/device/challenges/poll", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ deviceId: "0".repeat(43), challengeToken: "bad" }),
  }), env);
  assert.equal(poll.status, 400);
});

test("refresh denies malformed tokens before database access", async () => {
  const response = await route(new Request("https://auth.example/v1/auth/refresh", {
    method: "POST",
    headers: { Origin: "https://zori-xyz.github.io", "content-type": "application/json" },
    body: JSON.stringify({ refreshToken: "not-a-token" }),
  }), env);
  assert.equal(response.status, 401);
  assert.deepEqual(await response.json(), { error: "refresh_rejected" });
});

test("preflight echoes only an allowed origin", async () => {
  const response = await route(new Request("https://auth.example/v1/auth/telegram/code", {
    method: "OPTIONS",
    headers: { Origin: "https://zori-xyz.github.io" },
  }), env);
  assert.equal(response.status, 204);
  assert.equal(response.headers.get("access-control-allow-origin"), "https://zori-xyz.github.io");
  assert.match(response.headers.get("access-control-allow-headers"), /authorization/);
});

test("profile, device, and release routes require a bearer session", async () => {
  for (const path of ["/v1/session/profile", "/v1/session/avatar", "/v1/session/devices", "/v1/releases/latest"]) {
    const response = await route(new Request(`https://auth.example${path}`, {
      headers: { Origin: "https://zori-xyz.github.io" },
    }), env);
    assert.equal(response.status, 401);
    assert.equal(response.headers.get("access-control-allow-origin"), "https://zori-xyz.github.io");
  }
});

test("device revocation requires a bearer session", async () => {
  const response = await route(new Request("https://auth.example/v1/session/devices/revoke", {
    method: "POST",
    headers: { Origin: "https://zori-xyz.github.io", "content-type": "application/json" },
    body: JSON.stringify({ sessionId: crypto.randomUUID() }),
  }), env);
  assert.equal(response.status, 401);
});

test("Telegram bootstrap endpoint is closed without a deployment secret", async () => {
  const response = await route(new Request("https://auth.example/internal/configure-telegram", {
    method: "POST",
  }), env);
  assert.equal(response.status, 404);
});
