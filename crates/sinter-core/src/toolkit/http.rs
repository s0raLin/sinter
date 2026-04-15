use reqwest::{Client, Method, Response};
use std::collections::HashMap;

// Similar to sttp
pub struct Backend(Client);

impl Backend {
    pub fn new() -> Self {
        Backend(Client::new())
    }

    pub async fn close(self) {
        // reqwest client doesn't need explicit close
    }
}

pub struct Request {
    method: Method,
    url: String,
    headers: HashMap<String, String>,
    body: Option<String>,
}

impl Request {
    pub fn new(method: Method, url: String) -> Self {
        Request {
            method,
            url,
            headers: HashMap::new(),
            body: None,
        }
    }

    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn body(mut self, body: String) -> Self {
        self.body = Some(body);
        self
    }

    pub async fn send(self, backend: &Backend) -> Result<ResponseWrapper, reqwest::Error> {
        let mut builder = backend.0.request(self.method, &self.url);

        for (key, value) in self.headers {
            builder = builder.header(&key, &value);
        }

        if let Some(body) = self.body {
            builder = builder.body(body);
        }

        let response = builder.send().await?;
        Ok(ResponseWrapper(response))
    }
}

pub struct ResponseWrapper(Response);

impl ResponseWrapper {
    pub fn status(&self) -> u16 {
        self.0.status().as_u16()
    }

    pub async fn text(self) -> Result<String, reqwest::Error> {
        self.0.text().await
    }

    pub async fn json<T: serde::de::DeserializeOwned>(self) -> Result<T, reqwest::Error> {
        self.0.json().await
    }
}

pub fn basic_request() -> RequestBuilderWrapper {
    RequestBuilderWrapper {
        method: Method::GET,
        url: None,
        headers: HashMap::new(),
        body: None,
    }
}

pub struct RequestBuilderWrapper {
    method: Method,
    url: Option<String>,
    headers: HashMap<String, String>,
    body: Option<String>,
}

impl RequestBuilderWrapper {
    pub fn get(mut self, url: &str) -> Self {
        self.method = Method::GET;
        self.url = Some(url.to_string());
        self
    }

    pub fn post(mut self, url: &str) -> Self {
        self.method = Method::POST;
        self.url = Some(url.to_string());
        self
    }

    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn body(mut self, body: String) -> Self {
        self.body = Some(body);
        self
    }

    pub async fn send(self) -> Result<ResponseWrapper, reqwest::Error> {
        let client = Client::new();
        let mut builder = client.request(self.method, self.url.as_ref().unwrap());

        for (key, value) in self.headers {
            builder = builder.header(&key, &value);
        }

        if let Some(body) = self.body {
            builder = builder.body(body);
        }

        let response = builder.send().await?;
        Ok(ResponseWrapper(response))
    }
}
