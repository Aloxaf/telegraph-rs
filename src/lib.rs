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
//! let telegraph = Telegraph::new();
//! let account = telegraph.account("test_account").create().await?;
//!
//! let page = account.create_page("title", &html_to_node("<p>Hello, world</p>"), false).await?;
//! # Ok(())
//! # }
//! ```
pub mod error;
pub mod types;
pub mod utils;

pub use error::*;
pub use types::*;
pub use utils::*;

use reqwest::{Client, Response};
use std::collections::HashMap;

pub type Result<T> = std::result::Result<T, Error>;

const DEFAULT_API_URL: &str = "https://api.telegra.ph";

macro_rules! send {
    ($e:expr) => {
        $e.send().await.and_then(Response::error_for_status)
    };
}

/// Client for the Telegraph API.
#[derive(Debug, Clone)]
pub struct Telegraph {
    client: Client,
    api_url: String,
}

impl Default for Telegraph {
    fn default() -> Self {
        Self {
            client: Client::new(),
            api_url: DEFAULT_API_URL.to_owned(),
        }
    }
}

impl Telegraph {
    /// Creates a client for the official Telegraph API.
    pub fn new() -> Self {
        Self::default()
    }

    /// Uses a different Telegraph-compatible API base URL.
    pub fn api_url(mut self, api_url: &str) -> Self {
        self.api_url = api_url.trim_end_matches('/').to_owned();
        self
    }

    /// Uses the supplied HTTP client for all API requests.
    pub fn client(mut self, client: Client) -> Self {
        self.client = client;
        self
    }

    /// Use this method to create a new Telegraph account or import an existing one.
    ///
    /// Most users only need one account, but this can be useful for channel administrators who
    /// would like to keep individual author names and profile links for each of their channels.
    ///
    /// On success, returns an account client initialized with the account's access token.
    ///
    /// ```
    /// # async fn run() -> Result<(), telegraph_rs::Error> {
    /// use telegraph_rs::Telegraph;
    ///
    /// let telegraph = Telegraph::new();
    /// let account = telegraph
    ///     .account("short_name")
    ///     .access_token("b968da509bb76866c35425099bc0989a5ec3b32997d55286c657e6994bbb")
    ///     .create()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn account(&self, short_name: &str) -> AccountBuilder {
        AccountBuilder {
            telegraph: self.clone(),
            access_token: None,
            short_name: short_name.to_owned(),
            author_name: None,
            author_url: None,
        }
    }

    /// Use this method to get a Telegraph page. Returns a Page object on success.
    pub async fn get_page(&self, path: &str, return_content: bool) -> Result<Page> {
        let response = send!(
            self.client
                .get(self.endpoint(&format!("getPage/{}", path)))
                .query(&[("return_content", return_content.to_string())])
        )?;
        response.json::<ApiResult<Page>>().await?.into()
    }

    /// Use this method to get the number of views for a Telegraph article.
    ///
    /// Returns a PageViews object on success.
    ///
    /// By default, the total number of page views will be returned.
    ///
    /// `time` may contain the year, month, day and hour, in that order.
    ///
    /// ```rust
    /// # async fn run() -> Result<(), telegraph_rs::Error> {
    /// use telegraph_rs::Telegraph;
    ///
    /// let telegraph = Telegraph::new();
    /// let view1 = telegraph.get_views("Sample-Page-12-15", &[2016, 12]).await?;
    /// let view2 = telegraph
    ///     .get_views("Sample-Page-12-15", &[2019, 5, 19, 12])
    ///     .await?; // year-month-day-hour
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_views(&self, path: &str, time: &[i32]) -> Result<PageViews> {
        let params = ["year", "month", "day", "hour"]
            .iter()
            .zip(time)
            .collect::<HashMap<_, _>>();
        let response = send!(
            self.client
                .get(self.endpoint(&format!("getViews/{}", path)))
                .query(&params)
        )?;
        response.json::<ApiResult<PageViews>>().await?.into()
    }

    fn endpoint(&self, method: &str) -> String {
        format!("{}/{}", self.api_url, method)
    }

    async fn create_account(
        &self,
        short_name: &str,
        author_name: Option<&str>,
        author_url: Option<&str>,
    ) -> Result<Account> {
        let mut params = HashMap::new();
        params.insert("short_name", short_name);
        if let Some(author_name) = author_name {
            params.insert("author_name", author_name);
        }
        if let Some(author_url) = author_url {
            params.insert("author_url", author_url);
        }
        let response = send!(
            self.client
                .get(self.endpoint("createAccount"))
                .query(&params)
        )?;
        response.json::<ApiResult<Account>>().await?.into()
    }
}

/// Configures a Telegraph account before it is created, imported, or edited.
#[derive(Debug, Clone)]
pub struct AccountBuilder {
    telegraph: Telegraph,
    access_token: Option<String>,
    short_name: String,
    author_name: Option<String>,
    author_url: Option<String>,
}

impl AccountBuilder {
    /// Account name, helps users with several accounts remember which they are currently using.
    ///
    /// Displayed to the user above the "Edit/Publish" button on Telegra.ph,
    ///
    /// other users don't see this name.
    pub fn short_name(mut self, short_name: &str) -> Self {
        self.short_name = short_name.to_owned();
        self
    }

    /// Access token of the Telegraph account.
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

    /// If `access_token` is not set, a new account will be created.
    ///
    /// Otherwise import the existing account.
    pub async fn create(mut self) -> Result<TelegraphAccount> {
        if self.access_token.is_none() {
            let account = self
                .telegraph
                .create_account(
                    &self.short_name,
                    self.author_name.as_deref(),
                    self.author_url.as_deref(),
                )
                .await?;
            self.access_token = Some(account.access_token.unwrap());
        }

        Ok(TelegraphAccount {
            telegraph: self.telegraph,
            access_token: self.access_token.unwrap(),
            short_name: self.short_name.clone(),
            author_name: self.author_name.unwrap_or(self.short_name),
            author_url: self.author_url,
        })
    }

    /// Edit info of an existing account.
    pub async fn edit(self) -> Result<TelegraphAccount> {
        let response = send!(
            self.telegraph
                .client
                .get(self.telegraph.endpoint("editAccountInfo"))
                .query(&[
                    ("access_token", self.access_token.as_ref().unwrap()),
                    ("short_name", &self.short_name),
                    ("author_name", self.author_name.as_ref().unwrap()),
                    (
                        "author_url",
                        self.author_url.as_ref().unwrap_or(&String::new()),
                    ),
                ])
        )?;
        let account: Result<Account> = response.json::<ApiResult<Account>>().await?.into();
        let account = account?;

        Ok(TelegraphAccount {
            telegraph: self.telegraph,
            access_token: self.access_token.unwrap(),
            short_name: account.short_name.clone().unwrap(),
            author_name: account.author_name.or(account.short_name).unwrap(),
            author_url: account.author_url,
        })
    }
}

/// An authenticated Telegraph account.
#[derive(Debug, Clone)]
pub struct TelegraphAccount {
    telegraph: Telegraph,
    access_token: String,
    short_name: String,
    author_name: String,
    author_url: Option<String>,
}

impl TelegraphAccount {
    /// Use this method to create a new Telegraph page. On success, returns a Page object.
    ///
    /// If `return_content` is true, a content field will be returned in the Page object.
    ///
    /// ```
    /// # async fn run() -> Result<(), telegraph_rs::Error> {
    /// use telegraph_rs::{Telegraph, html_to_node};
    ///
    /// let telegraph = Telegraph::new();
    /// let account = telegraph
    ///     .account("author")
    ///     .access_token("b968da509bb76866c35425099bc0989a5ec3b32997d55286c657e6994bbb")
    ///     .create()
    ///     .await?;
    ///
    /// let page = account
    ///     .create_page("title", &html_to_node("<p>Hello, world!</p>"), false)
    ///     .await?;
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
        let response = send!(
            self.telegraph
                .client
                .post(self.telegraph.endpoint("createPage"))
                .form(&[
                    ("access_token", &*self.access_token),
                    ("title", title),
                    ("author_name", &*self.author_name),
                    ("author_url", self.author_url.as_deref().unwrap_or("")),
                    ("content", content),
                    ("return_content", &*return_content.to_string()),
                ])
        )?;
        response.json::<ApiResult<Page>>().await?.into()
    }

    /// Use this method to update information about a Telegraph account.
    ///
    /// Pass only the parameters that you want to edit.
    ///
    /// On success, returns an Account object with the default fields.
    pub fn edit_account_info(self) -> AccountBuilder {
        AccountBuilder {
            telegraph: self.telegraph,
            access_token: Some(self.access_token),
            short_name: self.short_name,
            author_name: Some(self.author_name),
            author_url: self.author_url,
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
        let response = send!(
            self.telegraph
                .client
                .post(self.telegraph.endpoint("editPage"))
                .form(&[
                    ("access_token", &*self.access_token),
                    ("path", path),
                    ("title", title),
                    ("author_name", &*self.author_name),
                    ("author_url", self.author_url.as_deref().unwrap_or("")),
                    ("content", content),
                    ("return_content", &*return_content.to_string()),
                ])
        )?;
        response.json::<ApiResult<Page>>().await?.into()
    }

    /// Use this method to get information about a Telegraph account.
    ///
    /// Returns an Account object on success.
    ///
    /// Available fields: short_name, author_name, author_url, auth_url, page_count.
    pub async fn get_account_info(&self, fields: &[&str]) -> Result<Account> {
        let response = send!(
            self.telegraph
                .client
                .get(self.telegraph.endpoint("getAccountInfo"))
                .query(&[
                    ("access_token", &self.access_token),
                    ("fields", &serde_json::to_string(fields).unwrap()),
                ])
        )?;
        response.json::<ApiResult<Account>>().await?.into()
    }

    /// Use this method to get a list of pages belonging to a Telegraph account.
    ///
    /// Returns a PageList object, sorted by most recently created pages first.
    ///
    /// - `offset` Sequential number of the first page to be returned. (suggest: 0)
    /// - `limit` Limits the number of pages to be retrieved. (suggest: 50)
    pub async fn get_page_list(&self, offset: i32, limit: i32) -> Result<PageList> {
        let response = send!(
            self.telegraph
                .client
                .get(self.telegraph.endpoint("getPageList"))
                .query(&[
                    ("access_token", &self.access_token),
                    ("offset", &offset.to_string()),
                    ("limit", &limit.to_string()),
                ])
        )?;
        response.json::<ApiResult<PageList>>().await?.into()
    }

    /// Use this method to revoke access_token and generate a new one,
    ///
    /// for example, if the user would like to reset all connected sessions,
    ///
    /// or you have reasons to believe the token was compromised.
    ///
    /// On success, returns an Account object with new access_token and auth_url fields.
    pub async fn revoke_access_token(&mut self) -> Result<Account> {
        let response = send!(
            self.telegraph
                .client
                .get(self.telegraph.endpoint("revokeAccessToken"))
                .query(&[("access_token", &self.access_token)])
        )?;
        let account: Result<Account> = response.json::<ApiResult<Account>>().await?.into();
        if let Ok(account) = &account {
            self.access_token = account.access_token.as_ref().unwrap().to_owned();
        }
        account
    }
}

#[cfg(feature = "html")]
fn html_to_node_inner(node: &html_parser::Node) -> Option<Node> {
    match node {
        html_parser::Node::Text(text) => Some(Node::Text(text.to_owned())),
        html_parser::Node::Element(element) => Some(Node::NodeElement(NodeElement {
            tag: element.name.to_owned(),
            attrs: { (!element.attributes.is_empty()).then(|| element.attributes.clone()) },
            children: {
                if element.children.is_empty() {
                    None
                } else {
                    element
                        .children
                        .iter()
                        .map(|node| html_to_node_inner(node))
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
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn html_to_node() {
        let html = r#"<a>Text</a><p>img:<img src="https://me"></p>"#;
        println!("{}", super::html_to_node(html));
    }

    #[tokio::test]
    async fn create_and_revoke_account() {
        let mut account = Telegraph::new()
            .account("sample")
            .author_name("a")
            .create()
            .await
            .unwrap();
        let result = account.revoke_access_token().await;
        println!("{:?}", result);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn edit_account_info() {
        let result = Telegraph::new()
            .account("test")
            .access_token("d3b25feccb89e508a9114afb82aa421fe2a9712b963b387cc5ad71e58722")
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
        let result = Telegraph::new()
            .account("test")
            .access_token("d3b25feccb89e508a9114afb82aa421fe2a9712b963b387cc5ad71e58722")
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
        let telegraph = Telegraph::new();
        let account = telegraph
            .account("test")
            .access_token("d3b25feccb89e508a9114afb82aa421fe2a9712b963b387cc5ad71e58722")
            .create()
            .await
            .unwrap();
        let page = account
            .create_page(
                "OVO",
                r#"[{"tag":"p","children":["Hello,+world!"]}]"#,
                false,
            )
            .await;
        println!("{:?}", page);
        assert!(page.is_ok());

        let page = telegraph.get_page(&page.unwrap().path, true).await;
        println!("{:?}", page);
        assert!(page.is_ok());

        let page = account
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
        let account = Telegraph::new()
            .account("test")
            .access_token("d3b25feccb89e508a9114afb82aa421fe2a9712b963b387cc5ad71e58722")
            .create()
            .await
            .unwrap();
        let page_list = account.get_page_list(0, 3).await;
        println!("{:?}", page_list);
        assert!(page_list.is_ok());
    }

    #[tokio::test]
    async fn get_views() {
        let views = Telegraph::new()
            .get_views("Sample-Page-12-15", &[2016, 12])
            .await;
        println!("{:?}", views);
        assert!(views.is_ok());
    }

    /// 自定义 API 地址应统一接收公开接口和账户接口的请求。
    #[tokio::test]
    async fn custom_api_url() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            (0..2)
                .map(|_| {
                    let (mut stream, _) = listener.accept().unwrap();
                    let mut request = [0; 4096];
                    let size = stream.read(&mut request).unwrap();
                    let request_line = String::from_utf8_lossy(&request[..size])
                        .lines()
                        .next()
                        .unwrap()
                        .to_owned();
                    let body = r#"{"ok":true,"result":{"path":"test","url":"https://example.com/test","title":"title","description":"","views":0}}"#;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    stream.write_all(response.as_bytes()).unwrap();
                    request_line
                })
                .collect::<Vec<_>>()
        });

        let telegraph = Telegraph::new().api_url(&format!("http://{}/", address));
        let account = telegraph
            .account("test")
            .access_token("token")
            .create()
            .await
            .unwrap();
        let page = telegraph.get_page("test", true).await.unwrap();
        account.create_page("title", "[]", false).await.unwrap();

        assert_eq!(page.path, "test");
        assert_eq!(
            server.join().unwrap(),
            [
                "GET /getPage/test?return_content=true HTTP/1.1",
                "POST /createPage HTTP/1.1",
            ]
        );
    }
}
