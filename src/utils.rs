/// Parse html to node string
///
/// ```rust
/// use telegraph_rs::html_to_node;
///
/// let node = html_to_node("<p>Hello, world</p>");
/// assert_eq!(node, r#"[{"tag":"p","children":["Hello, world"]}]"#);
/// ```
#[cfg(feature = "html")]
pub fn html_to_node(html: &str) -> String {
    use html_parser::Dom;

    let dom = Dom::parse(html).unwrap();
    let nodes = dom
        .children
        .into_iter()
        .map(|node| crate::html_to_node_inner(&node))
        .collect::<Vec<_>>();
    serde_json::to_string(&nodes).unwrap()
}
