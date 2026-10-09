import test from "node:test";
import assert from "node:assert/strict";
import { route } from "../src/index.mjs";

const SECRET = "w".repeat(40);

function fakeDatabase(state) {
  const statement = (sql) => ({
    bind: (...args) => ({
      sql,
      args,
      first: async () => {
        if (sql.includes("FROM betterfy_users WHERE telegram_user_id")) return { language: state.language };
        if (sql.includes("INSERT INTO betterfy_users")) return { user_id: "u1" };
        if (sql.includes("FROM entitlements")) return state.entitlement ?? null;
        if (sql.includes("avatar_checked_at")) return { avatar_checked_at: Math.floor(Date.now() / 1000) };
        return null;
      },
      all: async () => ({ results: sql.includes("FROM auth_sessions") ? state.sessions : [] }),
      run: async () => ({ meta: { changes: 1 } }),
    }),
  });
  return {
    prepare: statement,
    batch: async (statements) => {
      state.batches.push(statements.map((entry) => entry.sql));
      return [];
    },
  };
}

async function send(update, state) {
  const calls = [];
  const realFetch = globalThis.fetch;
  globalThis.fetch = async (url, init) => {
    const method = String(url).split("/").pop();
    calls.push({ method, body: JSON.parse(init.body) });
    return new Response(JSON.stringify({ ok: true, result: {} }), { status: 200 });
  };
  try {
    const env = {
      TELEGRAM_WEBHOOK_SECRET: SECRET,
      TELEGRAM_BOT_TOKEN: "token",
      BOT_ASSET_BASE_URL: "https://assets.example/bot",
      AUTH_CODE_PEPPER: "p".repeat(40),
      ALLOWED_ORIGINS: "https://zori-xyz.github.io",
      AUTH_DB: fakeDatabase(state),
    };
    const response = await route(new Request("https://auth.example/v1/telegram/webhook", {
      method: "POST",
      headers: { "X-Telegram-Bot-Api-Secret-Token": SECRET, "content-type": "application/json" },
      body: JSON.stringify(update),
    }), env);
    assert.equal(response.status, 200);
  } finally {
    globalThis.fetch = realFetch;
  }
  return calls.filter((call) => !["getUserProfilePhotos", "getFile"].includes(call.method));
}

const from = { id: 42, first_name: "Test", language_code: "ru" };
const state = () => ({ language: "ru", sessions: [], batches: [] });
const message = (text) => ({ update_id: 1, message: { message_id: 7, from, chat: { id: 42, type: "private" }, text } });
const callback = (data) => ({
  update_id: 2,
  callback_query: { id: "cb", from, data, message: { message_id: 9, chat: { id: 42, type: "private" } } },
});

test("the menu button edits its own card instead of sending a new one", async () => {
  const calls = await send(callback("plans"), state());
  const edit = calls.find((call) => call.method === "editMessageMedia");
  assert.ok(edit, "expected the card to be edited in place");
  assert.equal(edit.body.message_id, 9);
  assert.ok(edit.body.reply_markup.inline_keyboard.flat().some((button) => button.callback_data === "menu"));
  assert.equal(calls.some((call) => call.method === "sendPhoto"), false);
});

test("free text gets the menu with a short hint, /start gets the greeting", async () => {
  const hint = await send(message("привет"), state());
  assert.match(hint.find((call) => call.method === "sendPhoto").body.caption, /кнопки и команды/);
  const start = await send(message("/start"), state());
  assert.match(start.find((call) => call.method === "sendPhoto").body.caption, /BetterFy на связи/);
});

test("/language flips the language and answers in it", async () => {
  const calls = await send(message("/language"), state());
  assert.match(calls.find((call) => call.method === "sendPhoto").body.caption, /Language switched to English/);
});

test("/devices lists active sign-ins and offers to close them all", async () => {
  const data = state();
  data.sessions = [{ client_kind: "windows", count: 1 }, { client_kind: "web", count: 2 }];
  const calls = await send(message("/devices"), data);
  const card = calls.find((call) => call.method === "sendPhoto").body;
  assert.match(card.caption, /Windows: 1/);
  assert.match(card.caption, /сайт: 2/);
  assert.ok(card.reply_markup.inline_keyboard.flat().some((button) => button.callback_data === "signout_ask"));
});

test("signing out everywhere needs a confirmation and then revokes sessions and refresh tokens", async () => {
  const ask = await send(callback("signout_ask"), state());
  const confirm = ask.find((call) => call.method === "editMessageMedia").body.reply_markup.inline_keyboard.flat();
  assert.ok(confirm.some((button) => button.callback_data === "signout_yes"));

  const data = state();
  await send(callback("signout_yes"), data);
  assert.equal(data.batches.length, 1);
  assert.match(data.batches[0][0], /UPDATE auth_sessions SET revoked_at/);
  assert.match(data.batches[0][1], /UPDATE auth_refresh_tokens SET revoked_at/);
});

test("/support points at the support account", async () => {
  const calls = await send(message("/support"), state());
  const reply = calls.find((call) => call.method === "sendMessage").body;
  assert.match(reply.text, /@BeterHelp/);
  assert.equal(reply.reply_markup.inline_keyboard[0][0].url, "https://t.me/BeterHelp");
});
