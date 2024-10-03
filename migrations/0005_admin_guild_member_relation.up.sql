-- Add up migration script here
ALTER TABLE member
    DROP COLUMN admin;

CREATE TABLE guild_admins(
    admin_id BIGINT REFERENCES member(member_id),
    guild BIGINT REFERENCES guild(guild_id),
    CONSTRAINT __pk_guild_admin_member_guild_relation PRIMARY KEY (admin_id,guild)
);

