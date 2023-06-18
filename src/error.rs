use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum ApiResult<T> {
    Ok { result: T },
    Err { ok: bool, error: String },
}

impl<T> Into<Result<T, Error>> for ApiResult<T> {
    fn into(self) -> Result<T, Error> {
        match self {
            ApiResult::Ok { result: v } => Ok(v),
            ApiResult::Err { error: e, .. } => Err(Error::ApiError(e)),
        }
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("api error: {0}")]
    ApiError(String),
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
}
