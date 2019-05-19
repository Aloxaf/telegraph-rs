use serde::Deserialize;
use std::error;
use std::fmt;

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
            ApiResult::Err { ok: _, error: e } => Err(Error::ApiError(e)),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    ReqwestError(reqwest::Error),
    ApiError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::ReqwestError(e) => write!(f, "reqwest error: {}", e),
            Error::ApiError(e) => write!(f, "api error: {}", e),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::ReqwestError(e) => Some(e),
            Error::ApiError(_) => None,
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::ReqwestError(e)
    }
}
