use twilight_model::id::marker::GuildMarker;
use twilight_model::id::{marker::UserMarker, Id};
use twilight_model::util::ImageHash;

// https://discord.com/developers/docs/reference#image-formatting-image-base-url
pub const CDN_BASE_URL: &str = "https://cdn.discordapp.com";

fn get_ext(hash: &ImageHash) -> &'static str {
    if hash.is_animated() {
        "gif"
    } else {
        "png"
    }
}

/// Gets the Discord CDN endpoint for the user's default avatar image.
#[must_use]
pub fn default_user_avatar(id: Id<UserMarker>) -> String {
    let index = (id.get() >> 22) % 6;
    format!("{CDN_BASE_URL}/embed/avatars/{index}.png")
}

/// Gets the Discord CDN endpoint for the guild member's avatar
/// or premium avatar image.
#[must_use]
pub fn premium_member_avatar(
    guild_id: Id<GuildMarker>,
    user_id: Id<UserMarker>,
    hash: ImageHash,
) -> String {
    let ext = get_ext(&hash);
    format!("{CDN_BASE_URL}/guilds/{guild_id}/users/{user_id}/avatars/{hash}.{ext}")
}

/// Gets the Discord CDN endpoint for the user's avatar image.
#[must_use]
pub fn user_avatar(id: Id<UserMarker>, hash: ImageHash) -> String {
    let ext = get_ext(&hash);
    format!("{CDN_BASE_URL}/avatars/{id}/{hash}.{ext}")
}
