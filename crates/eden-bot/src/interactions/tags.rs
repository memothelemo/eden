use eden_utils::Error;
use serde::{ser::SerializeMap, Serialize};
use twilight_model::guild::Permissions;

#[derive(Clone, Copy)]
pub struct CheckPermsInvokerTag {
    pub is_admin: bool,
}

impl Serialize for CheckPermsInvokerTag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // this is to differentiate various attachments
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("_type", "CHECK_PERMS_INVOKER")?;
        map.serialize_entry("is_admin", &self.is_admin)?;
        map.end()
    }
}

impl CheckPermsInvokerTag {
    fn install_hook() {
        Error::install_serde_hook::<Self>();
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
        Error::install_serde_hook::<Self>();
        Error::install_hook::<Self>(|this, ctx| {
            ctx.push_body(format!("missing permissions: {:?}", this.0));
        });
    }
}

impl Serialize for LackingPermissionsTag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // this is to differentiate various attachments
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("_type", "LACKED_PERMISSIONS")?;
        map.serialize_entry("required", &self.0)?;
        map.end()
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
