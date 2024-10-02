-- Add down migration script here
DROP TABLE IF EXISTS banned_words CASCADE ;
DROP INDEX IF EXISTS __author_banned_words_idx;