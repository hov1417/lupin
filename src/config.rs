use crate::Deserialize;

use eyre::Result;
use eyre::{Context, ContextCompat};
use tokio::fs::metadata;

#[derive(Default, Deserialize)]
pub struct TrelloConfig {
    pub auth_cookie: String,
    pub board_ids: Vec<String>,
    pub out_path: String,
}

#[derive(Default, Deserialize)]
pub struct TelegramConfig {
    pub api_id: i32,
    pub app_hash: String,
}

#[derive(Default, Deserialize)]
pub struct LupinConfig {
    #[serde(default)]
    pub trello_config: TrelloConfig,
    #[serde(default)]
    pub telegram_config: TelegramConfig,
}

pub async fn get_configs() -> Result<LupinConfig> {
    let config = dirs::home_dir()
        .context("cannot get home dir")?
        .join(".config")
        .join("lupin")
        .join("lupin.yml");
    let data = tokio::fs::read(config).await?;
    Ok(serde_yaml::from_slice(&data)?)
}

pub async fn save_telegram_token(token: Vec<u8>) -> Result<()> {
    let path = dirs::home_dir()
        .context("cannot get home dir")?
        .join(".config")
        .join("lupin")
        .join("telegram_token");
    tokio::fs::write(path, token)
        .await
        .context("cannot save token")
}

pub async fn get_telegram_token() -> Result<Option<Vec<u8>>> {
    let home = dirs::home_dir()
        .context("cannot get home dir")?
        .join(".config")
        .join("lupin")
        .join("telegram_token");
    if metadata(&home).await.is_err() {
        return Ok(None);
    }

    let data = tokio::fs::read(home).await?;
    Ok(Some(data))
}
