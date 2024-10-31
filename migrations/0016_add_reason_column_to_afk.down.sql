-- Add down migration script here
ALTER TABLE IF EXISTS afk_member_guild
    DROP COLUMN IF EXISTS reason;
