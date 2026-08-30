use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use ruststack_s3::{handle_s3_request, InMemoryStorage, S3Storage};
use std::sync::Arc;

fn setup_s3() -> Arc<dyn S3Storage> {
    Arc::new(InMemoryStorage::new())
}

#[tokio::test]
async fn test_bucket_lifecycle() {
    let storage = setup_s3();

    // 1. Create Bucket
    let req = Request::builder()
        .method(Method::PUT)
        .uri("/test-bucket")
        .body(Body::empty())
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 2. Head Bucket
    let req = Request::builder()
        .method(Method::HEAD)
        .uri("/test-bucket")
        .body(Body::empty())
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. List Buckets
    let req = Request::builder()
        .method(Method::GET)
        .uri("/")
        .body(Body::empty())
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8(body.to_vec()).unwrap();
    assert!(xml.contains("<Name>test-bucket</Name>"));

    // 4. Delete Bucket
    let req = Request::builder()
        .method(Method::DELETE)
        .uri("/test-bucket")
        .body(Body::empty())
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // 5. Head Bucket should 404
    let req = Request::builder()
        .method(Method::HEAD)
        .uri("/test-bucket")
        .body(Body::empty())
        .unwrap();
    let resp_err = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp_err.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_object_crud_and_range() {
    let storage = setup_s3();

    // Create bucket
    let req = Request::builder()
        .method(Method::PUT)
        .uri("/my-bucket")
        .body(Body::empty())
        .unwrap();
    let _ = handle_s3_request(storage.clone(), req).await;

    // Put Object
    let req = Request::builder()
        .method(Method::PUT)
        .uri("/my-bucket/hello.txt")
        .header("content-type", "text/plain")
        .header("x-amz-meta-custom", "rust-rocks")
        .body(Body::from("Hello, RustStack!"))
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(resp.headers().contains_key("etag"));

    // Head Object
    let req = Request::builder()
        .method(Method::HEAD)
        .uri("/my-bucket/hello.txt")
        .body(Body::empty())
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers().get("x-amz-meta-custom").unwrap(), "rust-rocks");
    assert_eq!(resp.headers().get("content-length").unwrap(), "17");

    // Get Object Full
    let req = Request::builder()
        .method(Method::GET)
        .uri("/my-bucket/hello.txt")
        .body(Body::empty())
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(body.as_ref(), b"Hello, RustStack!");

    // Get Object Range (0-4 -> "Hello")
    let req = Request::builder()
        .method(Method::GET)
        .uri("/my-bucket/hello.txt")
        .header("range", "bytes=0-4")
        .body(Body::empty())
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(resp.headers().get("content-range").unwrap(), "bytes 0-4/17");
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(body.as_ref(), b"Hello");

    // Delete Object
    let req = Request::builder()
        .method(Method::DELETE)
        .uri("/my-bucket/hello.txt")
        .body(Body::empty())
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_list_objects_v2() {
    let storage = setup_s3();

    // Create bucket
    let req = Request::builder().method(Method::PUT).uri("/data-bucket").body(Body::empty()).unwrap();
    let _ = handle_s3_request(storage.clone(), req).await;

    // Insert 5 objects
    for i in 1..=5 {
        let req = Request::builder()
            .method(Method::PUT)
            .uri(format!("/data-bucket/file_{}.txt", i))
            .body(Body::from(format!("content {}", i)))
            .unwrap();
        let _ = handle_s3_request(storage.clone(), req).await;
    }

    // List objects
    let req = Request::builder()
        .method(Method::GET)
        .uri("/data-bucket?list-type=2&max-keys=3")
        .body(Body::empty())
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8(body.to_vec()).unwrap();
    assert!(xml.contains("<Key>file_1.txt</Key>"));
    assert!(xml.contains("<Key>file_2.txt</Key>"));
    assert!(xml.contains("<Key>file_3.txt</Key>"));
    assert!(xml.contains("<IsTruncated>true</IsTruncated>"));
}

#[tokio::test]
async fn test_multipart_upload_lifecycle() {
    let storage = setup_s3();

    // Create bucket
    let req = Request::builder().method(Method::PUT).uri("/mp-bucket").body(Body::empty()).unwrap();
    let _ = handle_s3_request(storage.clone(), req).await;

    // 1. Initiate Multipart
    let req = Request::builder()
        .method(Method::POST)
        .uri("/mp-bucket/large.bin?uploads")
        .body(Body::empty())
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8(body.to_vec()).unwrap();
    let upload_id = xml.split("<UploadId>").nth(1).unwrap().split("</UploadId>").next().unwrap();

    // 2. Upload Part 1
    let req = Request::builder()
        .method(Method::PUT)
        .uri(format!("/mp-bucket/large.bin?uploadId={}&partNumber=1", upload_id))
        .body(Body::from("PART_ONE_DATA_"))
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let etag1 = resp.headers().get("etag").unwrap().to_str().unwrap().to_string();

    // 3. Upload Part 2
    let req = Request::builder()
        .method(Method::PUT)
        .uri(format!("/mp-bucket/large.bin?uploadId={}&partNumber=2", upload_id))
        .body(Body::from("PART_TWO_DATA"))
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let etag2 = resp.headers().get("etag").unwrap().to_str().unwrap().to_string();

    // 4. List Parts
    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/mp-bucket/large.bin?uploadId={}", upload_id))
        .body(Body::empty())
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8(body.to_vec()).unwrap();
    assert!(xml.contains("<PartNumber>1</PartNumber>"));
    assert!(xml.contains("<PartNumber>2</PartNumber>"));

    // 5. Complete Multipart
    let complete_xml = format!(
        r#"<CompleteMultipartUpload>
            <Part>
                <PartNumber>1</PartNumber>
                <ETag>{}</ETag>
            </Part>
            <Part>
                <PartNumber>2</PartNumber>
                <ETag>{}</ETag>
            </Part>
        </CompleteMultipartUpload>"#,
        etag1, etag2
    );
    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("/mp-bucket/large.bin?uploadId={}", upload_id))
        .body(Body::from(complete_xml))
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 6. Verify concatenated object
    let req = Request::builder()
        .method(Method::GET)
        .uri("/mp-bucket/large.bin")
        .body(Body::empty())
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(body.as_ref(), b"PART_ONE_DATA_PART_TWO_DATA");
}
