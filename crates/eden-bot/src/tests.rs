use eden_settings::{Bot, Database, LocalGuild, Settings};
use eden_utils::error::exts::*;
use eden_utils::error::tags::Suggestion;
use eden_utils::types::Sensitive;
use twilight_model::id::Id;

pub fn generate_real_settings() -> Settings {
    match Settings::from_env() {
        Ok(n) => n,
        Err(error) => {
            eden_utils::Error::init();
            let error = error.anonymize().attach(Suggestion::new(
                "Make sure to configure your Eden development environment before running tests",
            ));
            panic!("Cannot load settings: {error}");
        }
    }
}

pub fn generate_fake_settings() -> Settings {
    Settings::builder()
        .bot(
            Bot::builder()
                .local_guild(
                    LocalGuild::builder()
                        .id(Id::new(273534239310479360))
                        .build(),
                )
                .token("a test token")
                .build(),
        )
        .database(
            Database::builder()
                .url(Sensitive::new("postgres://test".try_into().unwrap()))
                .build(),
        )
        .build()
}
