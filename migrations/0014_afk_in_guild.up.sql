-- Add up migration script here
CREATE TABLE IF NOT EXISTS afk_member_guild
(
    id        BIGSERIAL PRIMARY KEY,
    member_id BIGINT REFERENCES member (member_id),
    guild_id  BIGINT REFERENCES guild (guild_id),
    time_afk timestamptz
);

CREATE INDEX IF NOT EXISTS __idx_afk_member_id on afk_member_guild (member_id);
CREATE INDEX IF NOT EXISTS __idx_afk_guild_id on afk_member_guild (guild_id);