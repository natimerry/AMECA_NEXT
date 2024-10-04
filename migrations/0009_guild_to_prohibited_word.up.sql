-- Add up migration script here
ALTER TABLE prohibited_words_for_guild
    ADD COLUMN guild_id BIGINT;
ALTER TABLE prohibited_words_for_guild
    ADD CONSTRAINT words_prohibited_for_fkey FOREIGN KEY (guild_id) REFERENCES guild (guild_id);