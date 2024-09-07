use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use axum::{
    http::{HeaderValue, StatusCode},
    Json,
};
use serde::{ser::SerializeStruct, Serialize};
use serde_json::{json, Value};

use crate::{libs::parse_file_link, log, pages::func::DEFAULT_PRODUCT_COVER};
/// 响应数据
#[derive(Debug)]
pub struct Response {
    /// 响应状态码
    code: StatusCode,
    status: i32,
    data: Value,
}

impl axum::response::IntoResponse for Response {
    fn into_response(self) -> axum::response::Response {
        Json(self).into_response()
    }
}

impl Serialize for Response {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("Response", 2)?;
        if self.status != 0 {
            log!("检测到请求错误，错误信息：{}", self.data);
        }
        s.serialize_field("status", &self.status)?;
        s.serialize_field("code", &self.code.as_u16())?;
        s.serialize_field("data", &self.data)?;
        s.end()
    }
}
impl Response {
    pub fn new(code: StatusCode, status: i32, data: Value) -> Response {
        Self { code, status, data }
    }
    pub fn ok(data: Value) -> Self {
        Self {
            code: StatusCode::OK,
            status: 0,
            data,
        }
    }
    pub fn empty() -> Self {
        Self {
            code: StatusCode::OK,
            status: 0,
            data: json!("OK"),
        }
    }
    pub fn token_error(e: impl Display) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, -1, json!(e.to_string()))
    }
    /// 内部错误
    pub fn internal_server_error(e: impl Display) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, -1, json!(e.to_string()))
    }
    /// 参数格式错误
    pub fn invalid_format(e: impl Display) -> Self {
        Self::new(StatusCode::OK, 1, json!(e.to_string()))
    }
    /// 请求的数据不存在
    pub fn not_exist(e: impl Display) -> Self {
        Self::new(StatusCode::OK, 2, json!(e.to_string()))
    }
    /// 要添加的数据已存在
    pub fn already_exist(e: impl Display) -> Self {
        Self::new(StatusCode::OK, 3, json!(e.to_string()))
    }
    /// 权限不足
    pub fn permission_denied() -> Self {
        Self::new(StatusCode::OK, 4, json!("权限不足"))
    }
    /// 密码错误
    pub fn wrong_password() -> Self {
        Self::new(StatusCode::OK, 5, json!("密码错误"))
    }
    /// 用于测试期间暂时不支持的功能
    pub fn not_supported() -> Self {
        Self::new(StatusCode::OK, 6, json!("暂时不支持"))
    }
    /// 数值不对
    pub fn invalid_value(e: impl Display) -> Self {
        Self::new(StatusCode::OK, 7, json!(e.to_string()))
    }
    /// 条件不满足
    pub fn dissatisfy(e: impl Display) -> Self {
        Self::new(StatusCode::OK, 8, json!(e.to_string()))
    }

    /// 不应该发生的错误
    pub fn unknown_err(e: impl Display) -> Self {
        Self::new(StatusCode::OK, 9, json!(e.to_string()))
    }
    pub fn code(&self) -> StatusCode {
        self.code
    }
    pub fn status(&self) -> i32 {
        self.status
    }
}

impl From<mysql::Error> for Response {
    fn from(value: mysql::Error) -> Self {
        if let mysql::Error::MySqlError(err) = &value {
            if err.code == 1062 {
                return Response::already_exist(format!("重复添加，错误信息：{err}"));
            }
        }

        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            -1,
            json!(value.to_string()),
        )
    }
}
impl From<std::io::Error> for Response {
    fn from(value: std::io::Error) -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            -1,
            json!(value.to_string()),
        )
    }
}

impl From<serde_json::Error> for Response {
    fn from(value: serde_json::Error) -> Self {
        Response::invalid_format(value)
    }
}
impl From<std::time::SystemTimeError> for Response {
    fn from(value: std::time::SystemTimeError) -> Self {
        Response::internal_server_error(value)
    }
}
impl From<axum::extract::multipart::MultipartError> for Response {
    fn from(value: axum::extract::multipart::MultipartError) -> Self {
        Response::internal_server_error(value)
    }
}
impl From<base64::DecodeError> for Response {
    fn from(value: base64::DecodeError) -> Self {
        Response::internal_server_error(format!("base64解码错误，具体信息为：{value}"))
    }
}
#[derive(Default)]
pub struct BodyFile {
    body: Vec<u8>,
    filename: String,
    mime: &'static str,
}
impl axum::response::IntoResponse for BodyFile {
    fn into_response(self) -> axum::response::Response {
        let mut response = self.body.into_response();

        let headers = response.headers_mut();
        headers.insert(
            axum::http::header::CONTENT_TYPE,
            HeaderValue::from_static(self.mime),
        );

        headers.insert(
            axum::http::header::CONTENT_DISPOSITION,
            HeaderValue::from_str(&format!("attachment; filename=\"{}\"", self.filename)).unwrap(),
        );
        response
    }
}

impl BodyFile {
    pub fn new(body: Vec<u8>) -> Self {
        Self { body, ..Default::default() }
    }
    pub fn new_with_base64_url(
        parent: impl AsRef<Path>,
        url: &str,
    ) -> Result<Self, (StatusCode, String)> {
        match url {
            _u if url == DEFAULT_PRODUCT_COVER.0 => {
                return Ok(Self {
                    body: DEFAULT_PRODUCT_COVER.1.to_vec(),
                    filename: "default.png".to_owned(),
                    mime: "image/png",
                });
            }
            _ => (),
        }

        let mut path = parent.as_ref().to_path_buf();
        path.push(url);
        if !path.is_file() {
            return Err((StatusCode::NOT_FOUND, "找不到该地址指向的文件".to_string()));
        }
        let body = std::fs::read(&path).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("内部错误，具体信息为：{e}"),
            )
        })?;
        let filename = op::result!(parse_file_link(url); ret Err((StatusCode::INTERNAL_SERVER_ERROR, "链接解析错误".into())));
        let p = PathBuf::from(&filename);
        let mime = if let Some(ext) = p.extension() {
            match ext.to_string_lossy().as_ref() {
                "jpeg" | "jpg" => "image/jpeg",
                "png" => "image/png",
                // 待定
                _ => "image/png",
            }
        } else {
            "text/plain"
        };
        Ok(Self {
            body,
            filename,
            mime,
        })
    }
}
