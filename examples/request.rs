use reqwest::blocking::Client;
use reqwest::Method;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

// Example of running this example:
//
// cargo run --example request -- <creds-path> <method> <url>

/// Send a request to Jira and pretty-print the JSON response.
#[derive(argh::FromArgs)]
struct Opt {
    /// path of the JSON credentials file containing the key and
    /// secret key
    #[argh(positional)]
    creds: PathBuf,

    /// http method such as "get"
    #[argh(positional)]
    method: String,

    /// url such as https://mycorp.atlassian.net/rest/api/3/project/search?query=KEY
    #[argh(positional)]
    url: String,
}

// TODO: add a way to send a JSON body

#[derive(Deserialize)]
struct Creds {
    key: String,
    secret: String,
}

fn main() {
    let opt: Opt = argh::from_env();

    // Read the credentials
    let creds_raw =
        fs::read_to_string(opt.creds).expect("failed to read creds file");
    let creds: Creds =
        serde_json::from_str(&creds_raw).expect("failed to parse creds file");

    // Create the request
    let client = Client::new();
    let method = Method::from_bytes(opt.method.to_uppercase().as_bytes())
        .expect("invalid method");
    let mut request = client
        .request(method, &opt.url)
        .build()
        .expect("invalid request");

    // Add the auth header
    let header = atlassian_app_auth::create_auth_header(
        &atlassian_app_auth::Parameters {
            method: request.method().as_str().into(),
            url: request.url().clone(),
            valid_for: Duration::from_secs(30),
            app_key: creds.key.clone(),
            shared_secret: creds.secret.clone(),
        },
    )
    .expect("failed to create auth header");
    request.headers_mut().insert(
        "Authorization",
        header.value.parse().expect("failed to parse auth value"),
    );

    // Send the request and print the response
    let resp = client.execute(request).expect("failed to send request");
    match resp.error_for_status_ref() {
        Ok(_) => {
            // Pretty-print the response
            let resp: serde_json::Value =
                resp.json().expect("failed to parse response");
            println!(
                "{}",
                serde_json::to_string_pretty(&resp)
                    .expect("failed to format response")
            );
        }
        Err(err) => {
            // Print an error including the body of the request
            println!(
                "request failed: {}, body: {}",
                err,
                resp.text().expect("failed to get body")
            );
        }
    }
}
