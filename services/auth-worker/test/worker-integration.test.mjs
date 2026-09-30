import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { Miniflare } from "miniflare";

// Runs the Worker in the real workerd runtime against a local D1 database
// built from the migrations, with Telegram and email calls stubbed. Unit
// tests cover pure functions; this covers the SQL they cannot see.
const root = fileURLToPath(new URL("..", import.meta.url));

test("worker flows against a real D1 schema", async (t) => {
  const outbound = [];
  const mf = new Miniflare({
    modules: true,
    scriptPath: `${root}src/index.mjs`,
    modulesRules: [{ type: "ESModule", include: ["**/*.mjs"] }],
    compatibilityDate: "2026-08-01",
    d1Databases: ["AUTH_DB"],
    bindings: {
      AUTH_CODE_PEPPER: "p".repeat(40),
      AUTH_PASSWORD_PEPPER: "q".repeat(40),
      TELEGRAM_WEBHOOK_SECRET: "w".repeat(40),
      TELEGRAM_BOT_TOKEN: "test",
      BOT_USERNAME: "BeterFyBot",
      PUBLIC_WORKER_URL: "https://example.test",
      BOT_ASSET_BASE_URL: "https://example.test",
      ALLOWED_ORIGINS: "https://zori-xyz.github.io",
      VERIFY_GLOBAL_FAILURE_LIMIT: "3",
      RESEND_API_KEY: "re_test",
      EMAIL_FROM: "BetterFy <id@example.test>",
      BETTERFY_PLAN_3D_STARS: "75",
      BETTERFY_PLAN_15D_STARS: "225",
      BETTERFY_PLAN_30D_STARS: "525",
    },
    outboundService: async (request) => {
      outbound.push({ url: request.url, body: await request.text() });
      if (request.url.includes("getUserProfilePhotos"))
        return Response.json({ ok: true, result: { photos: [] } });
      return Response.json({ ok: true, result: {} });
    },
  });
  t.after(() => mf.dispose());
  const db = await mf.getD1Database("AUTH_DB");
  for (const file of readdirSync(`${root}migrations`).sort()) {
    const sql = readFileSync(`${root}migrations/${file}`, "utf8").replace(/--.*$/gm, "");
    for (const statement of sql
      .split(";")
      .map((part) => part.trim())
      .filter(Boolean)) {
      await db.prepare(statement).run();
    }
  }
  const post = (path, body, headers = {}) =>
    mf.dispatchFetch(`https://example.test${path}`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
        "CF-Connecting-IP": "2001:db8:1:2::5",
        ...headers,
      },
      body: JSON.stringify(body),
    });
  const user = { id: 777, first_name: "Test", language_code: "ru" };
  let challenge;
  let token;
  const webhook = (update) =>
    post("/v1/telegram/webhook", update, { "X-Telegram-Bot-Api-Secret-Token": "w".repeat(40) });

  await t.test("Device challenge carries context and a match code shown in the bot", async () => {
    const created = await post("/v1/auth/device/challenges", {
      deviceId: null,
      clientKind: "desktop",
      credentialMode: "rotating-v1",
      platform: "windows",
      appVersion: "0.1.4",
    });
    assert.equal(created.status, 201);
    challenge = await created.json();
    assert.match(challenge.matchCode, /^\d{2}$/);
    token = challenge.deepLink.split("auth_")[1];
    assert.equal(
      (
        await webhook({
          message: { from: user, chat: { id: 777, type: "private" }, text: `/start auth_${token}` },
        })
      ).status,
      200,
    );
    const photo = outbound.find((c) => c.url.endsWith("/sendPhoto"));
    assert.ok(photo.body.includes("Windows · BetterFy 0.1.4"), photo.body);
    assert.ok(photo.body.includes(challenge.matchCode));
  });
  await t.test("Global failure budget stops code guessing across addresses", async () => {
    for (let i = 0; i < 3; i += 1) {
      const r = await post(
        "/v1/auth/telegram/code",
        { code: "123456" },
        { "CF-Connecting-IP": `2001:db8:${i}:1::1` },
      );
      assert.equal(r.status, 401);
    }
    const blocked = await post(
      "/v1/auth/telegram/code",
      { code: "123456" },
      { "CF-Connecting-IP": "2001:db8:99:1::1" },
    );
    assert.equal(blocked.status, 429);
  });
  await t.test("Payments: 30d, then 3d stacked, refund of 30d leaves exactly 3d", async () => {
    const userId = (
      await db.prepare("SELECT user_id FROM betterfy_users WHERE telegram_user_id = '777'").first()
    ).user_id;
    async function pay(planId, chargeId) {
      await webhook({
        callback_query: {
          id: "cb",
          from: user,
          message: { message_id: 1, chat: { id: 777 } },
          data: `buy_${planId}`,
        },
      });
      const order = await db
        .prepare(
          "SELECT * FROM payment_orders WHERE user_id = ? ORDER BY created_at DESC, rowid DESC LIMIT 1",
        )
        .bind(userId)
        .first();
      const r = await webhook({
        message: {
          from: user,
          chat: { id: 777, type: "private" },
          successful_payment: {
            invoice_payload: order.invoice_payload,
            currency: "XTR",
            total_amount: order.amount,
            telegram_payment_charge_id: chargeId,
          },
        },
      });
      assert.equal(r.status, 200);
    }
    const entitlement = () =>
      db.prepare("SELECT active_until FROM entitlements WHERE user_id = ?").bind(userId).first();
    await pay("30d", "charge-month");
    const afterMonth = (await entitlement()).active_until;
    await pay("3d", "charge-pass");
    const afterPass = (await entitlement()).active_until;
    assert.ok(Math.abs(afterPass - afterMonth - 3 * 86400) <= 2, `${afterPass - afterMonth}`);
    await pay("3d", "charge-pass"); // replayed update
    assert.equal((await entitlement()).active_until, afterPass);
    await webhook({
      message: {
        from: user,
        chat: { id: 777, type: "private" },
        refunded_payment: {
          telegram_payment_charge_id: "charge-month",
          currency: "XTR",
          total_amount: 525,
          invoice_payload: "x",
        },
      },
    });
    const afterRefund = (await entitlement()).active_until;
    const now = Math.floor(Date.now() / 1000);
    assert.ok(Math.abs(afterRefund - (now + 3 * 86400)) <= 5, `${afterRefund - now}`);
  });
  await t.test("Scheduled cleanup runs against the real schema", async () => {
    await db
      .prepare(
        "INSERT INTO auth_rate_limits (bucket_hash, window_started_at, request_count) VALUES ('old', 1, 1)",
      )
      .run();
    const worker = await mf.getWorker();
    await worker.scheduled({ cron: "17 3 * * *" });
    assert.equal(
      await db.prepare("SELECT 1 FROM auth_rate_limits WHERE bucket_hash = 'old'").first(),
      null,
    );
  });
  await t.test(
    "Refresh grace: a lost response can be retried once; the stale branch is then rejected",
    async () => {
      await webhook({
        callback_query: {
          id: "cb2",
          from: user,
          message: { message_id: 2, chat: { id: 777 } },
          data: `device_yes_${token}`,
        },
      });
      const polled = await post("/v1/auth/device/challenges/poll", {
        challengeToken: token,
        deviceId: challenge.deviceId,
      });
      assert.equal(polled.status, 200);
      const first = await polled.json();
      const second = await (
        await post("/v1/auth/refresh", { refreshToken: first.refreshToken })
      ).json();
      assert.ok(second.refreshToken);
      const retried = await post("/v1/auth/refresh", { refreshToken: first.refreshToken });
      assert.equal(retried.status, 200, "retry with the previous token inside the grace window");
      const third = await retried.json();
      await db
        .prepare("UPDATE auth_refresh_tokens SET used_at = used_at - 120 WHERE used_at IS NOT NULL")
        .run();
      const stale = await post("/v1/auth/refresh", { refreshToken: second.refreshToken });
      assert.equal(stale.status, 401, "outside the grace window a used token is a replay");
      assert.equal(
        (await post("/v1/auth/refresh", { refreshToken: third.refreshToken })).status,
        401,
        "family revoked after replay",
      );
    },
  );
  await t.test(
    "Registration: taken username is explicit, expired claims are released",
    async () => {
      const reg = (username, email) =>
        post(
          "/v1/auth/id/register/start",
          { username, email, password: "correct horse battery", language: "ru" },
          {
            Origin: "https://zori-xyz.github.io",
            "CF-Connecting-IP": `198.51.100.${Math.floor(Math.random() * 200)}`,
          },
        );
      assert.equal((await reg("player_one", "a@example.test")).status, 202);
      const taken = await reg("player_one", "b@example.test");
      assert.equal(taken.status, 409);
      assert.equal((await taken.json()).error, "username_taken");
      await db.prepare("UPDATE betterfy_id_registrations SET expires_at = 1").run();
      assert.equal(
        (await reg("player_one", "b@example.test")).status,
        202,
        "expired claim released",
      );
    },
  );
});
