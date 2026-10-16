use std::net::IpAddr;

use crate::kernel::error::AppError;

/// Validates a user-provided base URL against SSRF.
/// Rejects non-HTTPS, localhost, and private/reserved IP ranges.
pub fn validate_base_url(url_str: &str) -> Result<(), AppError> {
    let url = url::Url::parse(url_str)
        .map_err(|_| AppError::BadRequest("Invalid base URL".to_string()))?;

    if url.scheme() != "https" {
        return Err(AppError::BadRequest("Base URL must use HTTPS".to_string()));
    }

    if let Some(host) = url.host_str() {
        if host == "localhost" || host == "127.0.0.1" || host == "::1" || host == "[::1]" {
            return Err(AppError::BadRequest(
                "Base URL must not point to localhost".to_string(),
            ));
        }

        if let Ok(ip) = host.parse::<IpAddr>()
            && is_private_ip(&ip)
        {
            return Err(AppError::BadRequest(
                "Base URL must not point to a private IP address".to_string(),
            ));
        }
    }

    Ok(())
}

/// Returns true for loopback, RFC-1918, link-local, CGNAT, and unspecified addresses.
fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
            || v4.is_private()
            || v4.is_link_local()
            || v4.is_unspecified()
            || v4.octets()[0] == 100 && (v4.octets()[1] & 0xC0) == 64
        }
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified(),
    }
}
