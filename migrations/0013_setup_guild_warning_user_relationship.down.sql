-- Add down migration script here
DROP TABLE IF EXISTS warnings_guild_member CASCADE ;

ALTER TABLE member ADD COLUMN warnings_issued INT DEFAULT 0;
