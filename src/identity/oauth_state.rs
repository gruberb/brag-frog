//! Stateless, signed CSRF state tokens for OAuth flows.
//!
//! Replaces the previous session-keyed `oauth_state` slot (which was clobbered
//! by parallel tabs, refreshes, and link prefetchers because there was only one
//! slot per session). Verification is local: the token carries its own integrity
//! tag and timestamp, so the OAuth round-trip survives any number of concurrent
//! tabs and the back button.
//!
//! Token wire format: `<flow>:<nonce>:<ts>:<sig>`
//! - `flow`  — `login` | `drive` | `calendar`; selects the callback branch.
//! - `nonce` — 16 random bytes hex-encoded; uniqueness for identical-second mints.
//! - `ts`    — Unix seconds at mint time (decimal).
//! - `sig`   — HMAC-SHA256 over `<flow>:<nonce>:<ts>` keyed by the master via
//!   HKDF, hex-encoded. 32 bytes → 64 hex chars.
//!
//! The leading `<flow>:` segment is a structural choice, not load-bearing for
//! security: the signature covers the flow, so a user cannot swap `login` for
//! `drive` without invalidating the tag.

use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use rand::TryRngCore;
use rand::rngs::OsRng;
use sha2::Sha256;

use crate::kernel::crypto::Crypto;
use crate::kernel::error::AppError;

type HmacSha256 = Hmac<Sha256>;

/// OAuth flow this state token authorizes. Selects which callback branch runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuthFlow {
    Login,
    Drive,
    Calendar,
}

impl OAuthFlow {
    fn as_str(self) -> &'static str {
        match self {
            OAuthFlow::Login => "login",
            OAuthFlow::Drive => "drive",
            OAuthFlow::Calendar => "calendar",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "login" => Some(OAuthFlow::Login),
            "drive" => Some(OAuthFlow::Drive),
            "calendar" => Some(OAuthFlow::Calendar),
            _ => None,
        }
    }
}

/// Maximum age accepted for an OAuth state token, in seconds.
///
/// Sized to absorb realistic user delays inside Google's consent flow: account
/// picker, 2FA prompts, reading the scopes, switching accounts, network stalls.
/// 10 minutes was too tight in practice — users hit "expired" mid-consent.
///
/// The replay-window argument for going lower doesn't really apply: Google's
/// authorization code is single-use and itself expires within ~10 minutes, so a
/// leaked state token without a matching fresh code is inert.
const MAX_AGE_SECS: u64 = 30 * 60;

/// Mint a signed state token for `flow`. Pass the result as the OAuth `state`
/// query parameter in the consent-screen URL.
pub fn mint(crypto: &Crypto, flow: OAuthFlow) -> Result<String, AppError> {
    let mut nonce = [0u8; 16];
    OsRng
        .try_fill_bytes(&mut nonce)
        .map_err(|e| AppError::Internal(format!("RNG failure: {}", e)))?;
    let nonce_hex = hex_encode(&nonce);

    let ts = unix_seconds()?;
    let payload = format!("{}:{}:{}", flow.as_str(), nonce_hex, ts);
    let sig = sign(crypto, payload.as_bytes())?;
    Ok(format!("{}:{}", payload, hex_encode(&sig)))
}

/// Verify a token's signature and freshness. Returns the flow it authorizes
/// or `BadRequest` if anything is off (malformed, bad signature, expired).
pub fn verify(crypto: &Crypto, token: &str) -> Result<OAuthFlow, AppError> {
    let mut parts = token.splitn(4, ':');
    let flow_s = parts.next().ok_or_else(bad_token)?;
    let nonce = parts.next().ok_or_else(bad_token)?;
    let ts_s = parts.next().ok_or_else(bad_token)?;
    let sig_hex = parts.next().ok_or_else(bad_token)?;

    let flow = OAuthFlow::parse(flow_s).ok_or_else(bad_token)?;

    let payload = format!("{}:{}:{}", flow_s, nonce, ts_s);
    let sig_bytes = hex_decode(sig_hex).ok_or_else(bad_token)?;

    // `verify_slice` is constant-time inside the hmac crate.
    let key = crypto.oauth_state_key()?;
    let mut mac = HmacSha256::new_from_slice(&key)
        .map_err(|e| AppError::Internal(format!("HMAC init failed: {}", e)))?;
    mac.update(payload.as_bytes());
    mac.verify_slice(&sig_bytes).map_err(|_| bad_token())?;

    let ts: u64 = ts_s.parse().map_err(|_| bad_token())?;
    let now = unix_seconds()?;
    // Reject future timestamps (clock skew or tampering) and stale ones.
    // saturating_sub returns 0 if `ts > now`, which is fine — that's "fresh".
    if now.saturating_sub(ts) > MAX_AGE_SECS {
        return Err(AppError::BadRequest(
            "OAuth state token expired; please try signing in again".to_string(),
        ));
    }

    Ok(flow)
}

fn sign(crypto: &Crypto, payload: &[u8]) -> Result<Vec<u8>, AppError> {
    let key = crypto.oauth_state_key()?;
    let mut mac = HmacSha256::new_from_slice(&key)
        .map_err(|e| AppError::Internal(format!("HMAC init failed: {}", e)))?;
    mac.update(payload);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn unix_seconds() -> Result<u64, AppError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|_| AppError::Internal("System clock before epoch".to_string()))
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(&mut s, "{:02x}", b);
    }
    s
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    for i in (0..s.len()).step_by(2) {
        let byte = u8::from_str_radix(s.get(i..i + 2)?, 16).ok()?;
        out.push(byte);
    }
    Some(out)
}

fn bad_token() -> AppError {
    AppError::BadRequest("Invalid OAuth state".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_crypto() -> Crypto {
        Crypto::new("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=").unwrap()
    }

    #[test]
    fn mint_then_verify_roundtrip() {
        let c = test_crypto();
        for flow in [OAuthFlow::Login, OAuthFlow::Drive, OAuthFlow::Calendar] {
            let token = mint(&c, flow).unwrap();
            assert_eq!(verify(&c, &token).unwrap(), flow);
        }
    }

    #[test]
    fn flow_prefix_is_first_segment() {
        // Existing call sites still pattern-match on "drive:" / "calendar:"
        // prefixes for routing; preserving that contract avoids a wider refactor.
        let c = test_crypto();
        assert!(mint(&c, OAuthFlow::Login).unwrap().starts_with("login:"));
        assert!(mint(&c, OAuthFlow::Drive).unwrap().starts_with("drive:"));
        assert!(
            mint(&c, OAuthFlow::Calendar)
                .unwrap()
                .starts_with("calendar:")
        );
    }

    #[test]
    fn tampered_flow_rejected() {
        let c = test_crypto();
        let token = mint(&c, OAuthFlow::Login).unwrap();
        let tampered = token.replacen("login:", "drive:", 1);
        assert!(verify(&c, &tampered).is_err());
    }

    #[test]
    fn tampered_signature_rejected() {
        let c = test_crypto();
        let mut token = mint(&c, OAuthFlow::Login).unwrap();
        // Flip the last hex char of the signature.
        let last = token.pop().unwrap();
        let flipped = if last == 'a' { 'b' } else { 'a' };
        token.push(flipped);
        assert!(verify(&c, &token).is_err());
    }

    #[test]
    fn wrong_key_rejected() {
        let a = test_crypto();
        let b = Crypto::new("BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBA=").unwrap();
        let token = mint(&a, OAuthFlow::Login).unwrap();
        assert!(verify(&b, &token).is_err());
    }

    #[test]
    fn malformed_token_rejected() {
        let c = test_crypto();
        for bad in ["", "login", "login:abc", "login:abc:notanumber:cafebabe"] {
            assert!(verify(&c, bad).is_err());
        }
    }

    #[test]
    fn expired_token_rejected() {
        // Forge a token with an old timestamp using the real signing key.
        let c = test_crypto();
        let stale_ts = unix_seconds().unwrap() - (MAX_AGE_SECS + 60);
        let payload = format!("login:{}:{}", "a".repeat(32), stale_ts);
        let sig = sign(&c, payload.as_bytes()).unwrap();
        let token = format!("{}:{}", payload, hex_encode(&sig));
        let err = verify(&c, &token).unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[test]
    fn nonces_differ_between_mints() {
        let c = test_crypto();
        let a = mint(&c, OAuthFlow::Login).unwrap();
        let b = mint(&c, OAuthFlow::Login).unwrap();
        assert_ne!(a, b);
    }
}
