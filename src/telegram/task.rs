use std::io;
use std::io::{BufRead, Write};
use std::path::Path;

use clap::Parser;
use eyre::{Context, Result};
use futures::future::join_all;
use grammers_client::types::Chat;
use grammers_client::{Client, Config, SignInError};
use grammers_session::Session;
use grammers_tl_types::enums::Dialog;

use crate::config::LupinConfig;
use crate::telegram::backup::ChatBackup;

use super::message::Message;

#[derive(Debug, Clone, Parser)]
#[clap(alias = "te")]
pub struct LupinTelegramGet {}

impl LupinTelegramGet {
    pub async fn execute(&self, configs: &LupinConfig) -> Result<()> {
        download(configs).await
    }
}

// TODO move to configs
const SESSION_FILE: &str = "downloader.session";

// TODO add logging

// TODO integrate progress bar
async fn download(configs: &LupinConfig) -> Result<()> {
    let api_id = configs.telegram_config.app_id;
    let api_hash = configs.telegram_config.app_hash.clone();

    println!("Connecting to Telegram...");
    let client = Client::connect(Config {
        session: Session::load_file_or_create(SESSION_FILE)?,
        api_id,
        api_hash: api_hash.clone(),
        params: Default::default(),
    })
    .await?;
    println!("Connected!");

    authenticate(api_id, &api_hash, &client).await?;

    let mut dialog_iter = client.iter_dialogs();
    tokio::fs::create_dir_all(Path::new("backup")).await?;
    let mut tasks = Vec::new();
    while let Some(dialog) = dialog_iter.next().await? {
        match (dialog.dialog, &dialog.chat) {
            (Dialog::Dialog(d), Chat::User(user)) if !user.is_bot() => {
                let client = client.clone();
                let file = format!("backup/telegram_{}.json", user.id());
                let first_name = user.first_name().to_string();
                let last_name = user.last_name().map(String::from);
                let username = user.username().map(String::from);
                let task = async move {
                    let messages = download_chat(client, dialog.chat)
                        .await
                        .context("cannot download task")?;
                    let backup = ChatBackup {
                        first_name,
                        last_name,
                        username,
                        messages,
                    };
                    tokio::fs::write(
                        file,
                        serde_json::to_string(&backup)
                            .context("cannot serialize backups")?,
                    )
                    .await
                    .context("cannot write to file")
                };
                tasks.push(task);
            }
            (Dialog::Dialog(d), Chat::Group(group))
                if !group.is_megagroup() =>
            {
                // TODO pack groups
                println!("Group: {:?}", group.title());
            }
            (Dialog::Folder(d), _) => {
                //TODO make warning
                println!("Dialog Folder: {:?}", d);
            }
            _ => {}
        }
    }

    let results = join_all(tasks).await;

    for result in results {
        if let Err(e) = result {
            println!("Error: {}", e);
        }
    }

    Ok(())
}

async fn authenticate(
    api_id: i32,
    api_hash: &String,
    client: &Client,
) -> Result<()> {
    if !client.is_authorized().await? {
        println!("Signing in...");
        let phone = prompt("Enter your phone number (international format): ")?;
        let token =
            client.request_login_code(&phone, api_id, &api_hash).await?;
        let code = prompt("Enter the code you received: ")?;
        let signed_in = client.sign_in(&token, &code).await;
        match signed_in {
            Err(SignInError::PasswordRequired(password_token)) => {
                // Note: this `prompt` method will echo the password in the console.
                //       Real code might want to use a better way to handle this.
                let hint = password_token.hint().unwrap();
                let prompt_message =
                    format!("Enter the password (hint {}): ", &hint);
                let password = prompt(prompt_message.as_str())?;

                client
                    .check_password(password_token, password.trim())
                    .await?;
            }
            Ok(_) => (),
            Err(e) => panic!("{}", e),
        };
        println!("Signed in!");
        client
            .session()
            .save_to_file(SESSION_FILE)
            .context("failed to save the session")?;
    };
    Ok(())
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

fn prompt(message: &str) -> Result<String> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    stdout.write_all(message.as_bytes())?;
    stdout.flush()?;

    let stdin = io::stdin();
    let mut stdin = stdin.lock();

    let mut line = String::new();
    stdin.read_line(&mut line)?;
    Ok(line)
}
