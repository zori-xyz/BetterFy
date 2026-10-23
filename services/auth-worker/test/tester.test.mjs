import test from "node:test";
import assert from "node:assert/strict";
import { DatabaseSync } from "node:sqlite";
import { readFileSync, readdirSync } from "node:fs";
import { route } from "../src/index.mjs";
import { keyedHash } from "../src/security.mjs";
import { MAX_TESTER_IMAGE_BYTES, TESTER_REPORTS_PER_DAY, escapeHtml, imageKind, parseTesterReport, testerLogins, testerSummary } from "../src/tester.mjs";

const PEPPER = "test-only-code-pepper-with-more-than-32-characters";
const PNG = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13]);
const JPEG = new Uint8Array([0xff, 0xd8, 0xff, 0xe0, 0, 16]);

function database() {
  const sqlite = new DatabaseSync(":memory:");
  const directory = new URL("../migrations/", import.meta.url);
  for (const name of readdirSync(directory).filter((file) => file.endsWith(".sql")).sort()) {
    sqlite.exec(readFileSync(new URL(name, directory), "utf8"));
  }
  const wrap = (query, values = []) => ({
    bind(...bound) { return wrap(query, bound); },
    async first() { return sqlite.prepare(query).get(...values) ?? null; },
    async all() { return { results: sqlite.prepare(query).all(...values) }; },
    async run() { const result = sqlite.prepare(query).run(...values); return { meta: { changes: Number(result.changes) } }; },
  });
  return { sqlite, prepare(query) { return wrap(query); } };
}

async function account(db, login) {
  const token = `${login.padEnd(43, "x")}`.slice(0, 43);
  const userId = `user-${login}`;
  db.sqlite.prepare(
    `INSERT INTO betterfy_users (user_id, telegram_user_id, display_name, username, language, created_at, updated_at)
     VALUES (?, ?, ?, ?, 'ru', 1, 1)`,
  ).run(userId, `id:${login}`, login, login);
  db.sqlite.prepare(
    "INSERT INTO betterfy_id_credentials (user_id, username_key, password_hash, created_at, updated_at) VALUES (?, ?, 'x', 1, 1)",
  ).run(userId, login);
  db.sqlite.prepare(
    `INSERT INTO auth_sessions (session_hash, user_id, created_at, expires_at, last_used_at, session_id, client_kind)
     VALUES (?, ?, 1, 4102444800, 1, ?, 'desktop')`,
  ).run(await keyedHash(`session:${token}`, PEPPER), userId, `session-${login}`);
  return token;
}

function environment(db) {
  return {
    AUTH_DB: db,
    AUTH_CODE_PEPPER: PEPPER,
    ALLOWED_ORIGINS: "https://zori-xyz.github.io",
    TELEGRAM_BOT_TOKEN: "test-only-bot-token",
    BETTERFY_TESTER_LOGINS: " BetterTest1 ,bettertest2",
    TESTER_REPORTS_CHAT_ID: "123456789",
  };
}

function report(overrides = {}) {
  return {
    schema: 1,
    appVersion: "0.6.0",
    platform: "Windows 11 Pro 26100",
    tzOffsetMinutes: 180,
    startedAt: new Date(Date.now() - 42 * 60 * 1000).toISOString(),
    counts: { screens: 12, clicks: 87, failures: 2, flags: 1 },
    notes: [
      { at: "2026-10-19T17:14:00Z", text: "Курсор не сменился <script>alert(1)</script> & всё" },
      { at: "2026-10-19T17:20:00Z", text: "После отката всё вернулось" },
    ],
    images: [],
    events: [{ type: "screen", at: "2026-10-19T17:00:00Z", screen: "catalog" }],
    ...overrides,
  };
}

// The desktop sends a sized body, as reqwest does for in-memory parts.
async function upload(token, body, images = []) {
  const form = new FormData();
  form.set("report", typeof body === "string" ? body : JSON.stringify(body));
  for (const image of images) form.append("image", new Blob([image]), "picture.bin");
  const encoded = new Response(form);
  const bytes = new Uint8Array(await encoded.arrayBuffer());
  return new Request("https://auth.example/v1/tester/reports", {
    method: "POST",
    headers: {
      authorization: `Bearer ${token}`,
      "content-type": encoded.headers.get("content-type"),
      "content-length": String(bytes.length),
    },
    body: bytes,
  });
}

function telegramRecorder({ reject = () => false } = {}) {
  const calls = [];
  const fetch = async (url, options) => {
    const method = String(url).split("/").pop();
    const body = options.body instanceof FormData ? Object.fromEntries([...options.body.keys()].map((key) => [key, options.body.getAll(key)])) : JSON.parse(options.body);
    calls.push({ method, body });
    if (reject(method, body)) return Response.json({ ok: false, description: "Bad Request" }, { status: 400 });
    return Response.json({ ok: true, result: method === "sendMediaGroup" ? [{ message_id: 2 }] : { message_id: calls.length } });
  };
  return { calls, fetch };
}

async function withTelegram(recorder, action) {
  const previous = globalThis.fetch;
  globalThis.fetch = recorder.fetch;
  try {
    return await action();
  } finally {
    globalThis.fetch = previous;
  }
}

test("only listed BetterFy ID logins are testers, and never more than three", async () => {
  const db = database();
  const env = environment(db);
  const tester = await account(db, "bettertest1");
  const other = await account(db, "player_07");
  const profile = async (token) => (await route(new Request("https://auth.example/v1/session/profile", {
    headers: { authorization: `Bearer ${token}` },
  }), env)).json();
  await withTelegram(telegramRecorder(), async () => {
    assert.equal((await profile(tester)).tester, true);
    assert.equal((await profile(other)).tester, false);
    env.BETTERFY_TESTER_LOGINS = "bettertest1,bettertest2,bettertest3,bettertest4";
    assert.equal((await profile(tester)).tester, false, "a fourth login turns the program off");
  });
  assert.equal(testerLogins({ BETTERFY_TESTER_LOGINS: "" }, (value) => value).size, 0);
  db.sqlite.close();
});

test("a tester's report reaches the founder's chat as a summary, an album and the journal", async () => {
  const db = database();
  const env = environment(db);
  const token = await account(db, "bettertest1");
  const recorder = telegramRecorder();
  const body = report({ images: [{ kind: "snapshot", note: 0 }, { kind: "attachment", note: null }] });
  const response = await withTelegram(recorder, async () => route(await upload(token, body, [PNG, JPEG]), env));
  assert.equal(response.status, 200, await response.clone().text());
  assert.deepEqual(recorder.calls.map((call) => call.method), ["sendMessage", "sendMediaGroup", "sendDocument"]);

  const summary = recorder.calls[0].body;
  assert.equal(summary.chat_id, "123456789");
  assert.equal(summary.parse_mode, "HTML");
  assert.match(summary.text, /bettertest1/);
  assert.match(summary.text, /&lt;script&gt;alert\(1\)&lt;\/script&gt; &amp; всё/);
  assert.doesNotMatch(summary.text, /<script>/);
  assert.match(summary.text, /1\. 20:14 · /, "the tester's own clock");

  const album = recorder.calls[1].body;
  const media = JSON.parse(album.media[0]);
  assert.deepEqual(media.map((item) => item.type), ["photo", "photo"]);
  assert.match(media[0].caption, /снимок окна BetterFy · к заметке 1/);
  assert.equal(album.image0[0].type, "image/png");
  assert.equal(album.image1[0].type, "image/jpeg");

  const journal = recorder.calls[2].body;
  assert.match(journal.document[0].name, /^report-bettertest1-\d{8}T\d{6}Z\.json$/);
  assert.equal(JSON.parse(await journal.document[0].text()).events.length, 1);

  const row = db.sqlite.prepare("SELECT * FROM tester_reports").get();
  assert.equal(row.delivered, 1);
  assert.equal(row.image_count, 2);
  assert.equal(row.note_count, 2);
  assert.equal(Object.keys(row).some((key) => /text|note_body|content/.test(key)), false, "no content is stored");
  db.sqlite.close();
});

test("one picture goes as a photo, and as a file when Telegram refuses the photo", async () => {
  const db = database();
  const env = environment(db);
  const token = await account(db, "bettertest1");
  const recorder = telegramRecorder({ reject: (method) => method === "sendPhoto" });
  const body = report({ images: [{ kind: "attachment", note: 1 }] });
  const response = await withTelegram(recorder, async () => route(await upload(token, body, [JPEG]), env));
  assert.equal(response.status, 200);
  assert.deepEqual(recorder.calls.map((call) => call.method), ["sendMessage", "sendPhoto", "sendDocument", "sendDocument"]);
  assert.equal(recorder.calls[2].body.document[0].type, "image/jpeg");
  db.sqlite.close();
});

test("reports from others, malformed reports and disguised files are refused", async () => {
  const db = database();
  const env = environment(db);
  const tester = await account(db, "bettertest1");
  const player = await account(db, "player_07");
  const recorder = telegramRecorder();
  await withTelegram(recorder, async () => {
    assert.equal((await route(await upload(player, report()), env)).status, 403);
    assert.equal((await route(await upload("x".repeat(43), report()), env)).status, 401);
    assert.equal((await route(await upload(tester, "{not json"), env)).status, 400);
    assert.equal((await route(await upload(tester, report({ schema: 2 })), env)).status, 400);
    assert.equal((await route(await upload(tester, report(), [PNG]), env)).status, 400, "an image the report does not list");
    const listed = report({ images: [{ kind: "snapshot", note: null }] });
    const disguised = new TextEncoder().encode("MZ this is an executable");
    assert.equal((await route(await upload(tester, listed, [disguised]), env)).status, 400);
    assert.equal((await route(await upload(tester, listed, [new Uint8Array(MAX_TESTER_IMAGE_BYTES + 1).fill(0xff)]), env)).status, 400);
    assert.equal((await route(await upload(tester, report({ images: [{ kind: "snapshot", note: 7 }] }), [PNG]), env)).status, 400);
    const unsized = await upload(tester, report());
    const noLength = new Request(unsized.url, { method: "POST", headers: { authorization: `Bearer ${tester}`, "content-type": unsized.headers.get("content-type") }, body: await unsized.arrayBuffer() });
    noLength.headers.delete("content-length");
    assert.equal((await route(noLength, env)).status, 411);
    delete env.TESTER_REPORTS_CHAT_ID;
    assert.equal((await route(await upload(tester, report()), env)).status, 503);
  });
  db.sqlite.close();
});

test("a tester may send a bounded number of reports a day, and failed deliveries are recorded", async () => {
  const db = database();
  const env = environment(db);
  const token = await account(db, "bettertest1");
  await withTelegram(telegramRecorder({ reject: (method) => method === "sendMessage" }), async () => {
    assert.equal((await route(await upload(token, report()), env)).status, 502);
  });
  assert.equal(db.sqlite.prepare("SELECT delivered FROM tester_reports").get().delivered, 0);
  await withTelegram(telegramRecorder(), async () => {
    for (let index = 1; index < TESTER_REPORTS_PER_DAY; index += 1) {
      assert.equal((await route(await upload(token, report()), env)).status, 200);
    }
    assert.equal((await route(await upload(token, report()), env)).status, 429);
  });
  db.sqlite.close();
});

test("the summary fits one Telegram message however long the notes are", () => {
  const long = report({
    notes: Array.from({ length: 50 }, (_, index) => ({ at: "2026-10-19T17:14:00Z", text: `${index} ${"<&>".repeat(130)}` })),
  });
  const parsed = parseTesterReport(JSON.stringify(long), 0);
  assert.ok(parsed);
  const text = testerSummary(parsed, "bettertest1", Math.floor(Date.now() / 1000));
  assert.ok(text.length <= 4096, `${text.length}`);
  assert.match(text, /… ещё \d+ в файле/);
  assert.doesNotMatch(text, /&[a-z]*…/, "no entity is cut in half");
  assert.equal(escapeHtml("<b>&"), "&lt;b&gt;&amp;");
  assert.equal(imageKind(PNG), "png");
  assert.equal(imageKind(JPEG), "jpeg");
  assert.equal(imageKind(new Uint8Array([0x47, 0x49, 0x46])), null);
  assert.equal(parseTesterReport(JSON.stringify(report({ notes: [{ at: "x", text: "a" }] })), 0), null);
  assert.equal(parseTesterReport(JSON.stringify(report({ counts: { screens: -1, clicks: 0, failures: 0, flags: 0 } })), 0), null);
});

test("a crash note is validated, summarised for the founder, and dropped when malformed", () => {
  const good = parseTesterReport(JSON.stringify(report({
    previousRun: { uncleanExit: true, version: "0.5.0", panic: { message: "index out of bounds <x>", location: "src/main.rs:12" } },
  })), 0);
  assert.ok(good);
  assert.equal(good.previousRun.uncleanExit, true);
  const text = testerSummary(good, "bettertest1", Math.floor(Date.now() / 1000));
  assert.match(text, /Прошлый запуск закончился аварийно/);
  assert.match(text, /index out of bounds &lt;x&gt;/);

  const bad = parseTesterReport(JSON.stringify(report({ previousRun: { uncleanExit: "yes", panic: { message: 5 } } })), 0);
  assert.ok(bad, "the rest of the report stays valid");
  assert.equal(bad.previousRun, null);
  assert.equal(parseTesterReport(JSON.stringify(report()), 0).previousRun, null);
});
