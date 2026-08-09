PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS betterfy_users (
  user_id TEXT PRIMARY KEY,
  telegram_user_id TEXT NOT NULL UNIQUE,
  display_name TEXT NOT NULL,
  username TEXT,
  language TEXT NOT NULL CHECK (language IN ('ru', 'en')),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS auth_codes (
  code_hash TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES betterfy_users(user_id) ON DELETE CASCADE,
  created_at INTEGER NOT NULL,
  expires_at INTEGER NOT NULL,
  consumed_at INTEGER
);

CREATE INDEX IF NOT EXISTS auth_codes_user_created
  ON auth_codes(user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS auth_codes_expiry
  ON auth_codes(expires_at);

CREATE TABLE IF NOT EXISTS auth_rate_limits (
  bucket_hash TEXT PRIMARY KEY,
  window_started_at INTEGER NOT NULL,
  request_count INTEGER NOT NULL
);
