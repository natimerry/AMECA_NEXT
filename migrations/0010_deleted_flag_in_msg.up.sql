-- Add up migration script here
ALTER TABLE message ADD column if not exists deleted boolean default false;