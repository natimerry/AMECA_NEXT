-- Add down migration script here
CREATE TABLE IF NOT EXISTS banned_member_guild_relation
(
    banned_from_guild BIGINT REFERENCES guild (guild_id)   NOT NULL,
    banned_user       BIGINT REFERENCES member (member_id) NOT NULL,
    issued_by         BIGINT REFERENCES member (member_id),
    CONSTRAINT banned_guild_user_join PRIMARY KEY (banned_from_guild, banned_user, issued_by)
);
