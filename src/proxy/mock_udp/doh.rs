use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE};
use reqwest::Client;

pub async fn doh(req_wireformat: &[u8]) -> reqwest::Result<Vec<u8>> {
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/dns-message"),
    );
    headers.insert(ACCEPT, HeaderValue::from_static("application/dns-message"));
    let client = Client::new();
    let response = client
        .post("https://1.1.1.1/dns-query")
        .headers(headers)
        .body(req_wireformat.to_vec())
        .send()
        .await?
        .bytes()
        .await?;

    Ok(response.to_vec())
}
