use tracing::log::{info, trace};
use crate::BoxResult;
use poise::serenity_prelude::{GuildId, MessageId, RoleId};
use sqlx::FromRow;
use crate::bot::AMECA;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Role {
    pub id: i32,
    pub emoji: String,
    pub roles_id: i64,
    pub msg_id: i64,
    pub guild_id: i64,
    pub name: String,
}
pub trait RoleData {
    async fn new_reaction_role(
        db: &AMECA,
        msg_id: MessageId,
        role_id: RoleId,
        guild_id: GuildId,
        name: String,
        emoji: String,
    ) -> BoxResult<()>;
}

impl RoleData for Role {
    async fn new_reaction_role(
        db: &AMECA,
        msg_id: MessageId,
        role_id: RoleId,
        guild_id: GuildId,
        name: String,
        emoji: String,
    ) -> BoxResult<()> {
        info!("Setting up new role_reaction relationship");
        let msg_id = msg_id.get() as i64;
        let role_id = role_id.get() as i64;
        let guild_id = guild_id.get() as i64;
        #[derive(FromRow)]
        #[derive(Debug)]
struct SomeShit{
            id: i32
        }
        let shit: SomeShit = sqlx::query_as(
            "INSERT INTO reaction_role(roles_id, name, msg_id,emoji,guild_id) VALUES ($1, $2, $3,$4,$5) RETURNING id").bind(role_id)
            .bind(name.clone())
            .bind(msg_id)
            .bind(&emoji)
            .bind(guild_id)
            .fetch_one(&db.db)
            .await?;

        trace!("Reaction_role query insert res {shit:#?}");

        let role = Role {
            name,
            id: shit.id,
            roles_id: role_id,
            emoji,
            msg_id,
            guild_id,
        };
        trace!("Role obj {:#?}",role);

        db.watch_msgs
            .entry(guild_id)
            .and_modify(|list| list.push(role.clone()))
            .or_insert(vec![role]);
        Ok(())
    }
}
