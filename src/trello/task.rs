use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use indicatif::MultiProgress;
use tokio::fs::create_dir_all;
use tokio::sync::Mutex;

use crate::compression::{compress_directory, Compressor};
use crate::config::LupinConfig;
use crate::trello::boards::get_boards;

#[derive(Debug, Clone, Parser)]
#[clap(alias = "tl")]
pub struct LupinTrelloGet;

impl LupinTrelloGet {
    pub async fn execute(
        &self,
        config: &LupinConfig,
        mpb: MultiProgress,
    ) -> eyre::Result<()> {
        // TODO remove temp_dir, directly compress to out_path
        let temp_dir = temp_dir::TempDir::new()?;
        let out_path = PathBuf::from(&config.trello_config.out_path).join(
            format!("{}.tar.zst", chrono::Local::now().format("%Y-%m-%d")),
        );
        let compressor =
            Arc::new(Mutex::new(Compressor::new(&out_path).await?));
        get_boards(
            &mpb,
            &config.trello_config.board_ids,
            move |(p, data)| {
                let compressor = compressor.clone();
                let p = p.to_string();
                async move {
                    compressor.lock().await.add_file_with_data(&p, &data).await
                }
            },
            temp_dir.path(),
            &config.trello_config.auth_cookie,
        )
        .await?;
        create_dir_all(&config.trello_config.out_path).await?;
        compress_directory(temp_dir.path().to_path_buf(), out_path).await?;
        Ok(())
    }
}
