-- Add up migration script here
ALTER TABLE channel ADD COLUMN automod_exempt BOOLEAN DEFAULT (false) ;
