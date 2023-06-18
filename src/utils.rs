use libxml::parser::Parser;
use std::{fs::File, io::Read, path::Path};

#[cfg(feature = "upload")]
pub fn guess_mime<P: AsRef<Path>>(path: P) -> String {
    let mime = mime_guess::from_path(path).first_or(mime_guess::mime::TEXT_PLAIN);
    let mut s = format!("{}/{}", mime.type_(), mime.subtype());
    if let Some(suffix) = mime.suffix() {
        s.push('+');
        s.push_str(suffix.as_str());
    }
    s
}

#[cfg(feature = "upload")]
pub fn read_to_bytes<P: AsRef<Path>>(path: P) -> crate::Result<Vec<u8>> {
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
        .map(|node| crate::html_to_node_inner(&node))
        .collect::<Vec<_>>();
    serde_json::to_string(&nodes).unwrap()
}
