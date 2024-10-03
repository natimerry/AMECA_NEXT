-- Add up migration script here
ALTER TABLE deleted
    RENAME TO deleted_messages;

ALTER TABLE deleted_messages
    ADD COLUMN guild_id BIGINT references guild(guild_id);


ALTER TABLE banned
    RENAME TO banned_member_guild_relation;

ALTER TABLE banned_words
    RENAME TO prohibited_words_for_guild;

