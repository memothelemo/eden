use eden_utils::Error;
use twilight_model::guild::Permissions;

#[derive(Clone, Copy)]
pub struct CheckPermsInvokerTag {
    pub is_admin: bool,
}

impl CheckPermsInvokerTag {
    fn install_hook() {
        Error::install_hook::<Self>(|_this, _ctx| {});
    }
}

pub struct LackingPermissionsTag(Permissions);

impl LackingPermissionsTag {
    #[must_use]
    pub const fn new(input: Permissions, requirements: Permissions) -> Self {
        Self(get_missing_permissions(input, requirements))
    }

    #[must_use]
    pub const fn calculated(&self) -> Permissions {
        self.0
    }

    fn install_hook() {
        Error::install_hook::<Self>(|this, ctx| {
            ctx.push_body(format!("missing permissions: {:?}", this.0));
        });
    }
}

const fn get_missing_permissions(input: Permissions, requirements: Permissions) -> Permissions {
    requirements.difference(input)
}

pub fn install_hook() {
    LackingPermissionsTag::install_hook();
    CheckPermsInvokerTag::install_hook();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_missing_permissions() {
        let requirements = Permissions::KICK_MEMBERS | Permissions::BAN_MEMBERS;
        let input = Permissions::CREATE_INVITE | Permissions::ADD_REACTIONS;

        let missing = get_missing_permissions(input, requirements);
        assert_eq!(
            missing,
            Permissions::KICK_MEMBERS | Permissions::BAN_MEMBERS
        );

        let requirements = Permissions::KICK_MEMBERS | Permissions::BAN_MEMBERS;
        let input = Permissions::KICK_MEMBERS | Permissions::ADD_REACTIONS;

        let missing = get_missing_permissions(input, requirements);
        assert_eq!(missing, Permissions::BAN_MEMBERS);

        let requirements = Permissions::PRIORITY_SPEAKER | Permissions::CREATE_INVITE;
        let input = Permissions::PRIORITY_SPEAKER | Permissions::CREATE_INVITE;

        let missing = get_missing_permissions(input, requirements);
        assert_eq!(missing, Permissions::empty());
    }
}
