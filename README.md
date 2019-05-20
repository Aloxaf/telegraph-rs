# telegraph-rs

[![crates.io](https://img.shields.io/crates/v/telegraph-rs.svg)](https://crates.io/crates/telegraph-rs)
[![Documentation](https://docs.rs/telegraph-rs/badge.svg)](https://docs.rs/telegraph-rs)

telegraph binding in Rust

see https://telegra.ph/api for more information

## Examples

```rust
use telegraph_rs::{Telegraph, html_to_node};

let telegraph = Telegraph::new("test_account").create().unwrap();

let page = telegraph.create_page("title", &html_to_node("<p>Hello, world</p>"), false).unwrap();
```
