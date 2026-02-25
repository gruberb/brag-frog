use std::collections::HashMap;

use axum::{
    http::header,
    response::IntoResponse,
};

/// Build an HX-Redirect response for HTMX POST handlers.
pub fn hx_redirect(path: &'static str) -> impl IntoResponse {
    ([(header::HeaderName::from_static("hx-redirect"), path)], "")
}

/// Tera filter: renders Markdown to HTML via pulldown-cmark, then sanitizes with ammonia.
pub fn markdown_filter(
    value: &tera::Value,
    _args: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    let text = tera::try_get_value!("markdown", "value", String, value);
    let parser = pulldown_cmark::Parser::new(&text);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    let clean = ammonia::clean(&html);
    Ok(tera::Value::String(clean))
}
