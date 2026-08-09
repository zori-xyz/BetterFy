export type AuthSession = {
  userId: string;
  displayName: string;
  username?: string;
  accessTier: string;
  accessExpiresAt?: number;
  accessPlan?: string;
  accessRecurring?: boolean;
  sessionToken?: string;
  avatarAvailable?: boolean;
  source: "demo" | "server";
};

const authEndpoint = import.meta.env.VITE_BETTERFY_AUTH_URL as string | undefined;
const wait = (duration: number) => new Promise((resolve) => window.setTimeout(resolve, duration));

const isAuthSession = (value: unknown): value is Omit<AuthSession, "source"> => {
  if (!value || typeof value !== "object") return false;
  const record = value as Record<string, unknown>;
  return (
    typeof record.userId === "string"
    && typeof record.displayName === "string"
    && (record.username === undefined || typeof record.username === "string")
    && typeof record.accessTier === "string"
    && (record.accessExpiresAt === undefined || typeof record.accessExpiresAt === "number")
    && (record.accessPlan === undefined || typeof record.accessPlan === "string")
    && (record.accessRecurring === undefined || typeof record.accessRecurring === "boolean")
    && (record.sessionToken === undefined || typeof record.sessionToken === "string")
    && (record.avatarAvailable === undefined || typeof record.avatarAvailable === "boolean")
  );
};

function authenticatedEndpoint(path: string, session: AuthSession) {
  if (!authEndpoint || !session.sessionToken) throw new Error("auth_session_unavailable");
  const endpoint = new URL(path, authEndpoint);
  if (endpoint.protocol !== "https:" && endpoint.hostname !== "127.0.0.1" && endpoint.hostname !== "localhost") {
    throw new Error("auth_endpoint_insecure");
  }
  return endpoint;
}

export async function fetchTelegramAvatar(session: AuthSession): Promise<Blob | null> {
  if (session.source !== "server" || !session.avatarAvailable || !session.sessionToken) return null;
  const response = await fetch(authenticatedEndpoint("/v1/session/avatar", session), {
    headers: { Authorization: `Bearer ${session.sessionToken}` },
    credentials: "omit",
  });
  if (response.status === 404) return null;
  if (!response.ok) throw new Error("auth_avatar_unavailable");
  const blob = await response.blob();
  if (!blob.type.startsWith("image/") || blob.size > 5 * 1024 * 1024) throw new Error("auth_avatar_invalid");
  return blob;
}

export async function revokeAuthSession(session: AuthSession | null): Promise<void> {
  if (!session || session.source !== "server" || !session.sessionToken) return;
  await fetch(authenticatedEndpoint("/v1/session/logout", session), {
    method: "POST",
    headers: { Authorization: `Bearer ${session.sessionToken}` },
    credentials: "omit",
  }).catch(() => undefined);
}

export const authMode: "demo" | "server" = authEndpoint ? "server" : "demo";

export async function verifyTelegramCode(code: string): Promise<AuthSession> {
  if (!/^\d{6}$/.test(code)) throw new Error("auth_code_invalid");

  if (!authEndpoint) {
    await wait(1100);
    if (code === "000000") throw new Error("auth_code_invalid");
    return {
      userId: "demo-zori",
      displayName: "Zori",
      username: "zori",
      accessTier: "Founding Tester",
      source: "demo",
    };
  }

  const endpoint = new URL("/v1/auth/telegram/code", authEndpoint);
  if (endpoint.protocol !== "https:" && endpoint.hostname !== "127.0.0.1" && endpoint.hostname !== "localhost") {
    throw new Error("auth_endpoint_insecure");
  }

  const controller = new AbortController();
  const timeout = window.setTimeout(() => controller.abort(), 10_000);
  try {
    const response = await fetch(endpoint, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ code }),
      credentials: "omit",
      signal: controller.signal,
    });
    if (!response.ok) throw new Error("auth_code_invalid");
    const payload: unknown = await response.json();
    if (!isAuthSession(payload)) throw new Error("auth_response_invalid");
    return { ...payload, source: "server" };
  } finally {
    window.clearTimeout(timeout);
  }
}
