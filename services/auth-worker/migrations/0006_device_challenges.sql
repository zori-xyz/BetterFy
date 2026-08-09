PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS auth_device_challenges (
  challenge_hash TEXT PRIMARY KEY,
  device_hash TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending'
    CHECK (status IN ('pending', 'approved', 'denied', 'redeemed')),
  user_id TEXT REFERENCES betterfy_users(user_id) ON DELETE CASCADE,
  created_at INTEGER NOT NULL,
  expires_at INTEGER NOT NULL,
  decided_at INTEGER,
  redeemed_at INTEGER
);

CREATE INDEX IF NOT EXISTS auth_device_challenges_expiry
  ON auth_device_challenges(status, expires_at);

CREATE INDEX IF NOT EXISTS auth_device_challenges_user
  ON auth_device_challenges(user_id, created_at DESC);
