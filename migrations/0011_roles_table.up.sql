-- Add up migration script here
CREATE TABLE IF NOT EXISTS reaction_role
(
    id       SERIAL PRIMARY KEY,
    roles_id BIGINT,
    name     VARCHAR(255),
    emoji    VARCHAR(255),
    msg_id   BIGINT references message (msg_id),
    guild_id BIGINT references guild(guild_id)
);