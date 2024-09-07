use axum::http;
use http::header::{HeaderMap, HeaderValue};

use http::header::AUTHORIZATION;

use super::HeaderParserError;

impl TryFrom<&HeaderMap> for Bearer {
    type Error = HeaderParserError;

    fn try_from(value: &HeaderMap) -> Result<Self, Self::Error> {
        match value.get(AUTHORIZATION) {
            Some(value) => Self::try_from(value),
            _ => Err(HeaderParserError::MissingHeaderValue(AUTHORIZATION)),
        }
    }
}
impl TryFrom<&HeaderValue> for Bearer {
    type Error = HeaderParserError;

    fn try_from(value: &HeaderValue) -> Result<Self, Self::Error> {
        match value.to_str() {
            Ok(value) => Self::try_from(value),
            _ => Err(HeaderParserError::NotAllVisibleAscii),
        }
    }
}

impl TryFrom<String> for Bearer {
    type Error = HeaderParserError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}
impl TryFrom<&String> for Bearer {
    type Error = HeaderParserError;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl TryFrom<&str> for Bearer {
    type Error = HeaderParserError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if let Some(token) = value.strip_prefix("Bearer ") {
            Ok(Self {
                bearer: value.into(),
                token: token.into(),
            })
        } else {
            Err(HeaderParserError::InvalidValue(value.into()))
        }
    }
}
unsafe impl Send for Bearer {}

unsafe impl Sync for Bearer {}
impl Unpin for Bearer {}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bearer {
    bearer: String,
    token: String,
}
impl AsRef<str> for Bearer {
    fn as_ref(&self) -> &str {
        &self.token
    }
}
impl Bearer {
    pub fn token(&self) -> &str {
        self.token.as_str()
    }
    pub fn new(token: impl Into<String>) -> Self {
        let token = token.into();
        Self {
            bearer: format!("Bearer {}", token),
            token,
        }
    }
}
