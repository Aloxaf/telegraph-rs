//! telegraph API binding in Rust
//!
//! See https://telegra.ph/api for more information
//!
//! # Examples
//!
//! ```
//! # async fn run() -> Result<(), telegraph_rs::Error> {
//! use telegraph_rs::{Telegraph, html_to_node};
//!
//! let telegraph = Telegraph::new("test_account").create().await?;
//!
//! let page = telegraph.create_page("title", &html_to_node("<p>Hello, world</p>"), false).await?;
//! # Ok(())
//! # }
//! ```
#[cfg(feature = "blocking")]
pub mod blocking;
pub mod error;
pub mod types;

pub use error::*;
pub use types::*;

use libxml::{
    parser::Parser,
    tree::{self, NodeType},
};
use reqwest::{
    multipart::{Form, Part},
    Client,
};
use std::{collections::HashMap, fs::File, io::Read, path::Path};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Default)]
pub struct AccountBuilder {
    access_token: Option<String>,
    short_name: String,
    author_name: Option<String>,
    author_url: Option<String>,
    client: Client,
}

impl AccountBuilder {
    pub fn new(short_name: &str) -> Self {
        AccountBuilder {
            short_name: short_name.to_owned(),
            ..Default::default()
        }
    }

    /// Account name, helps users with several accounts remember which they are currently using.
    ///
    /// Displayed to the user above the "Edit/Publish" button on Telegra.ph,
    ///
    /// other users don't see this name.
    pub fn short_name(mut self, short_name: &str) -> Self {
        self.short_name = short_name.to_owned();
        self
    }

    ///  Access token of the Telegraph account.
    pub fn access_token(mut self, access_token: &str) -> Self {
        self.access_token = Some(access_token.to_owned());
        self
    }

    /// Default author name used when creating new articles.
    pub fn author_name(mut self, author_name: &str) -> Self {
        self.author_name = Some(author_name.to_owned());
        self
    }

    /// Default profile link, opened when users click on the author's name below the title.
    ///
    /// Can be any link, not necessarily to a Telegram profile or channel.
    pub fn author_url(mut self, author_url: &str) -> Self {
        self.author_url = Some(author_url.to_owned());
        self
    }

    /// Client
    pub fn client(mut self, client: Client) -> Self {
        self.client = client;
        self
    }

    /// If `access_token` is not set, an new account will be create.
    ///
    /// Otherwise import the existing account.
    pub async fn create(mut self) -> Result<Telegraph> {
        if self.access_token.is_none() {
            let account = Telegraph::create_account(
                &self.short_name,
                self.author_name.as_deref(),
                self.author_url.as_deref(),
            )
            .await?;
            self.access_token = Some(account.access_token.unwrap());
        }

        Ok(Telegraph {
            client: self.client,
            access_token: self.access_token.unwrap(),
            short_name: self.short_name.to_owned(),
            author_name: self.author_name.unwrap_or(self.short_name),
            author_url: self.author_url,
        })
    }

    /// Edit info of an an existing account.
    pub async fn edit(self) -> Result<Telegraph> {
        let response = Client::new()
            .get("https://api.telegra.ph/editAccountInfo")
            .query(&[
                ("access_token", self.access_token.as_ref().unwrap()),
                ("short_name", &self.short_name),
                ("author_name", self.author_name.as_ref().unwrap()),
                (
                    "author_url",
                    self.author_url.as_ref().unwrap_or(&String::new()),
                ),
            ])
            .send()
            .await?;
        let json: Result<Account> = response.json::<ApiResult<Account>>().await?.into();
        let json = json?;

        Ok(Telegraph {
            client: Client::new(),
            access_token: self.access_token.unwrap(),
            short_name: json.short_name.clone().unwrap(),
            author_name: json.author_name.or(json.short_name).unwrap(),
            author_url: json.author_url,
        })
    }
}

#[derive(Debug)]
pub struct Telegraph {
    client: Client,
    access_token: String,
    short_name: String,
    author_name: String,
    author_url: Option<String>,
}

impl Telegraph {
    /// Use this method to create a new Telegraph account or import an existing one.
    ///
    /// Most users only need one account, but this can be useful for channel administrators who would like to keep individual author names and profile links for each of their channels.
    ///
    /// On success, returns an Account object with the regular fields and an additional access_token field.
    ///
    /// ```
    /// # async fn run() -> Result<(), telegraph_rs::Error> {
    /// use telegraph_rs::Telegraph;
    ///
    /// let account = Telegraph::new("short_name")
    ///     .access_token("b968da509bb76866c35425099bc0989a5ec3b32997d55286c657e6994bbb")
    ///     .create()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(short_name: &str) -> AccountBuilder {
        AccountBuilder {
            short_name: short_name.to_owned(),
            ..Default::default()
        }
    }

    pub(crate) async fn create_account<'a, S, T>(
        short_name: &str,
        author_name: S,
        author_url: T,
    ) -> Result<Account>
    where
        T: Into<Option<&'a str>>,
        S: Into<Option<&'a str>>,
    {
        let mut params = HashMap::new();
        params.insert("short_name", short_name);
        if let Some(author_name) = author_name.into() {
            params.insert("author_name", author_name);
        }
        if let Some(author_url) = author_url.into() {
            params.insert("author_url", author_url);
        }
        let response = Client::new()
            .get("https://api.telegra.ph/createAccount")
            .query(&params)
            .send()
            .await?;
        response.json::<ApiResult<Account>>().await?.into()
    }

    /// Use this method to create a new Telegraph page. On success, returns a Page object.
    ///
    /// if `return_content` is true, a content field will be returned in the Page object.
    ///
    /// ```
    /// # async fn test() -> Result<(), telegraph_rs::Error> {
    /// use telegraph_rs::{Telegraph, html_to_node};
    ///
    /// let telegraph = Telegraph::new("author")
    ///     .access_token("b968da509bb76866c35425099bc0989a5ec3b32997d55286c657e6994bbb")
    ///     .create()
    ///     .await?;
    ///
    /// let page = telegraph.create_page("title", &html_to_node("<p>Hello, world!</p>"), false).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_page(
        &self,
        title: &str,
        content: &str,
        return_content: bool,
    ) -> Result<Page> {
        // TODO: content HTML 形式
        let response = self
            .client
            .post("https://api.telegra.ph/createPage")
            .form(&[
                ("access_token", &*self.access_token),
                ("title", title),
                ("author_name", &*self.author_name),
                ("author_url", self.author_url.as_deref().unwrap_or("")),
                ("content", content),
                ("return_content", &*return_content.to_string()),
            ])
            .send()
            .await?;
        response.json::<ApiResult<Page>>().await?.into()
    }

    /// Use this method to update information about a Telegraph account.
    ///
    /// Pass only the parameters that you want to edit.
    ///
    /// On success, returns an Account object with the default fields.
    pub fn edit_account_info(self) -> AccountBuilder {
        AccountBuilder {
            access_token: Some(self.access_token),
            short_name: self.short_name,
            author_name: Some(self.author_name),
            author_url: self.author_url,
            client: self.client,
        }
    }

    /// Use this method to edit an existing Telegraph page.
    ///
    /// On success, returns a Page object.
    pub async fn edit_page(
        &self,
        path: &str,
        title: &str,
        content: &str,
        return_content: bool,
    ) -> Result<Page> {
        let response = self
            .client
            .post("https://api.telegra.ph/editPage")
            .form(&[
                ("access_token", &*self.access_token),
                ("path", path),
                ("title", title),
                ("author_name", &*self.author_name),
                ("author_url", self.author_url.as_deref().unwrap_or("")),
                ("content", content),
                ("return_content", &*return_content.to_string()),
            ])
            .send()
            .await?;
        response.json::<ApiResult<Page>>().await?.into()
    }

    /// Use this method to get information about a Telegraph account. Returns an Account object on success.
    ///
    /// Available fields: short_name, author_name, author_url, auth_url, page_count.
    pub async fn get_account_info(&self, fields: &[&str]) -> Result<Account> {
        let response = self
            .client
            .get("https://api.telegra.ph/getAccountInfo")
            .query(&[
                ("access_token", &self.access_token),
                ("fields", &serde_json::to_string(fields).unwrap()),
            ])
            .send()
            .await?;
        response.json::<ApiResult<Account>>().await?.into()
    }

    /// Use this method to get a Telegraph page. Returns a Page object on success.
    pub async fn get_page(path: &str, return_content: bool) -> Result<Page> {
        let response = Client::new()
            .get(&format!("https://api.telegra.ph/getPage/{}", path))
            .query(&[("return_content", return_content.to_string())])
            .send()
            .await?;
        response.json::<ApiResult<Page>>().await?.into()
    }

    /// Use this method to get a list of pages belonging to a Telegraph account.
    ///
    /// Returns a PageList object, sorted by most recently created pages first.
    ///
    /// - `offset` Sequential number of the first page to be returned. (suggest: 0)
    /// - `limit` Limits the number of pages to be retrieved. (suggest: 50)
    pub async fn get_page_list(&self, offset: i32, limit: i32) -> Result<PageList> {
        let response = self
            .client
            .get("https://api.telegra.ph/getPageList")
            .query(&[
                ("access_token", &self.access_token),
                ("offset", &offset.to_string()),
                ("limit", &limit.to_string()),
            ])
            .send()
            .await?;
        response.json::<ApiResult<PageList>>().await?.into()
    }

    /// Use this method to get the number of views for a Telegraph article.
    ///
    /// Returns a PageViews object on success.
    ///
    /// By default, the total number of page views will be returned.
    ///
    /// ```rust
    /// # async fn run() -> Result<(), telegraph_rs::Error> {
    /// use telegraph_rs::Telegraph;
    ///
    /// let view1 = Telegraph::get_views("Sample-Page-12-15", &vec![2016, 12]).await?;
    /// let view2 = Telegraph::get_views("Sample-Page-12-15", &vec![2019, 5, 19, 12]).await?; // year-month-day-hour
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_views(path: &str, time: &[i32]) -> Result<PageViews> {
        let params = ["year", "month", "day", "hour"]
            .iter()
            .zip(time)
            .collect::<HashMap<_, _>>();

        let response = Client::new()
            .get(&format!("https://api.telegra.ph/getViews/{}", path))
            .query(&params)
            .send()
            .await?;
        response.json::<ApiResult<PageViews>>().await?.into()
    }

    /// Use this method to revoke access_token and generate a new one,
    ///
    /// for example, if the user would like to reset all connected sessions,
    ///
    /// or you have reasons to believe the token was compromised.
    ///
    /// On success, returns an Account object with new access_token and auth_url fields.
    pub async fn revoke_access_token(&mut self) -> Result<Account> {
        let response = self
            .client
            .get("https://api.telegra.ph/revokeAccessToken")
            .query(&[("access_token", &self.access_token)])
            .send()
            .await?;
        let json: Result<Account> = response.json::<ApiResult<Account>>().await?.into();
        if json.is_ok() {
            self.access_token = json
                .as_ref()
                .unwrap()
                .access_token
                .as_ref()
                .unwrap()
                .to_owned();
        }
        json
    }

    /// Upload files to telegraph with custom client
    #[cfg(feature = "upload")]
    pub async fn upload_with<P: AsRef<Path>>(
        files: &[P],
        client: &Client,
    ) -> Result<Vec<ImageInfo>> {
        let mut form = Form::new();
        for (idx, name) in files.iter().enumerate() {
            let bytes = read_to_bytes(name)?;
            let part = Part::bytes(bytes)
                .file_name(idx.to_string())
                .mime_str(&guess_mime(name))?;
            form = form.part(idx.to_string(), part);
        }
        let response = client
            .post("https://telegra.ph/upload")
            .multipart(form)
            .send()
            .await?;
        let full = response.bytes().await?;

        match serde_json::from_slice::<UploadResult>(&full) {
            Ok(UploadResult::Error { error }) => Err(Error::ApiError(error)),
            Ok(UploadResult::Source(v)) => Ok(v),
            Err(e) => {
                Err(Error::ApiError(format!("{}: {}", e, String::from_utf8_lossy(&full))))
            }
        }
    }

    /// Upload files to telegraph
    #[cfg(feature = "upload")]
    pub async fn upload<P: AsRef<Path>>(files: &[P]) -> Result<Vec<ImageInfo>> {
        Self::upload_with(files, &Client::new()).await
    }
}

#[cfg(feature = "upload")]
fn guess_mime<P: AsRef<Path>>(path: P) -> String {
    let mime = mime_guess::from_path(path).first_or(mime_guess::mime::TEXT_PLAIN);
    let mut s = format!("{}/{}", mime.type_(), mime.subtype());
    if let Some(suffix) = mime.suffix() {
        s.push('+');
        s.push_str(suffix.as_str());
    }
    s
}

#[cfg(feature = "upload")]
fn read_to_bytes<P: AsRef<Path>>(path: P) -> Result<Vec<u8>> {
    let mut bytes = vec![];
    let mut file = File::open(path)?;
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// Parse html to node string
///
/// ```rust
/// use telegraph_rs::html_to_node;
///
/// let node = html_to_node("<p>Hello, world</p>");
/// assert_eq!(node, r#"[{"tag":"p","attrs":null,"children":["Hello, world"]}]"#);
/// ```
pub fn html_to_node(html: &str) -> String {
    let parser = Parser::default_html();
    let document = parser.parse_string(html).unwrap();
    let node = document
        .get_root_element()
        .unwrap()
        .get_first_element_child()
        .unwrap();
    let nodes = node
        .get_child_nodes()
        .into_iter()
        .map(|node| html_to_node_inner(&node))
        .collect::<Vec<_>>();
    serde_json::to_string(&nodes).unwrap()
}

fn html_to_node_inner(node: &tree::Node) -> Option<Node> {
    match node.get_type() {
        Some(NodeType::TextNode) => Some(Node::Text(node.get_content())),
        Some(NodeType::ElementNode) => Some(Node::NodeElement(NodeElement {
            tag: node.get_name(),
            attrs: {
                let attrs = node.get_attributes();
                if attrs.is_empty() {
                    None
                } else {
                    Some(attrs)
                }
            },
            children: {
                let childs = node.get_child_nodes();
                if childs.is_empty() {
                    None
                } else {
                    childs
                        .into_iter()
                        .map(|node| html_to_node_inner(&node))
                        .collect::<Option<Vec<_>>>()
                }
            },
        })),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::Telegraph;

    #[test]
    fn html_to_node() {
        let html = r#"<a>Text</a><p>img:<img src="https://me"></p>"#;
        println!("{}", super::html_to_node(html));
    }

    #[tokio::test]
    async fn create_and_revoke_account() {
        let result = Telegraph::create_account("sample", "a", None).await;
        println!("{:?}", result);
        assert!(result.is_ok());

        let mut telegraph = Telegraph::new("test")
            .access_token(&result.unwrap().access_token.unwrap().to_owned())
            .create()
            .await
            .unwrap();
        let result = telegraph.revoke_access_token().await;
        println!("{:?}", result);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn edit_account_info() {
        let result = Telegraph::new("test")
            .access_token("b968da509bb76866c35425099bc0989a5ec3b32997d55286c657e6994bbb")
            .create()
            .await
            .unwrap()
            .edit_account_info()
            .short_name("wow")
            .edit()
            .await;
        println!("{:?}", result);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn get_account_info() {
        let result = Telegraph::new("test")
            .access_token("b968da509bb76866c35425099bc0989a5ec3b32997d55286c657e6994bbb")
            .create()
            .await
            .unwrap()
            .get_account_info(&["short_name"])
            .await;
        println!("{:?}", result);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_get_edit_page() {
        let telegraph = Telegraph::new("test")
            .access_token("b968da509bb76866c35425099bc0989a5ec3b32997d55286c657e6994bbb")
            .create()
            .await
            .unwrap();
        let page = telegraph
            .create_page(
                "OVO",
                r#"[{"tag":"p","children":["Hello,+world!"]}]"#,
                false,
            )
            .await;
        println!("{:?}", page);
        assert!(page.is_ok());

        let page = Telegraph::get_page(&page.unwrap().path, true).await;
        println!("{:?}", page);
        assert!(page.is_ok());

        let page = telegraph
            .edit_page(
                &page.unwrap().path,
                "QAQ",
                r#"[{"tag":"p","children":["Goodbye,+world!"]}]"#,
                false,
            )
            .await;
        println!("{:?}", page);
        assert!(page.is_ok());
    }

    #[tokio::test]
    async fn get_page_list() {
        let telegraph = Telegraph::new("test")
            .access_token("b968da509bb76866c35425099bc0989a5ec3b32997d55286c657e6994bbb")
            .create()
            .await
            .unwrap();
        let page_list = telegraph.get_page_list(0, 3).await;
        println!("{:?}", page_list);
        assert!(page_list.is_ok());
    }

    #[tokio::test]
    async fn get_views() {
        let views = Telegraph::get_views("Sample-Page-12-15", &vec![2016, 12]).await;
        println!("{:?}", views);
        assert!(views.is_ok());
    }

    #[ignore]
    #[tokio::test]
    #[cfg(feature = "upload")]
    async fn upload() {
        let images = Telegraph::upload(&vec!["1.jpeg", "2.jpeg"]).await;
        println!("{:?}", images);
        assert!(images.is_ok());
    }
}
