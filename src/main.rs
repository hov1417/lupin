use clap::{Parser, Subcommand};
use eyre::Result;
use serde::Deserialize;

use crate::config::get_configs;
use crate::telegram::task::LupinTelegramGet;
use crate::trello::task::LupinTrelloGet;

mod compression;
mod config;
mod progress;
mod telegram;
mod trello;

#[derive(Debug, Clone, Parser)]
#[command(author, version, about = "Cli tool to load data from trello")]
pub struct Lupin {
    #[command(subcommand)]
    command: LupinSubcommand,
}

#[derive(Debug, Clone, Subcommand)]
enum LupinSubcommand {
    Telegram(LupinTelegramGet),
    Trello(LupinTrelloGet),
}

#[tokio::main]
async fn main() -> Result<()> {
    let command: Lupin = Lupin::parse();

    let config = get_configs().await?;

    match command.command {
        LupinSubcommand::Trello(arg) => {
            arg.execute(&config).await?;
        }
        LupinSubcommand::Telegram(arg) => {
            arg.execute().await?;
        }
    }

    Ok(())
}
