-- Add up migration script here
ALTER TABLE afk_member_guild ADD COLUMN previous_name VARCHAR(255);
