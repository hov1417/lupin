use clap::{Parser, Subcommand};
use eyre::Result;
use indicatif::MultiProgress;
use serde::Deserialize;

use crate::config::get_configs;
use crate::logging::init_trace_logger;
use crate::telegram::task::LupinTelegramGet;
use crate::trello::task::LupinTrelloGet;

mod compression;
mod config;
mod logging;
mod progress;
mod telegram;
mod trello;

#[derive(Debug, Clone, Parser)]
#[command(author, version, about = "Cli tool to load data from trello")]
pub struct Lupin {
    #[command(subcommand)]
    command: LupinSubcommand,
    #[arg(
        long,
        short = 'v',
        action = clap::ArgAction::Count,
        global = true,
        help = "More output per occurrence",
    )]
    verbose: u8,

    #[arg(
        long,
        short = 'q',
        action = clap::ArgAction::Count,
        global = true,
        help = "Less output per occurrence",
        conflicts_with = "verbose",
    )]
    quiet: u8,
}

#[derive(Debug, Clone, Subcommand)]
enum LupinSubcommand {
    Telegram(LupinTelegramGet),
    Trello(LupinTrelloGet),
}

#[tokio::main]
async fn main() -> Result<()> {
    let command: Lupin = Lupin::parse();
    let mpb = MultiProgress::new();

    let _guard = init_trace_logger(mpb.clone(), command.verbose, command.quiet).await?;

    let config = get_configs().await?;

    match command.command {
        LupinSubcommand::Trello(arg) => {
            arg.execute(&config, mpb).await?;
        }
        LupinSubcommand::Telegram(arg) => {
            arg.execute(&config).await?;
        }
    }

    Ok(())
}
