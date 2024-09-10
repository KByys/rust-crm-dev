pub mod cache;
pub mod dser;
pub mod headers;
pub mod lazy;
pub mod time;
pub use dser::deserialize_any_to_bool;
use axum::extract::Multipart;
use base64::prelude::Engine;

use crate::Response;

pub use self::time::{TimeFormat, TIME};
/// base64 url safe encode
pub fn base64_encode(input: impl AsRef<[u8]>) -> String {
    base64::prelude::BASE64_URL_SAFE_NO_PAD.encode(input)
}

/// base64 url safe decode
pub fn base64_decode(input: impl AsRef<[u8]>) -> Result<Vec<u8>, base64::DecodeError> {
    base64::prelude::BASE64_URL_SAFE_NO_PAD.decode(input)
}
pub struct FilePart {
    pub bytes: Vec<u8>,
    pub filename: Option<String>,
    pub content_type: Option<String>,
}
impl FilePart {
    pub fn filename(&self) -> &str {
        self.filename.as_deref().unwrap_or("unknown.jpg")
    }
}
pub struct MessagePart {
    pub files: Vec<FilePart>,
    pub json: String,
}

pub async fn parse_multipart(mut part: Multipart) -> Result<MessagePart, Response> {
    let mut files = Vec::new();
    let mut data = String::new();
    while let Some(field) = part.next_field().await? {
        match field.name() {
            Some("file") => {
                let filename = field.file_name().map(|s| s.to_owned());
                println!("{:?}", filename);
                let content_type = field.content_type().map(|s| s.to_string());
                let chunk = field.bytes().await?.to_vec();
                files.push(FilePart {
                    bytes: chunk,
                    filename,
                    content_type,
                });
            }
            Some("data") => {
                data = field.text().await?;
            }
            _ => (),
        }
    }
    Ok(MessagePart { files, json: data })
}

pub fn gen_id(time: &TIME, name: &str) -> String {
    base64_encode(format!(
        "{}-{}-{}",
        name,
        time.naos() / 10000,
        rand::random::<u8>()
    ))
}

pub fn gen_file_link(time: &TIME, name: &str) -> String {
    base64_encode(format!(
        "{}\0{}?{}{}",
        name,
        time.naos() / 10000,
        rand::random::<u16>(),
        rand::random::<u16>()
    ))
}
#[test]
fn test() {
    let time = TIME::now().unwrap();
    let link = gen_file_link(&time, "test.png");
    println!("{:?}", link);
    let parse = parse_file_link(&link);
    println!("{:?}", parse.unwrap());
}

pub fn parse_file_link(link: &str) -> Result<String, Response> {
    let decode_bytes = base64_decode(link)?;
    let split: Vec<_> = decode_bytes.splitn(2, |b| *b == 0).collect();
    let bytes = *op::some!(split.first(); ret Err(Response::invalid_value("文件链接解析错误")));
    Ok(String::from_utf8_lossy(bytes).to_string())
}
