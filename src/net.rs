//! Cross-platform, frame-polled JSON HTTP for Macroquad clients.
//!
//! Enable the toolkit's `net` feature to use this module. Requests use
//! `quad-net`, which runs on a background thread on native and through the
//! publisher's `quad-net.js` bridge on WASM. A request never blocks the game
//! loop: retain the returned [`Pending`] value and poll it once per frame.
//!
//! The toolkit owns transport, request headers, JSON encoding/decoding, and
//! the timeout safety net. A game still owns its protocol types, endpoint
//! paths, authentication policy, session state, retry cadence, and server.
//!
//! ```rust,ignore
//! use macroquad_toolkit::net::{HttpClient, Pending};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Deserialize)]
//! struct WorldView { tick: u64 }
//! #[derive(Serialize)]
//! struct MoveCommand { x: i32, y: i32 }
//!
//! let mut api = HttpClient::new("https://example.test/api");
//! api.set_bearer_token(Some("account-token"));
//! let mut view: Pending<WorldView> = api.get("/view");
//! let mut action: Pending<WorldView> = api.post_json("/move", &MoveCommand { x: 1, y: 2 });
//! // In the Macroquad update loop:
//! if let Some(result) = view.poll_timed(dt, 6.0) { /* adopt or report result */ }
//! if let Some(result) = action.poll_timed(dt, 6.0) { /* adopt or report result */ }
//! ```

use crate::data_loader::parse_json_labeled;
use quad_net::http_request::{Method, Request, RequestBuilder};
use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;

/// HTTP verbs supported by the `quad-net` transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

impl HttpMethod {
    fn as_quad_net(self) -> Method {
        match self {
            Self::Get => Method::Get,
            Self::Post => Method::Post,
            Self::Put => Method::Put,
            Self::Delete => Method::Delete,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
        }
    }
}

/// A JSON response in flight.
///
/// Poll with [`Pending::poll`] or [`Pending::poll_timed`] from the game loop.
/// `None` means the transport has not delivered a response yet. A delivered
/// transport failure, JSON error, or timeout is returned as `Some(Err(_))`.
pub struct Pending<T> {
    request: Option<Request>,
    ready: Option<Result<T, String>>,
    label: String,
    elapsed: f32,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Pending<T> {
    fn new(request: Request, label: String) -> Self {
        Self {
            request: Some(request),
            ready: None,
            label,
            elapsed: 0.0,
            _marker: PhantomData,
        }
    }

    /// Create an already-failed pending response.
    ///
    /// This keeps a request-building failure on the same poll-based path as a
    /// transport failure, so callers do not need a second error channel for
    /// serialization errors.
    pub fn failed(error: impl Into<String>) -> Self {
        Self {
            request: None,
            ready: Some(Err(error.into())),
            label: "request".to_owned(),
            elapsed: 0.0,
            _marker: PhantomData,
        }
    }
}

impl<T: DeserializeOwned> Pending<T> {
    /// Poll for a response without advancing a timeout.
    pub fn poll(&mut self) -> Option<Result<T, String>> {
        if let Some(result) = self.ready.take() {
            return Some(result);
        }

        self.request.as_mut()?.try_recv().map(|result| {
            result
                .map_err(|error| format!("HTTP request '{}' failed: {error}", self.label))
                .and_then(|body| decode_json(&self.label, &body))
        })
    }

    /// Poll and fail if no response arrives within `timeout` seconds.
    ///
    /// `dt` is the frame delta. This timeout matters particularly on WASM:
    /// `quad-net`'s browser bridge only reports successful responses, so a
    /// refused connection can otherwise remain pending forever. Dropping a
    /// timed-out request does not cancel the browser fetch; callers should use
    /// their own retry cooldown to avoid issuing replacements every frame.
    pub fn poll_timed(&mut self, dt: f32, timeout: f32) -> Option<Result<T, String>> {
        if let Some(result) = self.poll() {
            return Some(result);
        }

        self.elapsed += dt.max(0.0);
        let timeout = timeout.max(0.0);
        if self.elapsed >= timeout {
            Some(Err(format!(
                "HTTP request '{}' timed out after {:.1} seconds",
                self.label, timeout
            )))
        } else {
            None
        }
    }
}

fn decode_json<T: DeserializeOwned>(label: &str, body: &str) -> Result<T, String> {
    parse_json_labeled(label, body)
}

/// A configured HTTP client for one game server or gateway.
///
/// Headers are copied onto every request. Games can use [`set_header`] for
/// protocol-specific session identifiers and [`set_bearer_token`] for a
/// conventional account token. Endpoint paths remain game-owned.
pub struct HttpClient {
    base_url: String,
    headers: Vec<(String, String)>,
}

impl HttpClient {
    /// Create a client addressed at `base_url`.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            headers: Vec::new(),
        }
    }

    /// The configured base URL, without trailing slashes.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Add or replace a header for all future requests.
    ///
    /// Header names are compared case-insensitively so replacing
    /// `Authorization` also replaces a previously stored `authorization`.
    pub fn set_header(&mut self, name: impl Into<String>, value: impl Into<String>) {
        let name = name.into();
        self.headers
            .retain(|(existing, _)| !existing.eq_ignore_ascii_case(&name));
        self.headers.push((name, value.into()));
    }

    /// Remove a header from all future requests.
    pub fn remove_header(&mut self, name: &str) {
        self.headers
            .retain(|(existing, _)| !existing.eq_ignore_ascii_case(name));
    }

    /// Configure or clear a conventional `Authorization: Bearer ...` header.
    pub fn set_bearer_token(&mut self, token: Option<&str>) {
        match token.map(str::trim).filter(|token| !token.is_empty()) {
            Some(token) => self.set_header("Authorization", format!("Bearer {token}")),
            None => self.remove_header("Authorization"),
        }
    }

    /// Return a client with one additional shared header.
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.set_header(name, value);
        self
    }

    /// Issue a JSON request with no request body.
    pub fn request<T: DeserializeOwned>(&self, method: HttpMethod, path: &str) -> Pending<T> {
        let label = format!("{} {}", method.label(), path);
        Pending::new(self.builder(method, path).send(), label)
    }

    /// Issue a JSON request with a serialized request body.
    pub fn request_json<T: DeserializeOwned, B: Serialize>(
        &self,
        method: HttpMethod,
        path: &str,
        body: &B,
    ) -> Pending<T> {
        let label = format!("{} {}", method.label(), path);
        let body = match serde_json::to_string(body) {
            Ok(body) => body,
            Err(error) => {
                return Pending::failed(format!(
                    "HTTP request '{label}' could not encode JSON: {error}"
                ));
            }
        };

        let request = self
            .builder(method, path)
            .header("Content-Type", "application/json")
            .body(&body)
            .send();
        Pending::new(request, label)
    }

    /// Issue a `GET` request.
    pub fn get<T: DeserializeOwned>(&self, path: &str) -> Pending<T> {
        self.request(HttpMethod::Get, path)
    }

    /// Issue a bodyless `POST` request.
    pub fn post<T: DeserializeOwned>(&self, path: &str) -> Pending<T> {
        self.request(HttpMethod::Post, path)
    }

    /// Issue a JSON `POST` request.
    pub fn post_json<T: DeserializeOwned, B: Serialize>(&self, path: &str, body: &B) -> Pending<T> {
        self.request_json(HttpMethod::Post, path, body)
    }

    /// Join an endpoint path to the configured base URL.
    fn url(&self, path: &str) -> String {
        let path = path.trim();
        if path.is_empty() {
            self.base_url.clone()
        } else if path.starts_with('/') {
            format!("{}{}", self.base_url, path)
        } else if self.base_url.is_empty() {
            path.to_owned()
        } else {
            format!("{}/{}", self.base_url, path)
        }
    }

    fn builder(&self, method: HttpMethod, path: &str) -> RequestBuilder {
        let mut builder = RequestBuilder::new(&self.url(path)).method(method.as_quad_net());
        for (name, value) in &self.headers {
            builder = builder.header(name, value);
        }
        builder
    }
}

#[cfg(test)]
mod tests;
