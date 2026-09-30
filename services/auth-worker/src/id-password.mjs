// New hashes use ITERATIONS. The Workers Free plan allows about 10 ms of CPU
// per request, and 600k PBKDF2-SHA256 rounds take far longer, so the count is
// sized to fit. The secret pepper (an HMAC key outside the database) is what
// keeps a leaked database from being cracked offline. The count is stored in
// each hash, so it can be raised later without invalidating existing ones.
export const ITERATIONS = 20_000;
const MIN_ITERATIONS = 20_000;
const MAX_ITERATIONS = 1_000_000;
const encoder = new TextEncoder();

export function normalizeIdUsername(value) {
  if (typeof value !== "string" || value !== value.trim() || !/^[a-zA-Z][a-zA-Z0-9_]{2,23}$/.test(value)) return null;
  return value.toLowerCase();
}

export function validIdPassword(value) {
  if (typeof value !== "string" || /[\u0000-\u001f\u007f]/.test(value)) return false;
  const byteLength = encoder.encode(value).length;
  return byteLength >= 12 && byteLength <= 128;
}

const toHex = (bytes) => Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
const fromHex = (value) => Uint8Array.from(value.match(/.{2}/g) ?? [], (byte) => Number.parseInt(byte, 16));

async function pepperPassword(password, pepper) {
  if (typeof pepper !== "string" || pepper.length < 32) throw new Error("password_pepper_unavailable");
  const key = await crypto.subtle.importKey("raw", encoder.encode(pepper), { name: "HMAC", hash: "SHA-256" }, false, ["sign"]);
  return new Uint8Array(await crypto.subtle.sign("HMAC", key, encoder.encode(password)));
}

export async function hashIdPassword(password, pepper, saltHex = null, iterations = ITERATIONS) {
  if (!validIdPassword(password)) throw new Error("invalid_password");
  const salt = saltHex ? fromHex(saltHex) : crypto.getRandomValues(new Uint8Array(16));
  if (salt.length !== 16) throw new Error("invalid_salt");
  const input = await pepperPassword(password, pepper);
  const key = await crypto.subtle.importKey("raw", input, "PBKDF2", false, ["deriveBits"]);
  const digest = new Uint8Array(await crypto.subtle.deriveBits({ name: "PBKDF2", hash: "SHA-256", salt, iterations }, key, 256));
  return `pbkdf2-sha256$${iterations}$${toHex(salt)}$${toHex(digest)}`;
}

export async function verifyIdPassword(password, pepper, encoded) {
  const parts = typeof encoded === "string" ? encoded.split("$") : [];
  if (parts.length !== 4 || parts[0] !== "pbkdf2-sha256" || !/^[0-9]{1,7}$/.test(parts[1]) || Number(parts[1]) < MIN_ITERATIONS || Number(parts[1]) > MAX_ITERATIONS
    || !/^[0-9a-f]{32}$/.test(parts[2]) || !/^[0-9a-f]{64}$/.test(parts[3]) || !validIdPassword(password)) return false;
  const candidate = await hashIdPassword(password, pepper, parts[2], Number(parts[1]));
  const actual = fromHex(parts[3]);
  const expected = fromHex(candidate.split("$")[3]);
  if (actual.length !== expected.length) return false;
  let difference = 0;
  for (let index = 0; index < actual.length; index += 1) difference |= actual[index] ^ expected[index];
  return difference === 0;
}
