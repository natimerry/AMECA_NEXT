create index if not exists idx_prohibited_words_by_guild
    on prohibited_words_for_guild(guild_id);

create index if not exists idx_roles_guild_id
    on reaction_role(guild_id);

create index if not exists idx_roles_name
    on reaction_role(name);

create index if not exists idx_roles_guild
    on reaction_role(guild_id);

