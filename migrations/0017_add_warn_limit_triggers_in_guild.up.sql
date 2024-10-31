-- Add up migration script here
CREATE TABLE IF NOT EXISTS warn_triggers_guild
(
    id              BIGSERIAL,
    guild_id        BIGINT references guild (guild_id),
    action          text,
    number_of_warns INT
);

CREATE INDEX IF NOT EXISTS __idx_warn_trigger_guild ON warn_triggers_guild(guild_id);


