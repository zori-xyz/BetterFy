PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS payment_orders (
  order_id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES betterfy_users(user_id) ON DELETE CASCADE,
  sku TEXT NOT NULL,
  invoice_payload TEXT NOT NULL UNIQUE,
  currency TEXT NOT NULL CHECK (currency = 'XTR'),
  amount INTEGER NOT NULL CHECK (amount BETWEEN 1 AND 10000),
  status TEXT NOT NULL CHECK (status IN ('created', 'precheckout', 'paid', 'refunded')),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS payment_orders_user_created
  ON payment_orders(user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS star_payment_events (
  telegram_charge_id TEXT PRIMARY KEY,
  order_id TEXT NOT NULL REFERENCES payment_orders(order_id) ON DELETE RESTRICT,
  user_id TEXT NOT NULL REFERENCES betterfy_users(user_id) ON DELETE RESTRICT,
  amount INTEGER NOT NULL,
  currency TEXT NOT NULL CHECK (currency = 'XTR'),
  paid_at INTEGER NOT NULL,
  expires_at INTEGER NOT NULL,
  is_recurring INTEGER NOT NULL CHECK (is_recurring IN (0, 1)),
  is_first_recurring INTEGER NOT NULL CHECK (is_first_recurring IN (0, 1)),
  refunded_at INTEGER,
  canceled_at INTEGER
);

CREATE INDEX IF NOT EXISTS star_events_user_expiry
  ON star_payment_events(user_id, expires_at DESC);

CREATE TABLE IF NOT EXISTS entitlements (
  user_id TEXT NOT NULL REFERENCES betterfy_users(user_id) ON DELETE CASCADE,
  entitlement_key TEXT NOT NULL,
  active_until INTEGER NOT NULL,
  source TEXT NOT NULL CHECK (source = 'telegram_stars'),
  source_charge_id TEXT NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (user_id, entitlement_key)
);
