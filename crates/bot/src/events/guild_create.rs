use eden_db::forms::InsertAdminForm;
use eden_db::schema::Admin;
use eden_tasks::Scheduled;
use eden_utils::error::{AnyResultExt, ResultExt};
use eden_utils::Result;
use futures::{FutureExt, TryFutureExt};
use std::future::IntoFuture;
use thiserror::Error;
use twilight_model::{
    guild::{Guild, Permissions},
    id::{marker::RoleMarker, Id},
    user::User,
};

use crate::{tasks, ShardContext};

#[derive(Debug, Error)]
#[error("could not fetch local guild members")]
struct FetchMembersError;

#[derive(Debug, Error)]
#[error("could not upsert local guild admins into the database")]
struct UpsertAdminsError;

#[allow(clippy::expect_used)]
#[tracing::instrument(skip_all, fields(
    %guild.id,
    guild.members = %guild.member_count.unwrap_or_default(),
))]
pub async fn handle(shard: &ShardContext, guild: Guild) -> Result<()> {
    let is_local = shard.bot.settings.bot.guild.id == guild.id;
    let span = tracing::Span::current();
    if !span.is_disabled() {
        span.record("guild.is_local", is_local);
    }

    if !is_local {
        return Ok(());
    }

    tracing::debug!("found local guild. fetching local guild administrators");

    let admins = fetch_local_guild_admins(&guild, shard).await?;
    let total_admins = admins.len();

    tracing::debug!(
        "found {total_admins} local guild administrator(s). upserting them into the database",
    );

    if let Err(error) = upsert_admins(shard, &admins).await {
        tracing::warn!(%error, "unable to upsert admins. scheduling to upsert local guild admins later");

        let task = tasks::UpsertLocalGuildAdmins {
            entries: admins.into_iter().map(|v| (v.id, v.name)).collect(),
        };

        // maybe we're rate limited, so give it around 2 minutes
        let queue = &shard.bot.queue;
        queue
            .schedule(task, Scheduled::in_minutes(2))
            .await
            .attach_printable("could not schedule task")?;
    }
    tracing::info!("initialized {total_admins} local guild administrator(s)");

    Ok(())
}

#[tracing::instrument(skip_all, fields(admins = %admins.len()))]
async fn upsert_admins(shard: &ShardContext, admins: &[User]) -> Result<()> {
    let mut conn = shard
        .bot
        .db_transaction()
        .await
        .transform_context(UpsertAdminsError)?;

    for admin in admins {
        upsert_admin(&mut conn, &admin).await?;
    }

    conn.commit()
        .await
        .change_context(UpsertAdminsError)
        .attach_printable("could not commit transaction into the database")?;

    Ok(())
}

#[tracing::instrument(skip_all, fields(%admin.id))]
async fn upsert_admin(conn: &mut sqlx::PgConnection, admin: &User) -> Result<()> {
    let form = InsertAdminForm::builder()
        .id(admin.id)
        .name(Some(admin.name.as_str()))
        .build();

    Admin::upsert(conn, form)
        .await
        .change_context(UpsertAdminsError)
        .attach_printable("could not upsert admin")?;

    Ok(())
}

#[tracing::instrument(skip_all)]
async fn fetch_local_guild_admins(guild: &Guild, shard: &ShardContext) -> Result<Vec<User>> {
    let mut after = None;
    let mut admins = Vec::new();

    let everyone_role = guild
        .roles
        .iter()
        .find(|v| v.name == "@everyone")
        .map(|v| v.permissions)
        .unwrap_or_else(Permissions::empty);

    loop {
        const PAGE_SIZE: u16 = 500;

        let mut request = shard
            .bot
            .http
            .guild_members(guild.id)
            .limit(PAGE_SIZE)
            .expect("unexpected error when setting limit to PAGE_SIZE");

        tracing::debug!(?after, "fetching guild members after {after:?}");
        if let Some(after) = after.as_ref() {
            request = request.after(*after);
        }

        let members = request
            .into_future()
            .map(|v| v.anonymize_error())
            .and_then(|v| v.model().map(|v| v.anonymize_error()))
            .await
            .transform_context(FetchMembersError)?;

        for member in members.iter() {
            let roles = member.roles.iter().map(|v| {
                let perms = guild
                    .roles
                    .iter()
                    .find(|n| n.id == *v)
                    .map(|v| v.permissions)
                    .unwrap_or_else(Permissions::empty);

                (*v, perms)
            });
            let roles = roles.collect::<Vec<_>>();
            let user = &member.user;
            if is_real_administrator(&guild, &user, everyone_role, &roles) {
                tracing::debug!(%user.id, "found admin ({}) of local guild", user.id);
                admins.push(user.clone());
            }
        }

        if members.len() != (PAGE_SIZE as usize) {
            tracing::debug!("fetch stop");
            break;
        }
        after = members.iter().last().map(|v| v.user.id);
    }

    Ok(admins)
}

fn is_real_administrator(
    guild: &Guild,
    user: &User,
    everyone_role: Permissions,
    member_roles: &[(Id<RoleMarker>, Permissions)],
) -> bool {
    use twilight_util::permission_calculator::PermissionCalculator;

    // Bots do not classify as admin members unfortunately
    let calculator = PermissionCalculator::new(guild.id, user.id, everyone_role, member_roles);
    let is_administrator = calculator.root().contains(Permissions::ADMINISTRATOR);
    let is_not_a_bot = !user.bot;

    is_administrator && is_not_a_bot
}
