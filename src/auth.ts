export type AuthSession = {
  userId: string;
  displayName: string;
  username?: string;
  accessTier: string;
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
  );
};

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
