-- Add up migration script here

ALTER TABLE IF EXISTS afk_member_guild
    ADD COLUMN IF NOT EXISTS reason TEXT;