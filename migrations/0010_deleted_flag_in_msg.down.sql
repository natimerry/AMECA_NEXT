-- Add down migration script here
ALTER TABLE message DROP column if exists deleted;
