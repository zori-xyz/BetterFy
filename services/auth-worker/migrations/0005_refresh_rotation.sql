PRAGMA foreign_keys = ON;

ALTER TABLE auth_sessions ADD COLUMN family_id TEXT;

CREATE TABLE IF NOT EXISTS auth_refresh_tokens (
  token_hash TEXT PRIMARY KEY,
  family_id TEXT NOT NULL,
  user_id TEXT NOT NULL REFERENCES betterfy_users(user_id) ON DELETE CASCADE,
  generation INTEGER NOT NULL CHECK (generation >= 0),
  created_at INTEGER NOT NULL,
  expires_at INTEGER NOT NULL,
  used_at INTEGER,
  revoked_at INTEGER,
  replaced_by_hash TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS auth_refresh_family_generation
  ON auth_refresh_tokens(family_id, generation);

CREATE INDEX IF NOT EXISTS auth_refresh_user_active
  ON auth_refresh_tokens(user_id, revoked_at, expires_at DESC);

CREATE INDEX IF NOT EXISTS auth_sessions_family
  ON auth_sessions(family_id, revoked_at);
