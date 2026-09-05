use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

#[test]
fn frame_polled_http_preserves_methods_headers_bodies_and_json() {
    for method in [Method::Get, Method::Post, Method::Put, Method::Delete] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            let mut byte = [0];
            while !request.ends_with(b"\r\n\r\n") {
                stream.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
            }
            let headers = String::from_utf8(request).unwrap();
            let size: usize = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse().unwrap())
                })
                .unwrap_or(0);
            let mut body = vec![0; size];
            stream.read_exact(&mut body).unwrap();
            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{\"value\":7}").unwrap();
            (headers, body)
        });
        let mut client = crate::net::HttpClient::new(format!("http://{address}"));
        client.set_bearer_token(Some("test-token"));
        let payload = serde_json::json!({"step": 2});
        let mut pending = client.request_json::<serde_json::Value, _>(method, "/test", &payload);
        let deadline = Instant::now() + Duration::from_secs(5);
        let response = loop {
            if let Some(response) = pending.poll() {
                break response.unwrap();
            }
            assert!(Instant::now() < deadline, "native request timed out");
            std::thread::sleep(Duration::from_millis(5));
        };
        assert_eq!(response["value"], 7);
        assert!(pending.poll().is_none());
        assert!(pending.poll_timed(10.0, 1.0).is_none());
        let (headers, body) = server.join().unwrap();
        assert!(headers.starts_with(&format!("{} /test HTTP/1.1", method.label())));
        assert!(headers
            .to_ascii_lowercase()
            .contains("authorization: bearer test-token"));
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
            payload
        );
    }
}

#[test]
fn disconnected_workers_report_an_error_instead_of_pending_forever() {
    let (sender, receiver) = mpsc::channel();
    drop(sender);
    let error = Request { receiver }.try_recv().unwrap().unwrap_err();
    assert!(error.to_string().contains("disconnected"));
}
