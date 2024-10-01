use serenity::all::User;
use sqlx::{FromRow, PgPool};
use std::future::Future;
use log::info;
use tracing::debug;
use crate::BoxResult;

#[derive(Debug, FromRow)]
pub struct Members {
    pub member_id: i64,
    pub admin: bool,
    pub name: String, // real name
    pub warnings_issued: i32,
}

pub trait MemberData {
    fn new_user(
        db: &PgPool,
        user: serenity::all::User,
    ) -> impl std::future::Future<Output = BoxResult<()>> + Send;
}

impl MemberData for PgPool {
    async fn new_user(db: &PgPool, user: User) -> BoxResult<()>{
        let user_id = user.id.get() as i64;
        let name = user.name;
        info!("Inserting new user {} into database",&name);
        let _user = sqlx::query!(
            "INSERT INTO member(member_id,name,admin,warnings_issued) VALUES($1,$2,$3,$4);",
            user_id,
            name,
            false,
            0
        ).execute(db).await?;
        debug!("User insertion query result: {:?}", _user);
        Ok(())
    }
}
