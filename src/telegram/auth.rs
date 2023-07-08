use crate::config::{get_telegram_token, save_telegram_token, TelegramConfig};
use eyre::{bail, Context, Result};
use grammers_client::{Client, Config, SignInError};
use grammers_session::Session;
use std::io;
use std::io::{BufRead, Write};
use tracing::info;

pub async fn authenticate(telegram_config: &TelegramConfig) -> Result<Client> {
    let api_id = telegram_config.api_id;
    let api_hash = telegram_config.app_hash.clone();

    let session = if let Some(token) = get_telegram_token().await? {
        Session::load(token.as_slice()).context("cannot load token")?
    } else {
        Session::new()
    };

    let client = Client::connect(Config {
        session,
        api_id,
        api_hash: api_hash.clone(),
        params: Default::default(),
    })
    .await?;

    if client.is_authorized().await? {
        return Ok(client);
    }
    info!("Signing in to telegram ...");
    let phone = prompt("Enter your phone number (international format): ")?;
    let token = client.request_login_code(&phone).await?;
    let code = prompt("Enter the code you received: ")?;
    let signed_in = client.sign_in(&token, &code).await;
    match signed_in {
        Err(SignInError::PasswordRequired(password_token)) => {
            let hint = if let Some(hint) = password_token.hint() {
                format!(" (hint {})", &hint)
            } else {
                String::new()
            };
            let password = rpassword::prompt_password(format!(
                "Enter the password{hint}: "
            ))?;

            client
                .check_password(password_token, password.trim())
                .await?;
        }
        Ok(_) => {}
        Err(e) => bail!("Sign in error: {}", e),
    };
    info!("Signed in!");
    save_telegram_token(client.session().save())
        .await
        .context("failed to save the session")?;
    Ok(client)
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
