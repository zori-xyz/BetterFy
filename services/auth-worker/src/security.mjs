const encoder = new TextEncoder();

export const CODE_TTL_SECONDS = 10 * 60;
export const SESSION_TTL_SECONDS = 12 * 60 * 60;
export const DESKTOP_SESSION_TTL_SECONDS = 15 * 60;
export const REFRESH_FAMILY_TTL_SECONDS = 30 * 24 * 60 * 60;

export function normalizeCode(value) {
  if (typeof value !== "string") return null;
  const code = value.replace(/[\s-]/g, "");
  return /^\d{6}$/.test(code) ? code : null;
}

export function formatCode(code) {
  return `${code.slice(0, 3)} ${code.slice(3)}`;
}

export function chooseLanguage(languageCode) {
  return typeof languageCode === "string" && languageCode.toLowerCase().startsWith("ru")
    ? "ru"
    : "en";
}

export function constantTimeEqual(left, right) {
  const a = encoder.encode(String(left ?? ""));
  const b = encoder.encode(String(right ?? ""));
  let mismatch = a.length ^ b.length;
  const length = Math.max(a.length, b.length);
  for (let index = 0; index < length; index += 1) {
    mismatch |= (a[index] ?? 0) ^ (b[index] ?? 0);
  }
  return mismatch === 0;
}

export function generateCode(random = crypto.getRandomValues.bind(crypto)) {
  const bytes = new Uint32Array(1);
  const ceiling = 0x1_0000_0000 - (0x1_0000_0000 % 1_000_000);
  do {
    random(bytes);
  } while (bytes[0] >= ceiling);
  return String(bytes[0] % 1_000_000).padStart(6, "0");
}

export function generateSessionToken(random = crypto.getRandomValues.bind(crypto)) {
  const bytes = new Uint8Array(32);
  random(bytes);
  return btoa(String.fromCharCode(...bytes))
    .replace(/\+/g, "-")
    .replace(/\//g, "_")
    .replace(/=+$/g, "");
}

export async function keyedHash(value, pepper) {
  if (typeof pepper !== "string" || pepper.length < 32) {
    throw new Error("AUTH_CODE_PEPPER must contain at least 32 characters");
  }
  const key = await crypto.subtle.importKey(
    "raw",
    encoder.encode(pepper),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"],
  );
  const signature = await crypto.subtle.sign("HMAC", key, encoder.encode(value));
  return [...new Uint8Array(signature)]
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

export function isFreshCode(row, nowSeconds) {
  return Boolean(row && row.consumed_at == null && row.expires_at > nowSeconds);
}
