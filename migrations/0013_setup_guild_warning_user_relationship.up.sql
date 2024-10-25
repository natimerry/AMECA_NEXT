ALTER TABLE IF EXISTS member
    DROP COLUMN IF EXISTS warnings_issued;

CREATE TABLE IF NOT EXISTS warnings_guild_member
(
    guild_id  BIGINT references guild (guild_id),
    member_id BIGINT references member (member_id),
    id        BIGSERIAL PRIMARY KEY
);
create index idx_warning_guilds
    on warnings_guild_member(guild_id);

create index idx_warning_members
    on warnings_guild_member(member_id);