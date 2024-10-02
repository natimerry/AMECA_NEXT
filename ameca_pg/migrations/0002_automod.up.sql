-- Add up migration script here

CREATE TABLE IF NOT EXISTS banned_words
(
    id      SERIAL PRIMARY KEY,
    name    VARCHAR(255),
    pattern VARCHAR(255),
    author BIGINT references member(member_id)
);

create index __author_banned_words_idx
    on banned_words (author);