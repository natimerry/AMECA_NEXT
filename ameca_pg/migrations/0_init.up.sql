-- Add up migration script here
CREATE TABLE  IF NOT EXISTS guild(
                                     guild_id INT PRIMARY KEY,
                                     members INT,
                                     join_date TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS channel(
                                      channel_id INT PRIMARY KEY,
                                      channel_name VARCHAR(255) NOT NULL ,
                                      muted BOOLEAN NOT NULL,
                                      logging_channel BOOLEAN,
                                      guild_id INT REFERENCES guild(guild_id),
                                      CONSTRAINT guild_id__fk
                                          FOREIGN KEY (guild_id) REFERENCES guild(guild_id)
);

create index if not exists channel_guild_idx
    on channel(guild_id);

CREATE TABLE IF NOT EXISTS member(
                                     member_id INT PRIMARY KEY,
                                     admin BOOLEAN,
                                     name VARCHAR(255) NOT NULL,
                                     warnings_issuesd INT
);

CREATE TABLE IF NOT EXISTS message(
                                      msg_id INT PRIMARY KEY ,
                                      content TEXT,
                                      time TIMESTAMPTZ,
                                      author_id INT REFERENCES member(member_id),
                                      CONSTRAINT author_id__fk
                                          FOREIGN KEY (author_id) REFERENCES member(member_id)
);
create index if not exists __idx_messsage_author
    on message(author_id);


CREATE TABLE IF NOT EXISTS guild_join_member(
                                                guild_id INT REFERENCES guild(guild_id) ON UPDATE CASCADE,
                                                member_id INT REFERENCES member(member_id) ON UPDATE CASCADE,
                                                time timestamptz,
                                                CONSTRAINT member_join_guild_pk PRIMARY KEY (guild_id,member_id)
);

create index if not exists __idx_guild_join_member_guildid
    on guild_join_member(guild_id);
create index if not exists __idx_guild_join_member_memberid
    on guild_join_member(member_id);

CREATE TABLE IF NOT EXISTS banned(
                                     banned_from_guild INT REFERENCES guild(guild_id) NOT NULL ,
                                     banned_user INT REFERENCES member(member_id) NOT NULL ,
                                     issued_by INT REFERENCES member(member_id),
                                     CONSTRAINT banned_guild_user_join PRIMARY KEY (banned_from_guild,banned_user,issued_by)
);