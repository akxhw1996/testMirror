use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::collections::HashMap;
use std::str::FromStr;

pub fn send_request(
    method: &str,
    url: &str,
    body: Option<String>,
    headers: Option<HashMap<String, String>>,
) -> Result<String, reqwest::Error> {
    let client = Client::new();
    let method = reqwest::Method::from_str(method).unwrap();

    let mut request = client.request(method, url);

    if let Some(body) = body {
        request = request.body(body);
    }

    if let Some(headers) = headers {
        let mut header_map = HeaderMap::new();
        for (key, value) in headers {
            let header_name = HeaderName::from_str(&key).unwrap();
            let header_value = HeaderValue::from_str(&value).unwrap();
            header_map.insert(header_name, header_value);
        }
        request = request.headers(header_map);
    }

    let response = request.send()?;
    let response_body = response.text()?;

    Ok(response_body)
}
