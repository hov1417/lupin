use std::path::Path;

use clap::Parser;
use eyre::{Context, Result};
use futures::future::join_all;
use grammers_client::types::Chat;
use grammers_client::Client;

use grammers_tl_types::enums::Dialog;
use tracing::{error, info};

use crate::config::LupinConfig;
use crate::telegram::auth::authenticate;
use crate::telegram::backup::{DialogBackup, DialogType};

use super::message::Message;

#[derive(Debug, Clone, Parser)]
#[clap(alias = "te")]
pub struct LupinTelegramGet {}

impl LupinTelegramGet {
    pub async fn execute(&self, configs: &LupinConfig) -> Result<()> {
        download(configs).await
    }
}

// TODO add logging

// TODO integrate progress bar
async fn download(configs: &LupinConfig) -> Result<()> {
    let client = authenticate(&configs.telegram_config).await?;

    let mut dialog_iter = client.iter_dialogs();
    tokio::fs::create_dir_all(Path::new("backup")).await?;
    let mut tasks = Vec::new();
    while let Some(dialog) = dialog_iter.next().await? {
        if matches!(&dialog.dialog, Dialog::Folder(_)) {
            //TODO make warning
            info!("Dialog Folder: {:?}", dialog.dialog);
            continue;
        }
        match &dialog.chat {
            Chat::User(user) if !user.is_bot() => {
                let task = download_dialog(
                    client.clone(),
                    format!("backup/telegram_user_{}.json", user.id()),
                    user.first_name().to_string(),
                    user.last_name().map(String::from),
                    user.username().map(String::from),
                    dialog.chat,
                    DialogType::User,
                );
                tasks.push(task);
            }
            Chat::Group(group) if !group.is_megagroup() => {
                let task = download_dialog(
                    client.clone(),
                    format!("backup/telegram_group_{}.json", group.id()),
                    group.title().to_string(),
                    None,
                    group.username().map(String::from),
                    dialog.chat,
                    DialogType::Group,
                );
                tasks.push(task);
            }
            _ => {}
        }
    }

    let results = join_all(tasks).await;

    for result in results {
        if let Err(e) = result {
            error!("Error: {}", e);
        }
    }

    Ok(())
}

async fn download_dialog(
    client: Client,
    file: String,
    name: String,
    last_name: Option<String>,
    username: Option<String>,
    chat: Chat,
    dialog_type: DialogType,
) -> Result<()> {
    let messages = download_chat(client, chat)
        .await
        .context("cannot download task")?;
    let backup = DialogBackup {
        name,
        last_name,
        username,
        messages,
        dialog_type,
    };
    tokio::fs::write(
        file,
        serde_json::to_string(&backup).context("cannot serialize backups")?,
    )
    .await
    .context("cannot write to file")
}

async fn download_chat(client: Client, chat: Chat) -> Result<Vec<Message>> {
    let mut message_iter = client.iter_messages(&chat);

    let total = message_iter.total().await?;

    let mut messages = Vec::with_capacity(total);
    while let Some(msg) = message_iter.next().await? {
        messages.push(Message::parse(msg));
        // if let Some(media) = msg.media() {
        //     let dest = format!(
        //         "target/message-{}{}",
        //         &msg.id().to_string(),
        //         get_file_extension(&media)
        //     );
        //     client
        //         .download_media(&media, &Path::new(dest.as_str()))
        //         .await
        //         .expect("Error downloading message");
        // }
    }

    Ok(messages)
}
