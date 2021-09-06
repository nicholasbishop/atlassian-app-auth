use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use serde::Serialize;
use sha2::Digest;
use std::time;
use url::Url;

/// The set of characters to percent-encode for query parameters. The
/// Jira documentation says these should be consistent with OAuth 1.0,
/// which is defined in RFC 5849.
///
/// From https://tools.ietf.org/html/rfc5849#page-29:
/// * (ALPHA, DIGIT, "-", ".", "_", "~") MUST NOT be encoded
/// * All other characters MUST be encoded.
pub const QUERY_PARAM_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'!')
    .add(b'"')
    .add(b'#')
    .add(b'$')
    .add(b'%')
    .add(b'&')
    .add(b'\'')
    .add(b'(')
    .add(b')')
    .add(b'*')
    .add(b'+')
    .add(b',')
    .add(b'/')
    .add(b':')
    .add(b';')
    .add(b'<')
    .add(b'=')
    .add(b'>')
    .add(b'?')
    .add(b'@')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}');

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
fn create_canonical_request(params: &Parameters) -> String {
    let url = &params.url;
    let method = params.method.as_str().to_uppercase();
    // Assume the path is already canonical
    let path = url.path();

    let mut query_pairs = url
        .query_pairs()
        .map(|(key, val)| {
            format!(
                "{}={}",
                key,
                utf8_percent_encode(&val, QUERY_PARAM_ENCODE_SET)
            )
        })
        .collect::<Vec<_>>();
    query_pairs.sort_unstable();

    format!("{}&{}&{}", method, path, query_pairs.join("&"))
}

fn create_query_string_hash(params: &Parameters) -> String {
    let canonical_request = create_canonical_request(params);
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
    pub name: &'static str,
    /// Header value.
    pub value: String,
}

pub fn create_auth_header(params: &Parameters) -> Result<Header, AuthError> {
    let claims = Claims::new(params)?;

    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(
            params.shared_secret.as_bytes(),
        ),
    )?;

    Ok(Header {
        name: "Authorization",
        value: format!("JWT {}", token),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_params(method: &str, url: &str) -> Parameters {
        Parameters {
            method: method.into(),
            url: Url::parse(url).unwrap(),
            app_key: String::new(),
            shared_secret: String::new(),
            valid_for: time::Duration::new(0, 0),
        }
    }

    #[test]
    fn test_canonical_request() {
        let params = create_params(
            "get",
            "https://somecorp.atlassian.net/rest/api/3/project/search?query=myproject",
        );
        assert_eq!(
            create_canonical_request(&params),
            "GET&/rest/api/3/project/search&query=myproject"
        );
    }

    #[test]
    fn test_canonical_request_query_params_encoding() {
        let params = create_params(
            "get",
            "https://example.com/example?query=x y,z%2B*~",
        );
        assert_eq!(
            create_canonical_request(&params),
            "GET&/example&query=x%20y%2Cz%2B%2A~"
        );
    }

    #[test]
    fn test_query_string_hash() {
        let params = create_params("get", "https://example.com/example");
        assert_eq!(
            create_query_string_hash(&params),
            "0073e2edb5df6a8af18c4398d32532f2b46a05295d10fac402131dd044032a61"
        );
    }
}
