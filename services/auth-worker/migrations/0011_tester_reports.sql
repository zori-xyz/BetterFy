-- Reports from the tester program. Only counts are kept here; the notes,
-- pictures and recorded events go to the founder's Telegram chat and nowhere
-- else.
CREATE TABLE tester_reports (
  report_id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES betterfy_users(user_id) ON DELETE CASCADE,
  created_at INTEGER NOT NULL,
  app_version TEXT NOT NULL,
  event_count INTEGER NOT NULL,
  failure_count INTEGER NOT NULL,
  note_count INTEGER NOT NULL,
  image_count INTEGER NOT NULL,
  delivered INTEGER NOT NULL
);

CREATE INDEX tester_reports_by_user ON tester_reports (user_id, created_at);
