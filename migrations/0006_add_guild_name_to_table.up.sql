-- Add up migration script here
ALTER TABLE guild
    ADD COLUMN name VARCHAR(1024);