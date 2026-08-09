PRAGMA foreign_keys = ON;

ALTER TABLE betterfy_users ADD COLUMN avatar_file_id TEXT;
ALTER TABLE betterfy_users ADD COLUMN avatar_checked_at INTEGER;

CREATE TABLE IF NOT EXISTS auth_sessions (
  session_hash TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES betterfy_users(user_id) ON DELETE CASCADE,
  created_at INTEGER NOT NULL,
  expires_at INTEGER NOT NULL,
  last_used_at INTEGER NOT NULL,
  revoked_at INTEGER
);

CREATE INDEX IF NOT EXISTS auth_sessions_user_created
  ON auth_sessions(user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS auth_sessions_expiry
  ON auth_sessions(expires_at);
