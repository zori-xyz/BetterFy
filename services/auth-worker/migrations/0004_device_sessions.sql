PRAGMA foreign_keys = ON;

ALTER TABLE auth_sessions ADD COLUMN session_id TEXT;
ALTER TABLE auth_sessions ADD COLUMN client_kind TEXT;

UPDATE auth_sessions
SET session_id = lower(hex(randomblob(16))), client_kind = 'unknown'
WHERE session_id IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS auth_sessions_public_id
  ON auth_sessions(session_id);

CREATE INDEX IF NOT EXISTS auth_sessions_user_active
  ON auth_sessions(user_id, revoked_at, expires_at DESC);
