use grammers_client::types::media::Document;
use grammers_client::types::Media;
use mime::Mime;

pub fn get_file_extension(media: &Media) -> String {
    match media {
        Media::Photo(_) => ".jpg".to_string(),
        Media::Sticker(sticker) => {
            get_mime_extension(sticker.document.mime_type()).unwrap_or_default()
        }
        Media::Document(document) => get_document_suffix(document),
        Media::Contact(_) => ".vcf".to_string(),
        _ => String::new(),
    }
}

fn get_document_suffix(document: &Document) -> String {
    get_mime_extension(document.mime_type())
        .unwrap_or_else(|| format!("-{}", document.name()))
}

pub fn get_mime_extension(mime_type: Option<&str>) -> Option<String> {
    mime_type.map(|m| {
        let mime: Mime = m.parse().unwrap();
        format!(".{}", mime.subtype())
    })
}
