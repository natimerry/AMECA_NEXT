-- Add down migration script here
ALTER table guild
    DROP COLUMN if exists name;

