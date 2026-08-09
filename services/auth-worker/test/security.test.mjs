import test from "node:test";
import assert from "node:assert/strict";
import {
  chooseLanguage,
  constantTimeEqual,
  formatCode,
  generateCode,
  generateSessionToken,
  isFreshCode,
  keyedHash,
  normalizeCode,
} from "../src/security.mjs";

test("normalizes only six digit codes", () => {
  assert.equal(normalizeCode("123 456"), "123456");
  assert.equal(normalizeCode("123-456"), "123456");
  assert.equal(normalizeCode("12345"), null);
  assert.equal(normalizeCode("12345x"), null);
  assert.equal(formatCode("123456"), "123 456");
});

test("session tokens contain 256 bits and use URL-safe encoding", () => {
  const token = generateSessionToken((buffer) => {
    buffer.fill(255);
    return buffer;
  });
  assert.equal(token.length, 43);
  assert.match(token, /^[A-Za-z0-9_-]+$/);
  assert.equal(token.includes("="), false);
});

test("code generation is six digits and supports leading zeroes", () => {
  const code = generateCode((buffer) => {
    buffer[0] = 42;
    return buffer;
  });
  assert.equal(code, "000042");
});

test("keyed hashes are deterministic and do not expose the code", async () => {
  const pepper = "a-secure-test-pepper-that-is-longer-than-32-chars";
  const left = await keyedHash("code:123456", pepper);
  const right = await keyedHash("code:123456", pepper);
  assert.equal(left, right);
  assert.equal(left.length, 64);
  assert.equal(left.includes("123456"), false);
});

test("freshness rejects expired and consumed codes", () => {
  assert.equal(isFreshCode({ expires_at: 101, consumed_at: null }, 100), true);
  assert.equal(isFreshCode({ expires_at: 100, consumed_at: null }, 100), false);
  assert.equal(isFreshCode({ expires_at: 101, consumed_at: 99 }, 100), false);
});

test("language and webhook comparisons are conservative", () => {
  assert.equal(chooseLanguage("ru-RU"), "ru");
  assert.equal(chooseLanguage("en-US"), "en");
  assert.equal(constantTimeEqual("secret", "secret"), true);
  assert.equal(constantTimeEqual("secret", "secrex"), false);
  assert.equal(constantTimeEqual("secret", "secret-long"), false);
});
