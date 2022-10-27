mod compression;
mod config;
mod get_board;

use crate::compression::compress_directory;
use crate::config::get_configs;
use crate::get_board::get_boards;
use clap::Parser;
use eyre::Result;
use serde::Deserialize;
use std::path::PathBuf;
use tokio::fs::create_dir_all;

#[derive(Debug, Parser)]
pub struct LupinGet {}

#[derive(Debug, Parser)]
#[clap(author, version, about = "Cli tool to load data from trello")]
pub enum Lupin {
    Get(LupinGet),
}

#[tokio::main]
async fn main() -> Result<()> {
    let command: Lupin = Lupin::parse();

    let config = get_configs().await?;

    match command {
        Lupin::Get(_) => {
            let temp_dir = temp_dir::TempDir::new()?;
            get_boards(&config.board_ids, temp_dir.path(), &config.auth_cookie)
                .await?;
            create_dir_all(&config.out_path).await?;
            compress_directory(
                temp_dir.path().to_path_buf(),
                PathBuf::from(config.out_path).join(format!(
                    "{}.tar.zst",
                    chrono::Local::today().format("%Y-%m-%d")
                )),
            )
            .await?;
        }
    }

    Ok(())
}
