use crate::Deserialize;
use eyre::ContextCompat;
use eyre::Result;

#[derive(Default, Deserialize)]
pub struct TrelloConfig {
    pub auth_cookie: String,
    pub board_ids: Vec<String>,
    pub out_path: String,
}

#[derive(Default, Deserialize)]
pub struct TelegramConfig {
    pub app_id: i32,
    pub app_hash: String,
    // TODO
}

#[derive(Default, Deserialize)]
pub struct LupinConfig {
    #[serde(default)]
    pub trello_config: TrelloConfig,
    #[serde(default)]
    pub telegram_config: TelegramConfig,
}

pub async fn get_configs() -> Result<LupinConfig> {
    let home = dirs::home_dir()
        .context("cannot get home dir")?
        .join(".config")
        .join("lupin.yml");
    let data = tokio::fs::read(home).await?;
    Ok(serde_yaml::from_slice(&data)?)
}
