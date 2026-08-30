use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use bytes::Bytes;
use ruststack_compat_tests::RustStackTestClient;

#[tokio::test]
async fn test_s3_bucket_and_object_crud() {
    let client = RustStackTestClient::new();

    // 1. CreateBucket
    let (status, _, _) = client
        .call_s3(
            Method::PUT,
            "/test-compat-bucket",
            HeaderMap::new(),
            Bytes::new(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // 2. HeadBucket
    let (status, _, _) = client
        .call_s3(
            Method::HEAD,
            "/test-compat-bucket",
            HeaderMap::new(),
            Bytes::new(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // 3. PutObject
    let (status, _, _) = client
        .call_s3(
            Method::PUT,
            "/test-compat-bucket/greeting.txt",
            HeaderMap::new(),
            Bytes::from_static(b"Hello from RustStack S3 Compat!"),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // 4. GetObject full
    let (status, headers, body) = client
        .call_s3(
            Method::GET,
            "/test-compat-bucket/greeting.txt",
            HeaderMap::new(),
            Bytes::new(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_ref(), b"Hello from RustStack S3 Compat!");
    assert!(headers.get("etag").is_some());

    // 5. GetObject with byte-range Range: bytes=0-4 ("Hello")
    let mut range_headers = HeaderMap::new();
    range_headers.insert("range", HeaderValue::from_static("bytes=0-4"));
    let (status, _, body) = client
        .call_s3(
            Method::GET,
            "/test-compat-bucket/greeting.txt",
            range_headers,
            Bytes::new(),
        )
        .await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(body.as_ref(), b"Hello");

    // 6. HeadObject
    let (status, headers, _) = client
        .call_s3(
            Method::HEAD,
            "/test-compat-bucket/greeting.txt",
            HeaderMap::new(),
            Bytes::new(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("content-length").unwrap().to_str().unwrap(),
        "31"
    );

    // 7. DeleteObject
    let (status, _, _) = client
        .call_s3(
            Method::DELETE,
            "/test-compat-bucket/greeting.txt",
            HeaderMap::new(),
            Bytes::new(),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // 8. DeleteBucket
    let (status, _, _) = client
        .call_s3(
            Method::DELETE,
            "/test-compat-bucket",
            HeaderMap::new(),
            Bytes::new(),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_s3_virtual_hosting_and_list_objects() {
    let client = RustStackTestClient::new();

    // Create bucket via virtual host
    let (status, _, _) = client
        .call_s3_virtual_host(
            "vh-bucket",
            Method::PUT,
            "/",
            HeaderMap::new(),
            Bytes::new(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Put several objects with prefixes
    let files = vec![
        ("logs/2026/01.log", "log data 1"),
        ("logs/2026/02.log", "log data 2"),
        ("data/file.csv", "csv data"),
    ];

    for (k, content) in files {
        let (status, _, _) = client
            .call_s3_virtual_host(
                "vh-bucket",
                Method::PUT,
                &format!("/{}", k),
                HeaderMap::new(),
                Bytes::from(content.to_string()),
            )
            .await;
        assert_eq!(status, StatusCode::OK);
    }

    // ListObjectsV2 with prefix logs/
    let (status, _, body) = client
        .call_s3_virtual_host(
            "vh-bucket",
            Method::GET,
            "/?list-type=2&prefix=logs/",
            HeaderMap::new(),
            Bytes::new(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let xml = String::from_utf8_lossy(&body);
    assert!(xml.contains("<Key>logs/2026/01.log</Key>"));
    assert!(xml.contains("<Key>logs/2026/02.log</Key>"));
    assert!(!xml.contains("<Key>data/file.csv</Key>"));
}
