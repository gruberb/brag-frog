use std::collections::HashMap;

use axum::{
    extract::State,
    http::header,
    response::{Html, IntoResponse},
};

use super::error::AppError;
use crate::AppState;

/// Build an HX-Redirect response for HTMX POST handlers.
pub fn hx_redirect(path: &'static str) -> impl IntoResponse {
    ([(header::HeaderName::from_static("hx-redirect"), path)], "")
}

/// Renders the privacy policy page.
pub async fn static_page_privacy(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let ctx = tera::Context::new();
    let html = state.templates.render("pages/privacy.html", &ctx)?;
    Ok(Html(html))
}

/// Renders the terms of service page.
pub async fn static_page_terms(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let ctx = tera::Context::new();
    let html = state.templates.render("pages/terms.html", &ctx)?;
    Ok(Html(html))
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
