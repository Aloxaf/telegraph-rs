use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct Account {
    pub short_name: Option<String>,
    pub author_name: Option<String>,
    pub author_url: Option<String>,
    pub access_token: Option<String>,
    pub auth_url: Option<String>,
    pub page_count: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PageList {
    pub total_count: i32,
    pub pages: Vec<Page>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Page {
    pub path: String,
    pub url: String,
    pub title: String,
    pub description: String,
    pub author_name: Option<String>,
    pub author_url: Option<String>,
    pub image_url: Option<String>,
    pub content: Option<Vec<Node>>,
    pub views: i32,
    pub can_edit: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PageViews {
    pub views: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Node {
    Text(String),
    NodeElement(NodeElement),
}

#[derive(Debug, Clone, Deserialize)]
pub struct NodeElement {
    pub tag: String,
    pub attrs: Option<HashMap<String, String>>,
    pub children: Option<Vec<Node>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UploadResult {
    pub src: String
}