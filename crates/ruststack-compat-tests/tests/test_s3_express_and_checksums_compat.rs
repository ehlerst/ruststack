use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use bytes::Bytes;
use ruststack_compat_tests::RustStackTestClient;
use ruststack_s3::checksum::{calculate_checksum, ChecksumAlgorithm};

#[tokio::test]
async fn test_s3_express_one_zone_directory_bucket_lifecycle() {
    let client = RustStackTestClient::new();
    let bucket_name = "test-wal-bucket--usw2-az1--x-s3";

    // 1. Create Directory Bucket via PUT /<bucket> with CreateBucketConfiguration
    let create_body = r#"<?xml version="1.0" encoding="UTF-8"?>
<CreateBucketConfiguration xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
    <LocationConstraint>us-west-2</LocationConstraint>
    <Location>
        <Type>AvailabilityZone</Type>
        <Name>usw2-az1</Name>
    </Location>
    <Bucket>
        <DataRedundancy>SingleAvailabilityZone</DataRedundancy>
        <Type>Directory</Type>
    </Bucket>
</CreateBucketConfiguration>"#;

    let (status, _, _) = client
        .call_s3(
            Method::PUT,
            &format!("/{}", bucket_name),
            HeaderMap::new(),
            Bytes::from(create_body),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // 2. HeadBucket should return Directory bucket header
    let (status, headers, _) = client
        .call_s3(
            Method::HEAD,
            &format!("/{}", bucket_name),
            HeaderMap::new(),
            Bytes::new(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers
            .get("x-amz-bucket-type")
            .and_then(|v| v.to_str().ok()),
        Some("Directory")
    );

    // 3. CreateSession API via POST /<bucket>?session
    let mut session_headers = HeaderMap::new();
    session_headers.insert(
        "x-amz-create-session-mode",
        HeaderValue::from_static("ReadWrite"),
    );

    let (status, headers, body) = client
        .call_s3(
            Method::POST,
            &format!("/{}?session", bucket_name),
            session_headers,
            Bytes::new(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("content-type").and_then(|v| v.to_str().ok()),
        Some("application/xml")
    );
    let xml = String::from_utf8_lossy(&body);
    assert!(xml.contains("<CreateSessionResult"));
    assert!(xml.contains("<AccessKeyId>ASIA_RUSTSTACK_EXPRESS_"));
    assert!(xml.contains("<SecretAccessKey>"));
    assert!(xml.contains("<SessionToken>"));
    assert!(xml.contains("<Expiration>"));

    // 4. CreateSession via GET /<bucket>?session also works
    let (status, _, body_get) = client
        .call_s3(
            Method::GET,
            &format!("/{}?session", bucket_name),
            HeaderMap::new(),
            Bytes::new(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(String::from_utf8_lossy(&body_get).contains("<CreateSessionResult"));

    // 5. PutObject in Directory Bucket -> StorageClass EXPRESS_ONEZONE
    let object_payload = Bytes::from_static(b"WAL record entry #0001: commit");
    let (status, headers, _) = client
        .call_s3(
            Method::PUT,
            &format!("/{}/wal/0001.log", bucket_name),
            HeaderMap::new(),
            object_payload.clone(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers
            .get("x-amz-storage-class")
            .and_then(|v| v.to_str().ok()),
        Some("EXPRESS_ONEZONE")
    );

    // 6. GetObject in Directory Bucket -> StorageClass EXPRESS_ONEZONE
    let (status, headers, body) = client
        .call_s3(
            Method::GET,
            &format!("/{}/wal/0001.log", bucket_name),
            HeaderMap::new(),
            Bytes::new(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, object_payload);
    assert_eq!(
        headers
            .get("x-amz-storage-class")
            .and_then(|v| v.to_str().ok()),
        Some("EXPRESS_ONEZONE")
    );

    // 7. ListObjectsV2 in Directory Bucket shows EXPRESS_ONEZONE
    let (status, _, body) = client
        .call_s3(
            Method::GET,
            &format!("/{}?list-type=2&prefix=wal/", bucket_name),
            HeaderMap::new(),
            Bytes::new(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let list_xml = String::from_utf8_lossy(&body);
    assert!(list_xml.contains("<Key>wal/0001.log</Key>"));
    assert!(list_xml.contains("<StorageClass>EXPRESS_ONEZONE</StorageClass>"));
}

#[tokio::test]
async fn test_s3_put_get_checksum_verification() {
    let client = RustStackTestClient::new();
    let bucket_name = "checksum-test-bucket";

    // Create bucket
    let (status, _, _) = client
        .call_s3(
            Method::PUT,
            &format!("/{}", bucket_name),
            HeaderMap::new(),
            Bytes::new(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let payload = Bytes::from_static(b"Hello world checksum test payload!");
    let expected_crc32 = calculate_checksum(ChecksumAlgorithm::Crc32, &payload);
    let expected_crc32c = calculate_checksum(ChecksumAlgorithm::Crc32c, &payload);
    let expected_sha256 = calculate_checksum(ChecksumAlgorithm::Sha256, &payload);

    // 1. PutObject with valid CRC32 and CRC32C headers
    let mut put_headers = HeaderMap::new();
    put_headers.insert(
        "x-amz-checksum-crc32",
        HeaderValue::from_str(&expected_crc32).unwrap(),
    );
    put_headers.insert(
        "x-amz-checksum-crc32c",
        HeaderValue::from_str(&expected_crc32c).unwrap(),
    );

    let (status, res_headers, _) = client
        .call_s3(
            Method::PUT,
            &format!("/{}/valid_checksum.dat", bucket_name),
            put_headers,
            payload.clone(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        res_headers
            .get("x-amz-checksum-crc32")
            .and_then(|v| v.to_str().ok()),
        Some(expected_crc32.as_str())
    );
    assert_eq!(
        res_headers
            .get("x-amz-checksum-crc32c")
            .and_then(|v| v.to_str().ok()),
        Some(expected_crc32c.as_str())
    );

    // 2. PutObject with INVALID checksum -> BadDigest 400
    let mut bad_headers = HeaderMap::new();
    bad_headers.insert(
        "x-amz-checksum-crc32",
        HeaderValue::from_static("AAAAAA=="), // Invalid checksum
    );

    let (status, _, err_body) = client
        .call_s3(
            Method::PUT,
            &format!("/{}/corrupt.dat", bucket_name),
            bad_headers,
            payload.clone(),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let err_xml = String::from_utf8_lossy(&err_body);
    assert!(err_xml.contains("<Code>BadDigest</Code>"));

    // 3. PutObject requesting algorithm calculation via x-amz-checksum-algorithm: SHA256
    let mut algo_headers = HeaderMap::new();
    algo_headers.insert(
        "x-amz-checksum-algorithm",
        HeaderValue::from_static("SHA256"),
    );

    let (status, res_headers, _) = client
        .call_s3(
            Method::PUT,
            &format!("/{}/algo_calc.dat", bucket_name),
            algo_headers,
            payload.clone(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        res_headers
            .get("x-amz-checksum-sha256")
            .and_then(|v| v.to_str().ok()),
        Some(expected_sha256.as_str())
    );

    // 4. GetObject returns stored checksums
    let (status, get_headers, body) = client
        .call_s3(
            Method::GET,
            &format!("/{}/valid_checksum.dat", bucket_name),
            HeaderMap::new(),
            Bytes::new(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, payload);
    assert_eq!(
        get_headers
            .get("x-amz-checksum-crc32")
            .and_then(|v| v.to_str().ok()),
        Some(expected_crc32.as_str())
    );
}

#[tokio::test]
async fn test_s3_complete_multipart_upload_checksums() {
    let client = RustStackTestClient::new();
    let bucket = "mp-checksum-bucket";

    // 1. Create Bucket
    let (status, _, _) = client
        .call_s3(
            Method::PUT,
            &format!("/{}", bucket),
            HeaderMap::new(),
            Bytes::new(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let key = "compacted-segment.log";

    // 2. Initiate Multipart Upload with x-amz-checksum-algorithm: CRC32
    let mut init_headers = HeaderMap::new();
    init_headers.insert(
        "x-amz-checksum-algorithm",
        HeaderValue::from_static("CRC32"),
    );

    let (status, _, init_body) = client
        .call_s3(
            Method::POST,
            &format!("/{}/{}?uploads", bucket, key),
            init_headers,
            Bytes::new(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let init_xml = String::from_utf8_lossy(&init_body);
    let upload_id = init_xml
        .split("<UploadId>")
        .nth(1)
        .and_then(|s| s.split("</UploadId>").next())
        .expect("UploadId found");

    // 3. Upload Part 1 with CRC32 checksum
    let part1_data = Bytes::from_static(b"Segment block 0001: 5MB compressed events");
    let part1_crc32 = calculate_checksum(ChecksumAlgorithm::Crc32, &part1_data);
    let mut p1_headers = HeaderMap::new();
    p1_headers.insert(
        "x-amz-checksum-crc32",
        HeaderValue::from_str(&part1_crc32).unwrap(),
    );

    let (status, p1_res_headers, _) = client
        .call_s3(
            Method::PUT,
            &format!("/{}/{}?uploadId={}&partNumber=1", bucket, key, upload_id),
            p1_headers,
            part1_data.clone(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let etag1 = p1_res_headers
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .unwrap()
        .to_string();

    // 4. Upload Part 2 with CRC32 checksum
    let part2_data = Bytes::from_static(b"Segment block 0002: 5MB compressed events");
    let part2_crc32 = calculate_checksum(ChecksumAlgorithm::Crc32, &part2_data);
    let mut p2_headers = HeaderMap::new();
    p2_headers.insert(
        "x-amz-checksum-crc32",
        HeaderValue::from_str(&part2_crc32).unwrap(),
    );

    let (status, p2_res_headers, _) = client
        .call_s3(
            Method::PUT,
            &format!("/{}/{}?uploadId={}&partNumber=2", bucket, key, upload_id),
            p2_headers,
            part2_data.clone(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let etag2 = p2_res_headers
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .unwrap()
        .to_string();

    // 5. CompleteMultipartUpload with ChecksumCRC32 elements in the Part request XML
    let complete_body = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<CompleteMultipartUpload xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
    <Part>
        <PartNumber>1</PartNumber>
        <ETag>{}</ETag>
        <ChecksumCRC32>{}</ChecksumCRC32>
    </Part>
    <Part>
        <PartNumber>2</PartNumber>
        <ETag>{}</ETag>
        <ChecksumCRC32>{}</ChecksumCRC32>
    </Part>
</CompleteMultipartUpload>"#,
        etag1, part1_crc32, etag2, part2_crc32
    );

    let (status, comp_headers, comp_body) = client
        .call_s3(
            Method::POST,
            &format!("/{}/{}?uploadId={}", bucket, key, upload_id),
            HeaderMap::new(),
            Bytes::from(complete_body),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let comp_xml = String::from_utf8_lossy(&comp_body);
    assert!(comp_xml.contains("<CompleteMultipartUploadResult"));
    assert!(comp_xml.contains("<ChecksumCRC32>"));
    assert!(comp_xml.contains("-2</ChecksumCRC32>")); // Composite multipart checksum ends with -[num_parts]

    // Verify response header also contains composite checksum
    let header_crc32 = comp_headers
        .get("x-amz-checksum-crc32")
        .and_then(|v| v.to_str().ok())
        .expect("x-amz-checksum-crc32 header present");
    assert!(header_crc32.ends_with("-2"));

    // 6. GetObject on the completed multipart object returns composite checksum
    let (status, get_headers, body) = client
        .call_s3(
            Method::GET,
            &format!("/{}/{}", bucket, key),
            HeaderMap::new(),
            Bytes::new(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body.as_ref(),
        [part1_data.as_ref(), part2_data.as_ref()]
            .concat()
            .as_slice()
    );
    assert_eq!(
        get_headers
            .get("x-amz-checksum-crc32")
            .and_then(|v| v.to_str().ok()),
        Some(header_crc32)
    );
}
