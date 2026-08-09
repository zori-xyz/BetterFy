export type AuthSession = {
  userId: string;
  displayName: string;
  username?: string;
  accessTier: string;
  accessExpiresAt?: number;
  accessPlan?: string;
  accessRecurring?: boolean;
  sessionId?: string;
  sessionToken?: string;
  avatarAvailable?: boolean;
  source: "demo" | "server";
};

export type DeviceSession = {
  sessionId: string;
  clientKind: "web" | "desktop" | "unknown";
  createdAt: number;
  lastUsedAt: number;
  expiresAt: number;
  current: boolean;
};

export type DeviceChallengeStart = {
  deepLink: string;
  expiresAt: number;
  pollAfterSeconds: number;
};

export type DeviceChallengePoll = {
  state: "pending" | "confirmed" | "denied" | "expired";
  profile?: AuthSession;
};

const authEndpoint = import.meta.env.VITE_BETTERFY_AUTH_URL as string | undefined;
const wait = (duration: number) => new Promise((resolve) => window.setTimeout(resolve, duration));
const isNativeDesktop = () => "__TAURI_INTERNALS__" in window;

export const supportsDeviceChallenge = () => isNativeDesktop();

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
    && (record.sessionId === undefined || typeof record.sessionId === "string")
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
  if (isNativeDesktop()) {
    const payload = await invoke<{ contentType: string; bytes: number[] } | null>("auth_fetch_avatar");
    return payload ? new Blob([new Uint8Array(payload.bytes)], { type: payload.contentType }) : null;
  }
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
  if (isNativeDesktop()) {
    await invoke("auth_logout");
    return;
  }
  if (!session || session.source !== "server" || !session.sessionToken) return;
  await fetch(authenticatedEndpoint("/v1/session/logout", session), {
    method: "POST",
    headers: { Authorization: `Bearer ${session.sessionToken}` },
    credentials: "omit",
  }).catch(() => undefined);
}

export async function fetchDeviceSessions(session: AuthSession): Promise<DeviceSession[]> {
  if (isNativeDesktop()) return invoke<DeviceSession[]>("auth_list_sessions");
  if (session.source !== "server" || !session.sessionToken) return [];
  const response = await fetch(authenticatedEndpoint("/v1/session/devices", session), {
    headers: { Authorization: `Bearer ${session.sessionToken}` },
    credentials: "omit",
  });
  if (!response.ok) throw new Error("auth_sessions_unavailable");
  const payload: unknown = await response.json();
  if (!payload || typeof payload !== "object" || !Array.isArray((payload as { sessions?: unknown }).sessions)) {
    throw new Error("auth_sessions_invalid");
  }
  return (payload as { sessions: DeviceSession[] }).sessions.filter((device) => (
    device
    && typeof device.sessionId === "string"
    && ["web", "desktop", "unknown"].includes(device.clientKind)
    && typeof device.expiresAt === "number"
    && typeof device.current === "boolean"
  ));
}

export async function revokeDeviceSession(session: AuthSession, sessionId: string): Promise<boolean> {
  if (isNativeDesktop()) return invoke<boolean>("auth_revoke_device", { sessionId });
  if (session.source !== "server" || !session.sessionToken) return false;
  if (!/^(?:[0-9a-f]{32}|[0-9a-f-]{36})$/i.test(sessionId)) throw new Error("auth_session_invalid");
  const response = await fetch(authenticatedEndpoint("/v1/session/devices/revoke", session), {
    method: "POST",
    headers: { Authorization: `Bearer ${session.sessionToken}`, "content-type": "application/json" },
    body: JSON.stringify({ sessionId }),
    credentials: "omit",
  });
  if (!response.ok) throw new Error("auth_session_revoke_failed");
  const payload = await response.json() as { ok?: boolean };
  return payload.ok === true;
}

export const authMode: "demo" | "server" = authEndpoint ? "server" : "demo";

export async function restoreDesktopSession(): Promise<AuthSession | null> {
  if (!isNativeDesktop()) return null;
  const payload = await invoke<Omit<AuthSession, "source"> | null>("auth_restore_session");
  if (!payload) return null;
  if (!isAuthSession(payload)) throw new Error("auth_response_invalid");
  return { ...payload, source: "server" };
}

export async function beginTelegramDeviceChallenge(): Promise<DeviceChallengeStart> {
  if (!isNativeDesktop()) throw new Error("auth_challenge_unsupported");
  const payload = await invoke<DeviceChallengeStart>("auth_begin_device_challenge");
  if (!payload
    || typeof payload.deepLink !== "string"
    || !payload.deepLink.startsWith("https://t.me/BeterFyBot?start=auth_")
    || typeof payload.expiresAt !== "number"
    || typeof payload.pollAfterSeconds !== "number"
    || payload.pollAfterSeconds < 1
    || payload.pollAfterSeconds > 10) {
    throw new Error("auth_response_invalid");
  }
  return payload;
}

export async function pollTelegramDeviceChallenge(): Promise<DeviceChallengePoll> {
  if (!isNativeDesktop()) throw new Error("auth_challenge_unsupported");
  const payload = await invoke<{ state: DeviceChallengePoll["state"]; profile?: unknown }>("auth_poll_device_challenge");
  if (!payload || !["pending", "confirmed", "denied", "expired"].includes(payload.state)) {
    throw new Error("auth_response_invalid");
  }
  if (payload.state === "confirmed") {
    if (!isAuthSession(payload.profile)) throw new Error("auth_response_invalid");
    return { state: "confirmed", profile: { ...payload.profile, source: "server" } };
  }
  return { state: payload.state };
}

export async function cancelTelegramDeviceChallenge(): Promise<void> {
  if (isNativeDesktop()) await invoke("auth_cancel_device_challenge");
}

export async function verifyTelegramCode(code: string): Promise<AuthSession> {
  if (!/^\d{6}$/.test(code)) throw new Error("auth_code_invalid");

  if (isNativeDesktop()) {
    const payload = await invoke<Omit<AuthSession, "source">>("auth_verify_code", { code });
    if (!isAuthSession(payload)) throw new Error("auth_response_invalid");
    return { ...payload, source: "server" };
  }

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
      body: JSON.stringify({ code, clientKind: "web" }),
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
import { invoke } from "@tauri-apps/api/core";
