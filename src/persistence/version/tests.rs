use super::*;

#[test]
fn test_peek_version_from_common_shapes() {
    assert_eq!(
        peek_version_from_str(r#"{"version":"2.0.0","data":{}}"#).unwrap(),
        Some("2.0.0".to_string())
    );
    assert_eq!(
        peek_version_from_str(r#"{"save_version":6,"state":{}}"#).unwrap(),
        Some("6".to_string())
    );
    assert_eq!(
        peek_version_from_str(r#"{"slot":{"version":"1.1"},"data":{}}"#).unwrap(),
        Some("1.1".to_string())
    );
}
