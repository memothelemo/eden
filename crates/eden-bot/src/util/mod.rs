use twilight_model::guild::{Guild, Member, Permissions, Role};
use twilight_model::id::marker::RoleMarker;
use twilight_model::id::Id;

pub mod http;

/// Gets the @everyone role from a guild.
pub fn get_everyone_role(guild: &Guild) -> Option<&Role> {
    guild.roles.iter().find(|v| v.name == "@everyone")
}

/// Gets the member's roles (ID only) with their role's permissions.
pub fn get_member_role_perms(
    member: &Member,
    guild_roles: &[Role],
) -> Vec<(Id<RoleMarker>, Permissions)> {
    member
        .roles
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
