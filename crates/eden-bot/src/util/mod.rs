use twilight_model::guild::{Guild, Member, Permissions, Role};
use twilight_model::id::marker::{GuildMarker, RoleMarker};
use twilight_model::id::Id;

pub mod http;
pub mod image;

#[must_use]
pub fn get_guild_member_avatar_url(id: Id<GuildMarker>, member: &Member) -> String {
    let user_id = member.user.id;
    if let Some(hash) = member.avatar {
        self::image::premium_member_avatar(id, user_id, hash)
    } else if let Some(hash) = member.user.avatar {
        self::image::user_avatar(user_id, hash)
    } else {
        self::image::default_user_avatar(user_id)
    }
}

/// Gets the @everyone role from a guild.
pub fn get_everyone_role(guild: &Guild) -> Option<&Role> {
    guild.roles.iter().find(|v| v.name == "@everyone")
}

/// Gets the member's roles (ID only) with their role's permissions.
pub fn get_member_role_perms(
    member_roles: &[Id<RoleMarker>],
    guild_roles: &[Role],
) -> Vec<(Id<RoleMarker>, Permissions)> {
    member_roles
        .iter()
        .map(|role_id| {
            let permissions = guild_roles
                .iter()
                .find(|guild_role| guild_role.id == *role_id)
                .map(|v| v.permissions)
                .unwrap_or_else(Permissions::empty);

            (*role_id, permissions)
        })
        .collect::<Vec<_>>()
}

/// Whether the input permission met the requirement permission unless
/// the input permission has an administrator permission
#[must_use]
pub fn has_permission(input: Permissions, requirement: Permissions) -> bool {
    if input.contains(Permissions::ADMINISTRATOR) {
        true
    } else {
        input.contains(requirement)
    }
}
