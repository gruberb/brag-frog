use axum::{
    extract::Request,
    http::{HeaderValue, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Axum middleware that injects CSP, X-Frame-Options, nosniff, and Referrer-Policy headers.
pub async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        axum::http::header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; font-src 'self'; img-src 'self' data: https://*.googleusercontent.com; connect-src 'self'; frame-ancestors 'none'"),
    );
    headers.insert(
        axum::http::header::X_FRAME_OPTIONS,
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        axum::http::header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        axum::http::header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    response
}

/// Header the web app sends on every API request. Cross-origin pages cannot set a custom
/// header without a CORS preflight, which the server never allows.
pub const API_CSRF_HEADER: &str = "X-Requested-With";
pub const API_CSRF_VALUE: &str = "brag-frog";

/// CSRF protection for state-changing API requests: they need `X-Requested-With: brag-frog`.
/// Requests the browser marks `Sec-Fetch-Site: cross-site` are always refused.
pub async fn csrf_protection(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let safe_method = method == Method::GET || method == Method::HEAD || method == Method::OPTIONS;
    if safe_method {
        return next.run(request).await;
    }

    let cross_site = request
        .headers()
        .get("Sec-Fetch-Site")
        .is_some_and(|v| v == "cross-site");
    let has_header = request
        .headers()
        .get(API_CSRF_HEADER)
        .is_some_and(|v| v == API_CSRF_VALUE);
    if cross_site || !has_header {
        return (
            StatusCode::FORBIDDEN,
            [(axum::http::header::CONTENT_TYPE, "application/json")],
            r#"{"error":{"code":"csrf","message":"CSRF validation failed"}}"#,
        )
            .into_response();
    }

    next.run(request).await
}
