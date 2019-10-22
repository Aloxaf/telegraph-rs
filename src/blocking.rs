//! telegraph API binding in Rust
//!
//! See https://telegra.ph/api for more information
//!
//! # Examples
//!
//! ```
//! use telegraph_rs::html_to_node;
//! use telegraph_rs::blocking::Telegraph;
//!
//! let telegraph = Telegraph::new("test_account").create().unwrap();
//!
//! let page = telegraph.create_page("title", &html_to_node("<p>Hello, world</p>"), false).unwrap();
//! ```
//!

pub use crate::{error::*, types::*};

use std::{collections::HashMap, path::Path};

use reqwest::blocking::{
    multipart::{Form, Part},
    Client,
};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Default)]
pub struct AccountBuilder {
    access_token: Option<String>,
    short_name: String,
    author_name: Option<String>,
    author_url: Option<String>,
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

    /// If `access_token` is not set, an new account will be create.
    ///
    /// Otherwise import the existing account.
    pub fn create(mut self) -> Result<Telegraph> {
        if self.access_token.is_none() {
            let account = Telegraph::create_account(
                &self.short_name,
                self.author_name.as_ref().map(|s| &**s),
                self.author_url.as_ref().map(|s| &**s),
            )?;
            self.access_token = Some(account.access_token.unwrap().to_owned());
        }

        Ok(Telegraph {
            client: Client::new(),
            access_token: self.access_token.unwrap().to_owned(),
            short_name: self.short_name.to_owned(),
            author_name: self.author_name.unwrap_or(self.short_name.to_owned()),
            author_url: self.author_url.clone(),
        })
    }

    /// Edit info of an an existing account.
    pub fn edit(self) -> Result<Telegraph> {
        let mut response = Client::new()
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
            .send()?;
        let json: Result<Account> = response.json::<ApiResult<Account>>()?.into();
        let json = json?;

        Ok(Telegraph {
            client: Client::new(),
            access_token: self.access_token.clone().unwrap(),
            short_name: json.short_name.clone().unwrap(),
            author_name: json.author_name.unwrap_or(json.short_name.clone().unwrap()),
            author_url: json.author_url.clone(),
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
    /// use telegraph_rs::blocking::Telegraph;
    ///
    /// let account = Telegraph::new("short_name")
    ///     .access_token("b968da509bb76866c35425099bc0989a5ec3b32997d55286c657e6994bbb")
    ///     .create();
    /// ```
    pub fn new(short_name: &str) -> AccountBuilder {
        AccountBuilder {
            short_name: short_name.to_owned(),
            ..Default::default()
        }
    }

    pub(crate) fn create_account<'a, S, T>(
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
        let mut response = Client::new()
            .get("https://api.telegra.ph/createAccount")
            .query(&params)
            .send()?;
        response.json::<ApiResult<Account>>()?.into()
    }

    /// Use this method to create a new Telegraph page. On success, returns a Page object.
    ///
    /// if `return_content` is true, a content field will be returned in the Page object.
    ///
    /// ```
    /// use telegraph_rs::blocking::Telegraph;
    /// use telegraph_rs::html_to_node;
    ///
    /// let telegraph = Telegraph::new("author")
    ///     .access_token("b968da509bb76866c35425099bc0989a5ec3b32997d55286c657e6994bbb")
    ///     .create()
    ///     .unwrap();
    ///
    /// let page = telegraph.create_page("title", &html_to_node("<p>Hello, world!</p>"), false);
    /// ```
    pub fn create_page(&self, title: &str, content: &str, return_content: bool) -> Result<Page> {
        // TODO: content HTML 形式
        let mut response = self
            .client
            .post("https://api.telegra.ph/createPage")
            .form(&[
                ("access_token", &*self.access_token),
                ("title", title),
                ("author_name", &*self.author_name),
                (
                    "author_url",
                    self.author_url.as_ref().map(|s| &**s).unwrap_or(""),
                ),
                ("content", content),
                ("return_content", &*return_content.to_string()),
            ])
            .send()?;
        response.json::<ApiResult<Page>>()?.into()
    }

    /// Use this method to update information about a Telegraph account.
    ///
    /// Pass only the parameters that you want to edit.
    ///
    /// On success, returns an Account object with the default fields.
    pub fn edit_account_info(self) -> AccountBuilder {
        AccountBuilder {
            access_token: Some(self.access_token.clone()),
            short_name: self.short_name.clone(),
            author_name: Some(self.author_name.clone()),
            author_url: self.author_url.clone(),
        }
    }

    /// Use this method to edit an existing Telegraph page.
    ///
    /// On success, returns a Page object.
    pub fn edit_page(
        &self,
        path: &str,
        title: &str,
        content: &str,
        return_content: bool,
    ) -> Result<Page> {
        let mut response = self
            .client
            .post("https://api.telegra.ph/editPage")
            .form(&[
                ("access_token", &*self.access_token),
                ("path", path),
                ("title", title),
                ("author_name", &*self.author_name),
                (
                    "author_url",
                    self.author_url.as_ref().map(|s| &**s).unwrap_or(""),
                ),
                ("content", content),
                ("return_content", &*return_content.to_string()),
            ])
            .send()?;
        response.json::<ApiResult<Page>>()?.into()
    }

    /// Use this method to get information about a Telegraph account. Returns an Account object on success.
    ///
    /// Available fields: short_name, author_name, author_url, auth_url, page_count.
    pub fn get_account_info(&self, fields: &[&str]) -> Result<Account> {
        let mut response = self
            .client
            .get("https://api.telegra.ph/getAccountInfo")
            .query(&[
                ("access_token", &self.access_token),
                ("fields", &serde_json::to_string(fields).unwrap()),
            ])
            .send()?;
        response.json::<ApiResult<Account>>()?.into()
    }

    /// Use this method to get a Telegraph page. Returns a Page object on success.
    pub fn get_page(path: &str, return_content: bool) -> Result<Page> {
        let mut response = Client::new()
            .get(&format!("https://api.telegra.ph/getPage/{}", path))
            .query(&[("return_content", return_content.to_string())])
            .send()?;
        response.json::<ApiResult<Page>>()?.into()
    }

    /// Use this method to get a list of pages belonging to a Telegraph account.
    ///
    /// Returns a PageList object, sorted by most recently created pages first.
    ///
    /// - `offset` Sequential number of the first page to be returned. (suggest: 0)
    /// - `limit` Limits the number of pages to be retrieved. (suggest: 50)
    pub fn get_page_list(&self, offset: i32, limit: i32) -> Result<PageList> {
        let mut response = self
            .client
            .get("https://api.telegra.ph/getPageList")
            .query(&[
                ("access_token", &self.access_token),
                ("offset", &offset.to_string()),
                ("limit", &limit.to_string()),
            ])
            .send()?;
        response.json::<ApiResult<PageList>>()?.into()
    }

    /// Use this method to get the number of views for a Telegraph article.
    ///
    /// Returns a PageViews object on success.
    ///
    /// By default, the total number of page views will be returned.
    ///
    /// ```rust
    /// use telegraph_rs::blocking::Telegraph;
    ///
    /// let view1 = Telegraph::get_views("Sample-Page-12-15", &vec![2016, 12]);
    /// let view2 = Telegraph::get_views("Sample-Page-12-15", &vec![2019, 5, 19, 12]); // year-month-day-hour
    /// ```
    pub fn get_views(path: &str, time: &[i32]) -> Result<PageViews> {
        let params = ["year", "month", "day", "hour"]
            .iter()
            .zip(time)
            .collect::<HashMap<_, _>>();

        let mut response = Client::new()
            .get(&format!("https://api.telegra.ph/getViews/{}", path))
            .query(&params)
            .send()?;
        response.json::<ApiResult<PageViews>>()?.into()
    }

    /// Use this method to revoke access_token and generate a new one,
    ///
    /// for example, if the user would like to reset all connected sessions,
    ///
    /// or you have reasons to believe the token was compromised.
    ///
    /// On success, returns an Account object with new access_token and auth_url fields.
    pub fn revoke_access_token(&mut self) -> Result<Account> {
        let mut response = self
            .client
            .get("https://api.telegra.ph/revokeAccessToken")
            .query(&[("access_token", &self.access_token)])
            .send()?;
        let json: Result<Account> = response.json::<ApiResult<Account>>()?.into();
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

    /// Upload files to telegraph
    pub fn upload<P: AsRef<Path>>(files: &[P]) -> Result<Vec<UploadResult>> {
        let mut form = Form::new();
        for (idx, file) in files.into_iter().enumerate() {
            let part = Part::file(file)?;
            form = form.part(idx.to_string(), part);
        }
        let mut response = Client::new()
            .post("https://telegra.ph/upload")
            .multipart(form)
            .send()?;

        Ok(response.json::<Vec<UploadResult>>()?)
    }
}

#[cfg(test)]
mod tests {
    use crate::blocking::Telegraph;

    #[test]
    fn create_and_revoke_account() {
        let result = Telegraph::create_account("sample", "a", None);
        println!("{:?}", result);
        assert!(result.is_ok());

        let mut telegraph = Telegraph::new("test")
            .access_token(&result.unwrap().access_token.unwrap().to_owned())
            .create()
            .unwrap();
        let result = telegraph.revoke_access_token();
        println!("{:?}", result);
        assert!(result.is_ok());
    }

    #[test]
    fn edit_account_info() {
        let result = Telegraph::new("test")
            .access_token("b968da509bb76866c35425099bc0989a5ec3b32997d55286c657e6994bbb")
            .create()
            .unwrap()
            .edit_account_info()
            .short_name("wow")
            .edit();
        println!("{:?}", result);
        assert!(result.is_ok());
    }

    #[test]
    fn get_account_info() {
        let result = Telegraph::new("test")
            .access_token("b968da509bb76866c35425099bc0989a5ec3b32997d55286c657e6994bbb")
            .create()
            .unwrap()
            .get_account_info(&["short_name"]);
        println!("{:?}", result);
        assert!(result.is_ok());
    }

    #[test]
    fn create_get_edit_page() {
        let telegraph = Telegraph::new("test")
            .access_token("b968da509bb76866c35425099bc0989a5ec3b32997d55286c657e6994bbb")
            .create()
            .unwrap();
        let page = telegraph.create_page(
            "OVO",
            r#"[{"tag":"p","children":["Hello,+world!"]}]"#,
            false,
        );
        println!("{:?}", page);
        assert!(page.is_ok());

        let page = Telegraph::get_page(&page.unwrap().path, true);
        println!("{:?}", page);
        assert!(page.is_ok());

        let page = telegraph.edit_page(
            &page.unwrap().path,
            "QAQ",
            r#"[{"tag":"p","children":["Goodbye,+world!"]}]"#,
            false,
        );
        println!("{:?}", page);
        assert!(page.is_ok());
    }

    #[test]
    fn get_page_list() {
        let telegraph = Telegraph::new("test")
            .access_token("b968da509bb76866c35425099bc0989a5ec3b32997d55286c657e6994bbb")
            .create()
            .unwrap();
        let page_list = telegraph.get_page_list(0, 3);
        println!("{:?}", page_list);
        assert!(page_list.is_ok());
    }

    #[test]
    fn get_views() {
        let views = Telegraph::get_views("Sample-Page-12-15", &vec![2016, 12]);
        println!("{:?}", views);
        assert!(views.is_ok());
    }

    #[ignore]
    #[test]
    fn upload() {
        let images = Telegraph::upload(&vec!["1.jpeg", "2.jpeg"]);
        println!("{:?}", images);
    }
}
