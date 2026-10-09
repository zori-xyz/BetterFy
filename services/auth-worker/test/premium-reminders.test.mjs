import test from "node:test";
import assert from "node:assert/strict";
import { sendPremiumReminders } from "../src/index.mjs";

const NOW = 1_800_000_000;
const DAY = 86_400;

function setup(rows, { recurring = null } = {}) {
  const sent = [];
  const marked = [];
  const env = {
    TELEGRAM_BOT_TOKEN: "token",
    BOT_ASSET_BASE_URL: "https://assets.example/bot",
    AUTH_DB: {
      prepare: (sql) => ({
        bind: (...args) => ({
          all: async () => ({ results: sql.includes("FROM entitlements e") ? rows : [] }),
          first: async () => (sql.includes("FROM star_payment_events p") ? recurring : null),
          run: async () => {
            if (sql.includes("SET reminded_until")) marked.push(args);
            return { meta: { changes: 1 } };
          },
        }),
      }),
    },
  };
  const realFetch = globalThis.fetch;
  globalThis.fetch = async (url, init) => {
    sent.push({ method: String(url).split("/").pop(), body: JSON.parse(init.body) });
    return new Response(JSON.stringify({ ok: true, result: {} }), { status: 200 });
  };
  return { env, sent, marked, restore: () => { globalThis.fetch = realFetch; } };
}

const row = { user_id: "u1", active_until: NOW + 2 * DAY, telegram_user_id: "42", language: "ru" };

test("a Premium ending soon gets one reminder with a renew button, then is marked", async () => {
  const run = setup([row]);
  try {
    assert.equal(await sendPremiumReminders(run.env, NOW), 1);
    assert.equal(run.sent[0].method, "sendPhoto");
    assert.match(run.sent[0].body.caption, /заканчивается/);
    assert.equal(run.sent[0].body.reply_markup.inline_keyboard[0][0].callback_data, "plans");
    assert.deepEqual(run.marked[0].slice(0, 2), [row.active_until, "u1"]);
  } finally {
    run.restore();
  }
});

test("a subscription that renews by itself is marked but not reminded", async () => {
  const run = setup([row], { recurring: { telegram_charge_id: "c", canceled_at: null } });
  try {
    assert.equal(await sendPremiumReminders(run.env, NOW), 0);
    assert.equal(run.sent.length, 0);
    assert.equal(run.marked.length, 1);
  } finally {
    run.restore();
  }
});

test("a canceled subscription is reminded like a one-time pass", async () => {
  const run = setup([{ ...row, language: "en" }], { recurring: { telegram_charge_id: "c", canceled_at: NOW - DAY } });
  try {
    assert.equal(await sendPremiumReminders(run.env, NOW), 1);
    assert.match(run.sent[0].body.caption, /ends on/);
  } finally {
    run.restore();
  }
});
