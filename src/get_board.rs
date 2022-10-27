use std::path::{Path, PathBuf};

use bytes::buf::Buf;
use bytes::Bytes;
use eyre::Result;
use futures::future::try_join_all;
use futures::{Stream, TryStreamExt};
use itertools::Itertools;
use reqwest::header;
use reqwest::header::HeaderMap;
use serde::Deserialize;
use tokio::fs;
use tokio::io::AsyncWriteExt;

pub async fn get_boards(
    board_ids: &[String],
    out_path: &Path,
    auth_cookie: &str,
) -> Result<()> {
    let load_boards = board_ids
        .iter()
        .map(|id| async {
            let board = get_board(id, auth_cookie).await?;

            let board_info: Board = serde_json::from_str(&board)?;

            fs::create_dir_all(out_path).await?;
            let path = PathBuf::from(out_path)
                .join(format!("{}.json", board_info.name));
            let mut file = fs::File::create(path).await?;
            file.write_all(board.as_bytes()).await?;
            download_attachments(board_info, out_path, auth_cookie).await?;

            Ok::<(), eyre::Report>(())
        })
        .collect_vec();

    try_join_all(load_boards).await?;

    Ok(())
}

#[derive(Deserialize)]
struct Board<'a> {
    name: &'a str,
    cards: Vec<Card<'a>>,
}

#[derive(Deserialize)]
struct Card<'a> {
    #[serde(borrow)]
    attachments: Vec<Attachment<'a>>,
}

#[derive(Deserialize)]
struct Attachment<'a> {
    name: &'a str,
    url: &'a str,
}

pub async fn get_board(id: &str, cookie: &str) -> Result<String> {
    let headers = get_headers(cookie)?;

    let client = reqwest::Client::new();
    let res = client
        .get(format!("https://trello.com/b/{id}.json"))
        .headers(headers)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    Ok(res)
}

async fn download_attachments(
    board_info: Board<'_>,
    out_path: &Path,
    cookie: &str,
) -> Result<()> {
    let downloads = board_info
        .cards
        .into_iter()
        .flat_map(|c| {
            c.attachments
                .into_iter()
                .filter(|a| !a.name.starts_with("http"))
        })
        .map(|attachment| async move {
            let mut data = get_attachment(&attachment, cookie).await?;

            let path = PathBuf::from(out_path).join(board_info.name);
            fs::create_dir_all(&path).await?;
            let mut file = fs::File::create(path.join(attachment.name)).await?;
            while let Some(bytes) = data.try_next().await? {
                file.write_all(bytes.chunk()).await?;
            }

            Ok::<(), eyre::Report>(())
        });

    try_join_all(downloads).await?;
    Ok(())
}

async fn get_attachment(
    attachment: &Attachment<'_>,
    cookie: &str,
) -> Result<impl Stream<Item = reqwest::Result<Bytes>>> {
    let headers = get_headers(cookie)?;

    let client = reqwest::Client::new();
    let res = client
        .get(attachment.url)
        .headers(headers)
        .send()
        .await?
        .bytes_stream();

    Ok(res)
}

fn get_headers(cookie: &str) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:106.0) Gecko/20100101 Firefox/106.0".parse()?);
    headers.insert("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8".parse()?);
    headers.insert("Accept-Language", "en-US,en;q=0.5".parse()?);
    headers.insert("Accept-Encoding", "gzip, deflate, br".parse()?);
    headers.insert("DNT", "1".parse()?);
    headers.insert("Connection", "keep-alive".parse()?);
    headers.insert(header::COOKIE, cookie.parse()?);
    headers.insert("Upgrade-Insecure-Requests", "1".parse()?);
    headers.insert("Sec-Fetch-Dest", "document".parse()?);
    headers.insert("Sec-Fetch-Mode", "navigate".parse()?);
    headers.insert("Sec-Fetch-Site", "same-origin".parse()?);
    headers.insert("Sec-Fetch-User", "?1".parse()?);
    headers.insert("Sec-GPC", "1".parse()?);
    headers.insert("TE", "trailers".parse()?);

    Ok(headers)
}
