INSERT INTO guild (guild_id,members,join_date,name) VALUES ($1,$2,$3::TIMESTAMPTZ,$4)
            ON CONFLICT(guild_id) DO UPDATE SET
            join_date=excluded.join_date,
            name = excluded.name,
            members = excluded.members;