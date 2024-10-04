-- Add down migration script here
ALTER table prohibited_words_for_guild DROP CONSTRAINT words_prohibited_for_fkey;
ALTER table prohibited_words_for_guild DROP COLUMN guild_id;