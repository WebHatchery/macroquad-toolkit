use super::*;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
struct TestResponse {
    value: u32,
}

#[test]
fn client_normalizes_base_url_and_joins_paths() {
    let client = HttpClient::new("https://example.test/api///");

    assert_eq!(client.base_url(), "https://example.test/api");
    assert_eq!(client.url("/view"), "https://example.test/api/view");
    assert_eq!(
        client.url("events?since=4"),
        "https://example.test/api/events?since=4"
    );
    assert_eq!(client.url(""), "https://example.test/api");
}

#[test]
fn headers_replace_without_case_duplicates() {
    let mut client = HttpClient::new("https://example.test");
    client.set_header("authorization", "old");
    client.set_header("Authorization", "new");
    client.set_bearer_token(Some(" account-token "));

    assert_eq!(client.headers.len(), 1);
    assert_eq!(
        client.headers[0],
        (
            "Authorization".to_owned(),
            "Bearer account-token".to_owned()
        )
    );

    client.set_bearer_token(None);
    assert!(client.headers.is_empty());
}

#[test]
fn failed_pending_is_reported_once() {
    let mut pending = Pending::<TestResponse>::failed("bad request");

    assert_eq!(pending.poll(), Some(Err("bad request".to_owned())));
    assert!(pending.poll().is_none());
}

#[test]
fn malformed_response_names_the_http_endpoint() {
    let error = decode_json::<TestResponse>("GET /view", "{not-json").unwrap_err();

    assert!(error.contains("GET /view"));
    assert!(error.contains("line 1"));
}

#[test]
fn method_labels_are_stable_for_diagnostics() {
    assert_eq!(HttpMethod::Get.label(), "GET");
    assert_eq!(HttpMethod::Post.label(), "POST");
    assert_eq!(HttpMethod::Put.label(), "PUT");
    assert_eq!(HttpMethod::Delete.label(), "DELETE");
}
