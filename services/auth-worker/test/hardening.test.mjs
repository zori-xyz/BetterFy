import test from "node:test";
import assert from "node:assert/strict";
import { rateLimitSubject } from "../src/security.mjs";
import {
  CLEANUP_STATEMENTS,
  deviceChallengeContext,
  deviceMatchCode,
  deviceRequestCaption,
  isTelegramIdentity,
  refreshReuseAllowed,
  verifyGlobalFailureLimit,
} from "../src/index.mjs";

const pepper = "p".repeat(32);

test("IPv6 clients share one rate-limit bucket per /64", () => {
  assert.equal(rateLimitSubject("2001:db8:1:2:aaaa::1"), "2001:db8:1:2::/64");
  assert.equal(rateLimitSubject("2001:0db8:0001:0002:ffff:ffff:ffff:ffff"), "2001:db8:1:2::/64");
  assert.equal(rateLimitSubject("2001:db8::1"), "2001:db8:0:0::/64");
  assert.equal(rateLimitSubject("203.0.113.9"), "203.0.113.9");
  assert.equal(rateLimitSubject("::ffff:203.0.113.9"), "203.0.113.9");
  assert.equal(rateLimitSubject(null), "unknown");
  assert.equal(rateLimitSubject("not:an:address"), "unknown");
});

test("the global verify failure budget has a safe default", () => {
  assert.equal(verifyGlobalFailureLimit({}), 60);
  assert.equal(verifyGlobalFailureLimit({ VERIFY_GLOBAL_FAILURE_LIMIT: "25" }), 25);
  assert.equal(verifyGlobalFailureLimit({ VERIFY_GLOBAL_FAILURE_LIMIT: "-1" }), 60);
});

test("a lost refresh response may be retried once, briefly", () => {
  const used = { used_at: 1000, revoked_at: null, replaced_by_hash: "next" };
  assert.equal(refreshReuseAllowed(used, 1030), true);
  assert.equal(refreshReuseAllowed(used, 1061), false);
  assert.equal(refreshReuseAllowed({ ...used, revoked_at: 1010 }, 1030), false);
  assert.equal(refreshReuseAllowed({ ...used, replaced_by_hash: null }, 1030), false);
  assert.equal(refreshReuseAllowed({ used_at: null, revoked_at: null }, 1030), false);
});

test("device context accepts only known, bounded values", () => {
  assert.deepEqual(deviceChallengeContext({ platform: "windows", appVersion: "0.1.4" }, "RU"), {
    platform: "windows", version: "0.1.4", country: "RU",
  });
  assert.deepEqual(deviceChallengeContext({ platform: "__proto__", appVersion: "<b>1</b>" }, "XX1"), {
    platform: null, version: null, country: null,
  });
});

test("the approval message names the device and repeats the match number", async () => {
  const code = await deviceMatchCode("t".repeat(43), pepper);
  assert.match(code, /^[1-9][0-9]$/);
  assert.equal(code, await deviceMatchCode("t".repeat(43), pepper));
  const caption = deviceRequestCaption("ru", {
    created_at: 0, client_platform: "windows", client_version: "0.1.4", request_country: "RU",
  }, code);
  assert.match(caption, /Windows · BetterFy 0\.1\.4/);
  assert.match(caption, /RU/);
  assert.ok(caption.includes(code));
  const legacy = deviceRequestCaption("en", { created_at: 0 }, null);
  assert.match(legacy, /unknown/);
  assert.doesNotMatch(legacy, /Number in the BetterFy window/);
});

test("cleanup never touches payment or identity records", () => {
  for (const statement of CLEANUP_STATEMENTS) {
    assert.doesNotMatch(statement, /payment|entitlement|betterfy_users|identities|credentials/);
  }
});

test("only numeric Telegram ids count as a linked Telegram account", () => {
  assert.equal(isTelegramIdentity("123456789"), true);
  assert.equal(isTelegramIdentity("id:2f1c0e5a-0000-4000-8000-000000000000"), false);
  assert.equal(isTelegramIdentity(undefined), false);
});

test("sign-in emails carry the code in HTML and plain text, per language and purpose", async () => {
  const { emailContent } = await import("../src/email.mjs");
  const ru = emailContent("123456", "ru", "register");
  assert.equal(ru.subject, "Подтверди почту для BetterFy ID");
  assert.ok(ru.html.includes("123 456") && ru.text.includes("123 456"));
  assert.ok(ru.html.includes('lang="ru"'));
  const en = emailContent("654321", "en", "signin");
  assert.equal(en.subject, "Your BetterFy ID sign-in code");
  assert.ok(en.html.includes("654 321"));
  assert.throws(() => emailContent("<b>1</b>", "en", "signin"));
});
