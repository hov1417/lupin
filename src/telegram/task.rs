use std::any::Any;
use std::io;
use std::io::Error;
use std::path::Path;

use clap::Parser;
use eyre::{Context, Result};
use futures::future::join_all;
use grammers_client::client::chats::InvocationError;
use grammers_client::types::{Chat, Downloadable, Media};
use grammers_client::Client;

use grammers_tl_types::enums::Dialog;
use tracing::{error, warn};

use crate::config::LupinConfig;
use crate::telegram::auth::authenticate;
use crate::telegram::backup::{DialogBackup, DialogType};
use crate::telegram::media::get_file_extension;

use super::message::Message;

#[derive(Debug, Clone, Parser)]
#[clap(alias = "tg")]
pub struct LupinTelegramGet;

impl LupinTelegramGet {
    pub async fn execute(&self, configs: &LupinConfig) -> Result<()> {
        download(configs).await
    }
}

// TODO add logging

// TODO integrate progress bar
async fn download(configs: &LupinConfig) -> Result<()> {
    let client = authenticate(&configs.telegram_config).await?;

    let mut dialog_iter =
        client.iter_dialogs().limit(10 /* TODO: remove this */);
    tokio::fs::create_dir_all(Path::new("backup")).await?;
    let mut tasks = Vec::new();
    while let Some(dialog) = dialog_iter.next().await? {
        if matches!(&dialog.dialog, Dialog::Folder(_)) {
            warn!("Dialog Folder: {:?}", dialog.dialog);
            continue;
        }
        match &dialog.chat {
            Chat::User(user) if !user.is_bot() => {
                let task = download_dialog(
                    client.clone(),
                    format!("backup/telegram_user_{}", user.id()),
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
                    format!("backup/telegram_group_{}", group.id()),
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
    path: String,
    name: String,
    last_name: Option<String>,
    username: Option<String>,
    chat: Chat,
    dialog_type: DialogType,
) -> Result<()> {
    let dir = Path::new(&path);

    tokio::fs::create_dir(dir).await?;

    let file = dir.join("metadata.json");
    let messages = download_chat(client, chat, &path)
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

async fn download_chat(
    client: Client,
    chat: Chat,
    path: &str,
) -> Result<Vec<Message>> {
    let mut message_iter = client.iter_messages(&chat);

    let total = message_iter.total().await?;

    let mut message_iter = message_iter;
    let mut messages = Vec::with_capacity(total);
    while let Some(msg) = message_iter.next().await? {
        messages.push(Message::parse(&msg));
        if let Some(media) = msg.media() {
            let dest = format!(
                "{path}/message-{}{}",
                msg.id(),
                get_file_extension(&media)
            );
            if matches!(media, Media::Photo(_) | Media::Document(_)) {
                let download_res = client
                    .download_media(
                        &Downloadable::Media(media),
                        &Path::new(dest.as_str()),
                    )
                    .await;
                match download_res {
                    Ok(_) => {}
                    Err(e) if e.kind() == io::ErrorKind::Other => {
                        let inner_err = e.into_inner();
                        if let Some(err_data) = inner_err {
                            warn!("Error downloading message: {:?}", err_data);
                            let rpc_err =
                                err_data.downcast_ref::<InvocationError>();
                            if let Some(InvocationError::Rpc(rpc_err)) = rpc_err
                            {
                                warn!("Error downloading downcast: {:?}", rpc_err);
                                if rpc_err.is("FLOOD_WAIT") {
                                    tokio::time::sleep(
                                        std::time::Duration::from_secs(
                                            rpc_err.value.unwrap_or(3) as u64,
                                        ),
                                    )
                                    .await;
                                }
                            }
                        }
                    }
                    Err(e) => return Err(e.into()),
                }
            }
        }
    }

    Ok(messages)
}
