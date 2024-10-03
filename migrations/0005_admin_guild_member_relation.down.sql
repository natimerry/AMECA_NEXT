-- Add down migration script here
ALTER table member
    ADD COLUMN admin boolean;

DROP table guild_admins;