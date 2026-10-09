-- The Premium end date a reminder was already sent for. A renewal moves
-- active_until, so the next period gets its own reminder.
ALTER TABLE entitlements ADD COLUMN reminded_until INTEGER;
