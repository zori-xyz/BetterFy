-- Email is an optional second credential for an existing BetterFy user.
-- Existing Telegram identities, entitlements and sessions retain their user_id.
CREATE TABLE IF NOT EXISTS betterfy_email_identities (
  email_hash TEXT PRIMARY KEY,
  user_id TEXT NOT NULL UNIQUE REFERENCES betterfy_users(user_id) ON DELETE CASCADE,
  email_hint TEXT NOT NULL,
  verified_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS betterfy_email_codes (
  code_hash TEXT PRIMARY KEY,
  email_hash TEXT NOT NULL,
  user_id TEXT NOT NULL REFERENCES betterfy_users(user_id) ON DELETE CASCADE,
  purpose TEXT NOT NULL CHECK (purpose IN ('link', 'signin')),
  created_at INTEGER NOT NULL,
  expires_at INTEGER NOT NULL,
  consumed_at INTEGER
);

CREATE INDEX IF NOT EXISTS betterfy_email_codes_lookup
  ON betterfy_email_codes(email_hash, purpose, created_at DESC);
