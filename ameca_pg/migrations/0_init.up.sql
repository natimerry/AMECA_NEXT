-- Add up migration script here
CREATE TABLE  IF NOT EXISTS guild(
                                     guild_id BIGINT PRIMARY KEY,
                                     members INT,
                                     join_date TIMESTAMP
);

CREATE TABLE IF NOT EXISTS channel(
                                      channel_id BIGINT PRIMARY KEY,
                                      channel_name VARCHAR(255) NOT NULL ,
                                      muted BOOLEAN NOT NULL,
                                      logging_channel BOOLEAN,
                                      guild_id BIGINT REFERENCES guild(guild_id),
                                      CONSTRAINT guild_id__fk
                                          FOREIGN KEY (guild_id) REFERENCES guild(guild_id)
);

create index if not exists channel_guild_idx
    on channel(guild_id);

CREATE TABLE IF NOT EXISTS member(
                                     member_id BIGINT PRIMARY KEY,
                                     admin BOOLEAN,
                                     name VARCHAR(255) NOT NULL,
                                     warnings_issued INT
);

CREATE TABLE IF NOT EXISTS message(
                                      msg_id BIGINT PRIMARY KEY ,
                                      content TEXT,
                                      time TIMESTAMP,
                                      author_id BIGINT REFERENCES member(member_id),
                                      CONSTRAINT author_id__fk
                                          FOREIGN KEY (author_id) REFERENCES member(member_id)
);
create index if not exists __idx_messsage_author
    on message(author_id);


CREATE TABLE IF NOT EXISTS guild_join_member(
                                                guild_id BIGINT REFERENCES guild(guild_id) ON UPDATE CASCADE,
                                                member_id BIGINT REFERENCES member(member_id) ON UPDATE CASCADE,
                                                time TIMESTAMP,
                                                CONSTRAINT member_join_guild_pk PRIMARY KEY (guild_id,member_id)
);

create index if not exists __idx_guild_join_member_guildid
    on guild_join_member(guild_id);
create index if not exists __idx_guild_join_member_memberid
    on guild_join_member(member_id);

CREATE TABLE IF NOT EXISTS banned(
                                     banned_from_guild BIGINT REFERENCES guild(guild_id) NOT NULL ,
                                     banned_user BIGINT REFERENCES member(member_id) NOT NULL ,
                                     issued_by BIGINT REFERENCES member(member_id),
                                     CONSTRAINT banned_guild_user_join PRIMARY KEY (banned_from_guild,banned_user,issued_by)
);
