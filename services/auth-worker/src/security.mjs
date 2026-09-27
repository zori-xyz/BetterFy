const encoder = new TextEncoder();

export const CODE_TTL_SECONDS = 10 * 60;
export const SESSION_TTL_SECONDS = 12 * 60 * 60;
export const DESKTOP_SESSION_TTL_SECONDS = 15 * 60;
export const REFRESH_FAMILY_TTL_SECONDS = 30 * 24 * 60 * 60;
export const DEVICE_CHALLENGE_TTL_SECONDS = 10 * 60;

// One IPv6 subscriber usually controls a whole /64, so a per-address bucket
// would give a single attacker billions of independent rate limits.
export function rateLimitSubject(address) {
  if (typeof address !== "string" || address.length === 0 || address.length > 64) return "unknown";
  if (!address.includes(":")) return address;
  if (address.includes(".")) return address.slice(address.lastIndexOf(":") + 1);
  const [head, tail = ""] = address.toLowerCase().split("::");
  const left = head ? head.split(":") : [];
  const right = tail ? tail.split(":") : [];
  if (address.includes("::")) {
    const missing = 8 - left.length - right.length;
    if (missing < 0) return "unknown";
    left.push(...Array(missing).fill("0"), ...right);
  }
  if (left.length !== 8 || left.some((group) => !/^[0-9a-f]{1,4}$/.test(group))) return "unknown";
  return `${left.slice(0, 4).map((group) => group.replace(/^0+(?=.)/, "")).join(":")}::/64`;
}

export function normalizeDeviceId(value) {
  return typeof value === "string" && /^[A-Za-z0-9_-]{43}$/.test(value) ? value : null;
}

export function normalizeChallengeToken(value) {
  return typeof value === "string" && /^[A-Za-z0-9_-]{43}$/.test(value) ? value : null;
}

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
