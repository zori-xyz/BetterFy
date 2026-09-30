-- Shown to the user in the Telegram approval message so a sign-in request
-- started by someone else is recognizable before it is approved.
ALTER TABLE auth_device_challenges ADD COLUMN client_platform TEXT;
ALTER TABLE auth_device_challenges ADD COLUMN client_version TEXT;
ALTER TABLE auth_device_challenges ADD COLUMN request_country TEXT;
