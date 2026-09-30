-- BetterFy ID credentials. The existing Telegram-backed user key remains stable.
-- Standalone IDs use a non-Telegram namespaced value in the legacy identity column;
-- bot and avatar operations must only accept numeric Telegram identities.
CREATE TABLE IF NOT EXISTS betterfy_id_credentials (
  user_id TEXT PRIMARY KEY REFERENCES betterfy_users(user_id) ON DELETE CASCADE,
  username_key TEXT NOT NULL UNIQUE,
  password_hash TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS betterfy_id_registrations (
  email_hash TEXT PRIMARY KEY,
  username_key TEXT NOT NULL UNIQUE,
  password_hash TEXT NOT NULL,
  code_hash TEXT NOT NULL,
  email_hint TEXT NOT NULL,
  language TEXT NOT NULL CHECK (language IN ('ru', 'en')),
  created_at INTEGER NOT NULL,
  expires_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS betterfy_id_registrations_expiry ON betterfy_id_registrations(expires_at);
