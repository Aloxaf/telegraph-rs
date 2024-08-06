use super::{error::Error, utils::*};
use reqwest::multipart::Part;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

/// This object represents a Telegraph account.
#[derive(Debug, Clone, Deserialize)]
pub struct Account {
    /// Account name, helps users with several accounts remember which they are currently using.
    ///
    /// Displayed to the user above the "Edit/Publish" button on Telegra.ph, other users don't see this name.
    pub short_name: Option<String>,
    /// Default author name used when creating new articles.
    pub author_name: Option<String>,
    /// Profile link, opened when users click on the author's name below the title.
    ///
    /// Can be any link, not necessarily to a Telegram profile or channel.
    pub author_url: Option<String>,
    /// Optional. Only returned by the createAccount and revokeAccessToken method.
    ///
    /// Access token of the Telegraph account.
    pub access_token: Option<String>,
    /// Optional. URL to authorize a browser on telegra.ph and connect it to a Telegraph account.
    ///
    /// This URL is valid for only one use and for 5 minutes only.
    pub auth_url: Option<String>,
    /// Optional. Number of pages belonging to the Telegraph account.
    pub page_count: Option<i32>,
}

/// This object represents a list of Telegraph articles belonging to an account. Most recently created articles first.
#[derive(Debug, Clone, Deserialize)]
pub struct PageList {
    /// Total number of pages belonging to the target Telegraph account.
    pub total_count: i32,
    /// Requested pages of the target Telegraph account.
    pub pages: Vec<Page>,
}

/// This object represents a page on Telegraph.
#[derive(Debug, Clone, Deserialize)]
pub struct Page {
    /// Path to the page.
    pub path: String,
    /// URL of the page.
    pub url: String,
    /// Title of the page.
    pub title: String,
    /// Description of the page.
    pub description: String,
    /// Optional. Name of the author, displayed below the title.
    pub author_name: Option<String>,
    /// Optional. Profile link, opened when users click on the author's name below the title.
    ///
    /// Can be any link, not necessarily to a Telegram profile or channel.
    pub author_url: Option<String>,
    /// Optional. Image URL of the page.
    pub image_url: Option<String>,
    /// Optional. Content of the page.
    pub content: Option<Vec<Node>>,
    /// Number of page views for the page.
    pub views: i32,
    /// Optional. Only returned if access_token passed.
    ///
    /// True, if the target Telegraph account can edit the page.
    pub can_edit: Option<bool>,
}

/// This object represents the number of page views for a Telegraph article.
#[derive(Debug, Clone, Deserialize)]
pub struct PageViews {
    /// Number of page views for the target page.
    pub views: i32,
}

/// This abstract object represents a DOM Node.
///
/// It can be a String which represents a DOM text node or a NodeElement object.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Node {
    Text(String),
    NodeElement(NodeElement),
}

/// This object represents a DOM element node.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NodeElement {
    /// Name of the DOM element.
    /// Available tags: a, aside, b, blockquote, br, code, em, figcaption, figure, h3, h4, hr, i, iframe, img, li, ol, p, pre, s, strong, u, ul, video.
    pub tag: String,
    /// Optional. Attributes of the DOM element.
    ///
    /// Key of object represents name of attribute, value represents value of attribute.
    ///
    /// Available attributes: href, src.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attrs: Option<HashMap<String, Option<String>>>,
    /// Optional. List of child nodes for the DOM element.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Node>>,
}

/// This object represents the upload result
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum UploadResult {
    Error { error: String },
    Source(Vec<ImageInfo>),
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImageInfo {
    /// Path of the file uploaded.
    pub src: String,
}

#[cfg(feature = "upload")]
pub trait Uploadable {
    fn part(&self) -> Result<Part, Error>;
}

#[cfg(feature = "upload")]
impl<T> Uploadable for T
where
    T: AsRef<Path>,
{
    fn part(&self) -> Result<Part, Error> {
        let path = self.as_ref();
        let bytes = read_to_bytes(path)?;
        let part = Part::bytes(bytes)
            .file_name(path.file_name().unwrap().to_string_lossy().to_string())
            .mime_str(&guess_mime(path))?;
        Ok(part)
    }
}
