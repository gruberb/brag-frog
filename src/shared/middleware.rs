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
        HeaderValue::from_static("default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data: https://*.googleusercontent.com"),
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

/// CSRF protection: reject state-changing requests that lack the HX-Request header.
/// HTMX sends this header automatically. Cross-origin requests cannot set custom
/// headers without a CORS preflight, which the server does not allow.
pub async fn csrf_protection(request: Request, next: Next) -> Response {
    let method = request.method().clone();

    // Only check state-changing methods
    if method == Method::GET || method == Method::HEAD || method == Method::OPTIONS {
        return next.run(request).await;
    }

    // Allow OAuth callback (GET-only, but be safe) and logout
    let path = request.uri().path().to_string();
    if path.starts_with("/auth/") {
        return next.run(request).await;
    }

    // Require HX-Request header for all other state-changing requests
    let has_hx = request.headers().get("HX-Request").is_some();
    if !has_hx {
        return (StatusCode::FORBIDDEN, "CSRF validation failed").into_response();
    }

    next.run(request).await
}
