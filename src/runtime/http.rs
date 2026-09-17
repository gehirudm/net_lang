use super::Value;
use crate::ast::HttpMethod;
use reqwest::{
    Method, Url,
    blocking::Client,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use std::{collections::BTreeMap, io::Read, time::Duration};

const MAX_BODY_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Clone, Default)]
pub(super) struct HttpRuntime {
    client: Option<Client>,
}

struct Options {
    headers: HeaderMap,
    query: Vec<(String, String)>,
    json: Option<serde_json::Value>,
    timeout: Duration,
    retries: usize,
}

impl HttpRuntime {
    pub fn request(
        &mut self,
        method: HttpMethod,
        url: &str,
        config: &Value,
    ) -> Result<Value, String> {
        let options = Options::parse(config)?;
        let url = Url::parse(url)
            .map_err(|_| "request URL must be an absolute HTTP(S) URL".to_string())?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err("request URL must be an absolute HTTP(S) URL".into());
        }
        if self.client.is_none() {
            self.client = Some(
                Client::builder()
                    .timeout(Duration::from_secs(30))
                    .redirect(reqwest::redirect::Policy::none())
                    .retry(reqwest::retry::never())
                    .no_proxy()
                    .build()
                    .map_err(|error| format!("cannot initialize HTTP runtime: {error}"))?,
            );
        }
        let client = self.client.as_ref().unwrap();
        let method = match method {
            HttpMethod::Get => Method::GET,
            HttpMethod::Post => Method::POST,
            HttpMethod::Put => Method::PUT,
            HttpMethod::Patch => Method::PATCH,
            HttpMethod::Delete => Method::DELETE,
            HttpMethod::Head => Method::HEAD,
        };
        for attempt in 0..=options.retries {
            let mut request = client
                .request(method.clone(), url.clone())
                .headers(options.headers.clone())
                .query(&options.query)
                .timeout(options.timeout);
            if let Some(json) = &options.json {
                request = request.json(json);
            }
            let response = match request.send() {
                Ok(response) => response,
                Err(error)
                    if attempt < options.retries && (error.is_timeout() || error.is_connect()) =>
                {
                    continue;
                }
                Err(error) => return Err(format!("HTTP request failed: {}", error.without_url())),
            };
            let status = response.status().as_u16();
            if attempt < options.retries && matches!(status, 429 | 502 | 503 | 504) {
                continue;
            }
            let mut headers = BTreeMap::new();
            for (name, value) in response.headers() {
                let value = value
                    .to_str()
                    .map_err(|_| "response header is not text".to_string())?;
                // Preserve repeated headers as an array rather than losing values.
                match headers.entry(name.as_str().into()) {
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        entry.insert(Value::String(value.into()));
                    }
                    std::collections::btree_map::Entry::Occupied(mut entry) => {
                        let previous = entry.get_mut();
                        match previous {
                            Value::Array(values) => values.push(Value::String(value.into())),
                            _ => {
                                *previous = Value::Array(vec![
                                    previous.clone(),
                                    Value::String(value.into()),
                                ])
                            }
                        }
                    }
                }
            }
            let mut bytes = Vec::new();
            response
                .take(MAX_BODY_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(|error| format!("cannot read HTTP response body: {error}"))?;
            if bytes.len() as u64 > MAX_BODY_BYTES {
                return Err("HTTP response body exceeds 8 MiB limit".into());
            }
            let body = String::from_utf8(bytes)
                .map_err(|_| "HTTP response body is not UTF-8 text".to_string())?;
            return Ok(Value::Object(BTreeMap::from([
                ("status".into(), Value::Integer(i64::from(status))),
                ("headers".into(), Value::Object(headers)),
                ("body".into(), Value::String(body)),
            ])));
        }
        unreachable!("the final attempt always returns")
    }
}

impl Options {
    fn parse(config: &Value) -> Result<Self, String> {
        let Value::Object(fields) = config else {
            return Err("request configuration must be an object".into());
        };
        let mut options = Self {
            headers: HeaderMap::new(),
            query: Vec::new(),
            json: None,
            timeout: Duration::from_secs(30),
            retries: 0,
        };
        for (key, value) in fields {
            match key.as_str() {
                "headers" => {
                    let Value::Object(headers) = value else {
                        return Err("headers must be an object".into());
                    };
                    for (name, value) in headers {
                        let Value::String(value) = value else {
                            return Err("header values must be strings".into());
                        };
                        let name = HeaderName::from_bytes(name.as_bytes())
                            .map_err(|_| "invalid HTTP header name".to_string())?;
                        let value = HeaderValue::from_str(value)
                            .map_err(|_| "invalid HTTP header value".to_string())?;
                        options.headers.insert(name, value);
                    }
                }
                "query" => {
                    let Value::Object(query) = value else {
                        return Err("query must be an object".into());
                    };
                    for (name, value) in query {
                        if !matches!(
                            value,
                            Value::String(_)
                                | Value::Integer(_)
                                | Value::Float(_)
                                | Value::Boolean(_)
                        ) {
                            return Err("query values must be strings, numbers, or booleans".into());
                        }
                        options.query.push((name.clone(), value.to_string()));
                    }
                }
                "json" => options.json = Some(json_value(value)?),
                "timeout" => {
                    let Value::Duration(ms @ 1..=86_400_000) = value else {
                        return Err(
                            "timeout must be a positive duration of at most 24 hours".into()
                        );
                    };
                    options.timeout = Duration::from_millis(*ms);
                }
                "retry" => {
                    let Value::Integer(count @ 0..=10) = value else {
                        return Err("retry must be an integer from 0 to 10".into());
                    };
                    options.retries = *count as usize;
                }
                _ => return Err(format!("unsupported request option '{key}'")),
            }
        }
        Ok(options)
    }
}

fn json_value(value: &Value) -> Result<serde_json::Value, String> {
    use serde_json::Value as Json;
    Ok(match value {
        Value::Null => Json::Null,
        Value::Boolean(v) => Json::Bool(*v),
        Value::Integer(v) => Json::Number((*v).into()),
        Value::String(v) => Json::String(v.clone()),
        Value::Float(v) => Json::Number(
            serde_json::Number::from_f64(*v)
                .ok_or_else(|| "JSON numbers must be finite".to_string())?,
        ),
        Value::Array(values) => {
            Json::Array(values.iter().map(json_value).collect::<Result<_, _>>()?)
        }
        Value::Object(fields) => Json::Object(
            fields
                .iter()
                .map(|(k, v)| Ok((k.clone(), json_value(v)?)))
                .collect::<Result<_, String>>()?,
        ),
        _ => return Err(format!("{} cannot be encoded as JSON", value.type_name())),
    })
}
