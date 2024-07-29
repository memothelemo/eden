use thiserror::Error;

#[derive(Debug, Error)]
#[error("Could not load Eden settings")]
pub struct SettingsLoadError;

#[derive(Debug, Error)]
#[error("Eden bot failed")]
pub struct StartBotError;
