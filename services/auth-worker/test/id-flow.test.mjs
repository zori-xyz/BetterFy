import test from "node:test";
import assert from "node:assert/strict";
import { DatabaseSync } from "node:sqlite";
import { readFileSync } from "node:fs";
import { route } from "../src/index.mjs";

function database() {
  const sqlite = new DatabaseSync(":memory:");
  for (let index = 1; index <= 8; index += 1) {
    const suffix = ["auth", "stars_subscriptions", "identity_sessions", "device_sessions", "refresh_rotation", "device_challenges", "betterfy_id_email", "betterfy_id_password"][index - 1];
    const file = new URL(`../migrations/${String(index).padStart(4, "0")}_${suffix}.sql`, import.meta.url);
    sqlite.exec(readFileSync(file, "utf8"));
  }
  const wrap = (query, values = []) => ({
    query, values,
    bind(...bound) { return wrap(query, bound); },
    async first() { return sqlite.prepare(query).get(...values) ?? null; },
    async all() { return { results: sqlite.prepare(query).all(...values) }; },
    async run() { const result = sqlite.prepare(query).run(...values); return { meta: { changes: Number(result.changes) } }; },
  });
  return {
    sqlite,
    prepare(query) { return wrap(query); },
    async batch(statements) {
      sqlite.exec("BEGIN");
      try {
        const result = [];
        for (const statement of statements) result.push(await statement.run());
        sqlite.exec("COMMIT");
        return result;
      } catch (error) {
        sqlite.exec("ROLLBACK");
        throw error;
      }
    },
  };
}

function post(path, data) {
  return new Request(`https://auth.example${path}`, {
    method: "POST",
    headers: { origin: "https://zori-xyz.github.io", "content-type": "application/json", "CF-Connecting-IP": "203.0.113.10" },
    body: JSON.stringify(data),
  });
}

test("ID registration verifies email, stores only a salted verifier, and signs in by username or email", async () => {
  const db = database();
  const env = {
    AUTH_DB: db,
    ALLOWED_ORIGINS: "https://zori-xyz.github.io",
    AUTH_CODE_PEPPER: "test-only-code-pepper-with-more-than-32-characters",
    AUTH_PASSWORD_PEPPER: "different-test-password-pepper-32-characters",
    RESEND_API_KEY: "test-only-mail-key",
    EMAIL_FROM: "test@example.com",
  };
  const previousFetch = globalThis.fetch;
  let deliveredCode = "";
  globalThis.fetch = async (url, options) => {
    assert.equal(url, "https://api.resend.com/emails");
    const mail = JSON.parse(options.body);
    deliveredCode = /\b\d{6}\b/.exec(mail.text)?.[0] ?? "";
    return new Response("{}", { status: 200 });
  };
  try {
    const registration = await route(post("/v1/auth/id/register/start", {
      username: "Player_07", email: "player@example.com", password: "a long passphrase 2026", language: "ru",
    }), env);
    assert.equal(registration.status, 202);
    assert.match(deliveredCode, /^\d{6}$/);
    assert.equal(db.sqlite.prepare("SELECT COUNT(*) AS count FROM betterfy_users").get().count, 0);

    const confirmed = await route(post("/v1/auth/id/register/verify", {
      email: "player@example.com", code: deliveredCode, clientKind: "web",
    }), env);
    assert.equal(confirmed.status, 200);
    const profile = await confirmed.json();
    assert.equal(profile.username, "player_07");
    assert.equal(typeof profile.sessionToken, "string");
    const stored = db.sqlite.prepare("SELECT u.telegram_user_id, c.password_hash FROM betterfy_users u JOIN betterfy_id_credentials c ON c.user_id = u.user_id").get();
    assert.match(stored.telegram_user_id, /^id:/);
    assert.match(stored.password_hash, /^pbkdf2-sha256\$600000\$/);
    assert.doesNotMatch(stored.password_hash, /long passphrase/);

    const replay = await route(post("/v1/auth/id/register/verify", { email: "player@example.com", code: deliveredCode }), env);
    assert.equal(replay.status, 401);
    const wrong = await route(post("/v1/auth/id/login", { identifier: "Player_07", password: "wrong passphrase 2026", clientKind: "web" }), env);
    assert.equal(wrong.status, 401);
    const byUsername = await route(post("/v1/auth/id/login", { identifier: "Player_07", password: "a long passphrase 2026", clientKind: "web" }), env);
    assert.equal(byUsername.status, 200);
    const byEmail = await route(post("/v1/auth/id/login", { identifier: "PLAYER@example.com", password: "a long passphrase 2026", clientKind: "web" }), env);
    assert.equal(byEmail.status, 200);
    assert.equal((await byEmail.json()).userId, profile.userId);
  } finally {
    globalThis.fetch = previousFetch;
    db.sqlite.close();
  }
});
