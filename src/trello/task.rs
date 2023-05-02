use std::path::PathBuf;

use clap::Parser;
use tokio::fs::create_dir_all;

use crate::compression::compress_directory;
use crate::config::LupinConfig;
use crate::trello::boards::get_boards;

#[derive(Debug, Clone, Parser)]
#[clap(alias = "tr")]
pub struct LupinTrelloGet {}

impl LupinTrelloGet {
    pub async fn execute(&self, config: &LupinConfig) -> eyre::Result<()> {
        // TODO remove temp_dir, directly compress to out_path
        let temp_dir = temp_dir::TempDir::new()?;
        get_boards(
            &config.trello_config.board_ids,
            temp_dir.path(),
            &config.trello_config.auth_cookie,
        )
        .await?;
        create_dir_all(&config.trello_config.out_path).await?;
        let out_path = PathBuf::from(&config.trello_config.out_path).join(
            format!("{}.tar.zst", chrono::Local::now().format("%Y-%m-%d")),
        );
        compress_directory(temp_dir.path().to_path_buf(), out_path).await?;
        Ok(())
    }
}
