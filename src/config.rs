use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub update_interval: u64,
    pub story_db: PathBuf,
    // envy bugs out on trying to parse u16 inside a flattened structure
    pub webdriver_host: String,
    pub webdriver_port: u16,
    #[serde(flatten)]
    pub telegram: TelegramOptions,
}

#[derive(Deserialize, Debug)]
pub struct TelegramOptions {
    #[serde(rename = "telegram_secret")]
    pub secret: String,
    #[serde(rename = "telegram_channel")]
    pub channel: String,
    #[serde(rename = "telegram_admin")]
    pub admin: String,
}
