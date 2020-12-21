use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub update_interval: u64,
    pub story_db: PathBuf,
    pub webdriver_host: String,
    pub webdriver_port: u16,
    pub telegram_secret: String,
    pub telegram_channel: String,
    pub telegram_admin: String,
}
