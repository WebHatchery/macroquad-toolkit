//! Native frame-polled transport without quad-net's obsolete websocket stack.

use std::sync::mpsc::{self, Receiver, TryRecvError};

pub(super) type Method = super::HttpMethod;

#[derive(Debug)]
pub(super) enum HttpError {
    UreqError(Box<ureq::Error>),
    Io(std::io::Error),
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UreqError(error) => error.fmt(formatter),
            Self::Io(error) => error.fmt(formatter),
        }
    }
}

pub(super) struct Request {
    receiver: Receiver<Result<String, HttpError>>,
}

impl Request {
    pub(super) fn try_recv(&mut self) -> Option<Result<String, HttpError>> {
        match self.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err(HttpError::Io(std::io::Error::other(
                "HTTP worker disconnected",
            )))),
        }
    }
}

pub(super) struct RequestBuilder {
    request: ureq::Request,
    body: Option<String>,
}

impl RequestBuilder {
    pub(super) fn new(url: &str) -> Self {
        Self {
            request: ureq::get(url),
            body: None,
        }
    }

    pub(super) fn method(mut self, method: Method) -> Self {
        self.request = ureq::request(method.label(), self.request.url());
        self
    }

    pub(super) fn header(mut self, name: &str, value: &str) -> Self {
        self.request = self.request.set(name, value);
        self
    }

    pub(super) fn body(mut self, body: &str) -> Self {
        self.body = Some(body.to_owned());
        self
    }

    pub(super) fn send(self) -> Request {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let response = match self.body {
                Some(body) => self.request.send_string(&body),
                None => self.request.call(),
            }
            .map_err(|error| HttpError::UreqError(Box::new(error)))
            .and_then(|response| response.into_string().map_err(HttpError::Io));
            // Dropping a Pending cancels observation; completing the worker is safe.
            let _ = sender.send(response);
        });
        Request { receiver }
    }
}

#[cfg(test)]
mod tests;
