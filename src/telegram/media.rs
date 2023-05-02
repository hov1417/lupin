use grammers_client::types;
use grammers_client::types::Media::{Contact, Document, Photo, Sticker};
use mime::Mime;

pub fn get_file_extension(media: &types::Media) -> String {
    match media {
        Photo(_) => ".jpg".to_string(),
        Sticker(sticker) => get_mime_extension(sticker.document.mime_type()),
        Document(document) => get_mime_extension(document.mime_type()),
        Contact(_) => ".vcf".to_string(),
        _ => String::new(),
    }
}

pub fn get_mime_extension(mime_type: Option<&str>) -> String {
    mime_type
        .map(|m| {
            let mime: Mime = m.parse().unwrap();
            format!(".{}", mime.subtype())
        })
        .unwrap_or(String::new())
}
