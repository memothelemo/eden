#![feature(let_chains)]
use config::{Config, ConfigBuilder};
use doku::Document;
use eden_utils::build;
use eden_utils::env::var_opt_parsed;
use eden_utils::error::exts::AnonymizedResultExt;
use eden_utils::error::exts::IntoTypedError;
use eden_utils::error::exts::ResultExt;
use eden_utils::error::tags::Suggestion;
use eden_utils::Result as EdenResult;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use typed_builder::TypedBuilder;

mod bot;
mod database;
mod error;
mod logging;

pub use self::bot::*;
pub use self::database::*;
pub use self::logging::*;

pub use self::error::SettingsLoadError;
pub use eden_tasks::Settings as Worker;

#[derive(Debug, Document, Deserialize, TypedBuilder)]
pub struct Settings {
    pub bot: Bot,
    pub database: Database,

    #[builder(default)]
    #[serde(default)]
    pub logging: Logging,

    #[builder(default)]
    #[serde(default)]
    pub worker: Worker,

    #[builder(setter(skip), default = None)]
    #[serde(skip)]
    #[doku(skip)]
    path: Option<PathBuf>,

    /// How many CPU threads which Eden will utilize.
    ///
    /// The good rule of thumb when setting the amount of CPU threads
    /// is ideally you want to have it at least 2 cores (one for the gateway
    /// and one for the task queueing system).
    ///
    /// Unless, if you want to start Eden instance with many shards to or your bot
    /// needs to cater a lot of members in your guild/server, you may want to adjust
    /// it up to 4 to 8.
    ///
    /// The default if not set is the total actual amount of your CPU cores
    /// divided by 2 (spare for the operating system). If the CPU however, is a single
    /// core, it will utilize one core only.
    #[builder(default = Settings::default_workers())]
    #[doku(example = "2")]
    #[serde(default = "Settings::default_workers")]
    pub threads: usize,
}

impl Settings {
    pub fn from_env() -> EdenResult<Self, SettingsLoadError> {
        let mut builder = Config::builder().add_source(
            config::Environment::with_prefix("EDEN")
                .prefix_separator("_")
                .separator("_")
                .convert_case(config::Case::Snake),
        );

        let resolved_path = Self::resolve_path()?;
        if let Some(resolved_path) = resolved_path.as_ref() {
            // this is to enforce users to use yaml instead
            let source: config::File<config::FileSourceFile, config::FileFormat> =
                resolved_path.clone().into();

            builder = builder.add_source(source.format(config::FileFormat::Toml));
        }

        let builder = Self::resolve_alternative_vars(builder)
            .change_context(SettingsLoadError)
            .attach_printable("could not resolve settings path")?;

        let mut settings: Settings = builder
            .build()
            .into_typed_error()
            .change_context(SettingsLoadError)
            .and_then(|v| {
                v.try_deserialize()
                    .into_typed_error()
                    .change_context(SettingsLoadError)
            })
            .attach_printable_lazy(|| format!("using settings file: {resolved_path:?}"))?;

        settings.path = resolved_path;
        Ok(settings)
    }

    const ALTERNATIVE_FILE_PATHS: &[&'static str] = &[
        "eden.toml",
        #[cfg(windows)]
        "%USERPROFILE%/.eden/settings.toml",
        // these are only applicable in Unix systems
        #[cfg(target_family = "unix")]
        "/etc/eden/settings.toml",
    ];

    pub fn resolve_path() -> EdenResult<Option<PathBuf>, SettingsLoadError> {
        // EDEN_SETTINGS
        let mut resolved_path = var_opt_parsed::<PathBuf>("EDEN_SETTINGS")
            .change_context(SettingsLoadError)
            .attach(Suggestion::new("`EDEN_SETTINGS` must be a valid path"))?;

        // Try to load from alternative paths
        for base_path in Self::ALTERNATIVE_FILE_PATHS {
            // No need to iterate all alternative file paths if `resolved_path`
            // is defined from either the `EDEN_SETTINGS` variable or loop itself.
            let is_resolved_exists = resolved_path
                .as_ref()
                .map(|v| v.exists())
                .unwrap_or_default();

            if is_resolved_exists {
                break;
            }

            // Try resolving settings path by loading from the current directory's
            // descendants if one of the alternative file path is not locating to
            // the exact location
            let base_path = PathBuf::from(base_path);
            if base_path.is_relative() && !base_path.exists() {
                let mut parent = std::env::current_dir().ok();
                'descendant_search: while let Some(descendant) = parent.take() {
                    let absolute = descendant.join(&base_path);
                    if absolute.exists() {
                        resolved_path = Some(absolute);
                        break 'descendant_search;
                    }

                    parent = descendant.parent().map(|v| v.to_path_buf());
                }
            } else if base_path.is_absolute() {
                let file_exists = std::fs::metadata(&base_path)
                    .map(|v| v.is_file())
                    .unwrap_or(false);

                if file_exists {
                    resolved_path = Some(resolved_path.unwrap_or_else(|| PathBuf::from(base_path)));
                    break;
                }
            }
        }

        Ok(resolved_path)
    }

    /// Generates TOML data with default values of [`Settings`] and
    /// documentation using [`doku`].
    #[must_use]
    pub fn generate_docs() -> String {
        use doku::toml::{EnumsStyle, Spacing};

        let fmt = doku::toml::Formatting {
            spacing: Spacing {
                lines_between_scalar_field_comments: 1,
                lines_between_scalar_fields: 0,
                ..Default::default()
            },
            enums_style: EnumsStyle::Commented,
            ..Default::default()
        };

        doku::to_toml_fmt::<Self>(&fmt)
    }
}

impl Settings {
    /// Current working path for the [`Settings`] file.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
}

impl Settings {
    fn default_workers() -> usize {
        (num_cpus::get_physical() / 2).max(1)
    }

    fn resolve_alternative_vars(
        mut builder: ConfigBuilder<config::builder::DefaultState>,
    ) -> EdenResult<ConfigBuilder<config::builder::DefaultState>> {
        // `DATABASE_URL` is used for testing environments but this statement
        // will be disabled on release.
        if build::PROFILE != "release"
            && let Some(value) = eden_utils::env::var_opt("DATABASE_URL")?
        {
            builder = builder
                .set_override("database.url", value)
                .into_typed_error()
                .attach_printable("could not override settings for DATABASE_URL")?;
        }

        // Some people configure their Discord bot token with environment variables:
        // "DISCORD_BOT_TOKEN", "BOT_TOKEN", "TOKEN", and so forth.
        let alt_token = eden_utils::env::var_opt("TOKEN")
            .or(eden_utils::env::var_opt("BOT_TOKEN"))
            .or(eden_utils::env::var_opt("DISCORD_BOT_TOKEN"))?;

        if let Some(token) = alt_token {
            // Just in case if `EDEN_BOT_TOKEN` or `bot.token` is actually missing
            builder = builder
                .set_default("bot.token", token)
                .into_typed_error()
                .attach_printable("could not override settings for bot token")?;
        }

        // `RUST_LOG` usage
        if let Some(value) = eden_utils::env::var_opt("RUST_LOG")? {
            builder = builder
                .set_override("logging.targets", value)
                .into_typed_error()
                .attach_printable("could not override settings for RUST_LOG")?;
        }

        Ok(builder)
    }
}
