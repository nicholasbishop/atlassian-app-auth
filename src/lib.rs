use serde::Serialize;
use sha2::Digest;
use std::time;
use url::Url;

// See
// https://developer.atlassian.com/cloud/jira/platform/understanding-jwt
// for details of how Atlassian implements JWT.

/// Input parameters for creating a JWT.
pub struct Parameters {
    /// HTTP of the request.
    pub method: String,

    /// URL of the request.
    pub url: Url,

    /// Duration that this key will be valid for (starting from the
    /// current time)
    pub valid_for: time::Duration,

    /// Connect App key. This is the same as the "key" field
    /// of the app descriptor JSON file, and is also returned in the
    /// "key" field of the installation lifecycle callback.
    pub app_key: String,

    /// Connect App shared secret. This is returned in the
    /// "sharedSecret" field of the installation lifecycle callback.
    pub shared_secret: String,
}

/// Authentication error enum.
#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    /// An error occurred when trying to encode the JWT.
    #[error("JWT encoding failed: {0}")]
    JwtError(#[from] jsonwebtoken::errors::Error),

    /// Something very unexpected happened with time itself.
    #[error("system time error: {0}")]
    TimeError(#[from] time::SystemTimeError),
}

// TODO: there are quite a few special cases described in the doc
// linked above that are not yet handled here.
fn create_query_string_hash(params: &Parameters) -> String {
    let url = &params.url;
    let method = params.method.as_str().to_uppercase();
    // Assume the path is already canonical
    let path = url.path();

    let mut query_pairs = url
        .query_pairs()
        .map(|(key, val)| format!("{}={}", key, val))
        .collect::<Vec<_>>();
    query_pairs.sort_unstable();

    let canonical_request = format!("{}&{}&{}", method, path, query_pairs.join("&"));

    format!("{:x}", sha2::Sha256::digest(canonical_request.as_bytes()))
}

#[derive(Debug, Serialize)]
struct Claims {
    /// The issuer of the claim. This matches the key in the app
    /// descriptor ("com.neverware.crash").
    iss: String,

    /// Custom Atlassian claim that prevents URL tampering.
    qsh: String,

    /// The time that this JWT was issued.
    iat: u64,

    /// JWT expiration time.
    exp: u64,
}

impl Claims {
    fn new(params: &Parameters) -> Result<Claims, AuthError> {
        let now = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)?
            .as_secs();
        Ok(Claims {
            iss: params.app_key.clone(),
            qsh: create_query_string_hash(params),

            // The time that this JWT was issued (now)
            iat: now,

            // JWT expiration time
            exp: now + params.valid_for.as_secs(),
        })
    }
}

/// Request header.
pub struct Header {
    /// Header name.
    pub name: String,
    /// Header value.
    pub value: String,
}

pub fn create_auth_header(params: &Parameters) -> Result<Header, AuthError> {
    let claims = Claims::new(params)?;

    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(params.shared_secret.as_bytes()),
    )?;

    Ok(Header {
        name: "Authorization".into(),
        value: format!("JWT {}", token),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_string_hash() {
        let params = Parameters {
            method: "get".into(),
            url: Url::parse(
                "https://somecorp.atlassian.net/rest/api/3/project/search?query=myproject",
            )
            .unwrap(),
            app_key: String::new(),
            shared_secret: String::new(),
            valid_for: time::Duration::new(0, 0),
        };
        assert_eq!(
            create_query_string_hash(&params),
            "29df35d41afcc61d322eba090286bf96b42fa3d7b5b5d1d2d261083d1cefd7fe"
        );
    }
}
