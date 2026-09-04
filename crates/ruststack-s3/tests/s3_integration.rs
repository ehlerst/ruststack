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
    assert_eq!(
        resp.headers().get("x-amz-meta-custom").unwrap(),
        "rust-rocks"
    );
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
    let req = Request::builder()
        .method(Method::PUT)
        .uri("/data-bucket")
        .body(Body::empty())
        .unwrap();
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
    let req = Request::builder()
        .method(Method::PUT)
        .uri("/mp-bucket")
        .body(Body::empty())
        .unwrap();
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
    let upload_id = xml
        .split("<UploadId>")
        .nth(1)
        .unwrap()
        .split("</UploadId>")
        .next()
        .unwrap();

    // 2. Upload Part 1
    let req = Request::builder()
        .method(Method::PUT)
        .uri(format!(
            "/mp-bucket/large.bin?uploadId={}&partNumber=1",
            upload_id
        ))
        .body(Body::from("PART_ONE_DATA_"))
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let etag1 = resp
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // 3. Upload Part 2
    let req = Request::builder()
        .method(Method::PUT)
        .uri(format!(
            "/mp-bucket/large.bin?uploadId={}&partNumber=2",
            upload_id
        ))
        .body(Body::from("PART_TWO_DATA"))
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let etag2 = resp
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

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

#[tokio::test]
async fn test_bucket_versioning_and_object_versions() {
    let storage = setup_s3();

    // 1. Create bucket
    let req = Request::builder()
        .method(Method::PUT)
        .uri("/ver-bucket")
        .body(Body::empty())
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 2. Put versioning configuration: Enabled
    let ver_xml = "<VersioningConfiguration><Status>Enabled</Status></VersioningConfiguration>";
    let req = Request::builder()
        .method(Method::PUT)
        .uri("/ver-bucket?versioning")
        .body(Body::from(ver_xml))
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. Get versioning configuration
    let req = Request::builder()
        .method(Method::GET)
        .uri("/ver-bucket?versioning")
        .body(Body::empty())
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8(body.to_vec()).unwrap();
    assert!(xml.contains("<Status>Enabled</Status>"));

    // 4. Put Object version 1
    let req = Request::builder()
        .method(Method::PUT)
        .uri("/ver-bucket/doc.txt")
        .body(Body::from("Version 1 content"))
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 5. Put Object version 2
    let req = Request::builder()
        .method(Method::PUT)
        .uri("/ver-bucket/doc.txt")
        .body(Body::from("Version 2 content updated"))
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 6. List object versions
    let req = Request::builder()
        .method(Method::GET)
        .uri("/ver-bucket?versions")
        .body(Body::empty())
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8(body.to_vec()).unwrap();
    assert!(xml.contains("<Key>doc.txt</Key>"));
    assert!(xml.contains("<IsLatest>true</IsLatest>"));
    assert!(xml.contains("<IsLatest>false</IsLatest>"));

    // Extract first versionId from xml
    let vid1 = xml.split("<VersionId>").nth(1).unwrap().split("</VersionId>").next().unwrap();
    let vid2 = xml.split("<VersionId>").nth(2).unwrap().split("</VersionId>").next().unwrap();

    // 7. Get specific version
    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/ver-bucket/doc.txt?versionId={}", vid2))
        .body(Body::empty())
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers().get("x-amz-version-id").unwrap(), vid2);

    // 8. Delete specific version
    let req = Request::builder()
        .method(Method::DELETE)
        .uri(format!("/ver-bucket/doc.txt?versionId={}", vid1))
        .body(Body::empty())
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_bucket_lifecycle_cors_policy_tagging() {
    let storage = setup_s3();

    // 1. Create bucket
    let req = Request::builder()
        .method(Method::PUT)
        .uri("/config-bucket")
        .body(Body::empty())
        .unwrap();
    let _ = handle_s3_request(storage.clone(), req).await;

    // 2. Lifecycle
    let lc_xml = r#"<LifecycleConfiguration>
        <Rule>
            <ID>expire-logs</ID>
            <Status>Enabled</Status>
            <Prefix>logs/</Prefix>
            <Days>30</Days>
        </Rule>
    </LifecycleConfiguration>"#;
    let req = Request::builder()
        .method(Method::PUT)
        .uri("/config-bucket?lifecycle")
        .body(Body::from(lc_xml))
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let req = Request::builder()
        .method(Method::GET)
        .uri("/config-bucket?lifecycle")
        .body(Body::empty())
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert!(String::from_utf8(body.to_vec()).unwrap().contains("expire-logs"));

    // 3. CORS
    let cors_xml = r#"<CORSConfiguration>
        <CORSRule>
            <AllowedOrigin>*</AllowedOrigin>
            <AllowedMethod>GET</AllowedMethod>
            <AllowedMethod>PUT</AllowedMethod>
            <AllowedHeader>*</AllowedHeader>
            <MaxAgeSeconds>3600</MaxAgeSeconds>
        </CORSRule>
    </CORSConfiguration>"#;
    let req = Request::builder()
        .method(Method::PUT)
        .uri("/config-bucket?cors")
        .body(Body::from(cors_xml))
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let req = Request::builder()
        .method(Method::GET)
        .uri("/config-bucket?cors")
        .body(Body::empty())
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 4. Policy
    let policy_json = r#"{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Principal":"*","Action":"s3:GetObject","Resource":"arn:aws:s3:::config-bucket/*"}]}"#;
    let req = Request::builder()
        .method(Method::PUT)
        .uri("/config-bucket?policy")
        .body(Body::from(policy_json))
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let req = Request::builder()
        .method(Method::GET)
        .uri("/config-bucket?policy")
        .body(Body::empty())
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert!(String::from_utf8(body.to_vec()).unwrap().contains("s3:GetObject"));

    // 5. Tagging
    let tag_xml = r#"<Tagging>
        <TagSet>
            <Tag><Key>Environment</Key><Value>Development</Value></Tag>
            <Tag><Key>Project</Key><Value>RustStack</Value></Tag>
        </TagSet>
    </Tagging>"#;
    let req = Request::builder()
        .method(Method::PUT)
        .uri("/config-bucket?tagging")
        .body(Body::from(tag_xml))
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let req = Request::builder()
        .method(Method::GET)
        .uri("/config-bucket?tagging")
        .body(Body::empty())
        .unwrap();
    let resp = handle_s3_request(storage.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let xml = String::from_utf8(body.to_vec()).unwrap();
    assert!(xml.contains("<Key>Environment</Key><Value>Development</Value>"));
}
