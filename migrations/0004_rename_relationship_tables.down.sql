-- Add down migration script here
ALTER TABLE deleted_messages
    RENAME TO deleted;

ALTER TABLE deleted DROP COLUMN guild_id;


ALTER TABLE banned_member_guild_relation
    RENAME TO banned;

ALTER TABLE prohibited_words_for_guild
    RENAME TO banned_words;