use axum::http::StatusCode;
use base64::Engine;
use ruststack_compat_tests::RustStackTestClient;
use serde_json::json;

#[tokio::test]
async fn test_kms_http_protocol_compat() {
    let client = RustStackTestClient::new();

    // 1. CreateKey via HTTP TrentService.CreateKey
    let (status, val) = client
        .call_json(
            "TrentService.CreateKey",
            json!({
                "Description": "Production Encryption Key",
                "KeyUsage": "ENCRYPT_DECRYPT",
                "KeySpec": "SYMMETRIC_DEFAULT"
            }),
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    let key_id = val["KeyMetadata"]["KeyId"].as_str().unwrap().to_string();
    let key_arn = val["KeyMetadata"]["Arn"].as_str().unwrap().to_string();
    assert_eq!(
        val["KeyMetadata"]["Description"],
        "Production Encryption Key"
    );

    // 2. CreateAlias via HTTP TrentService.CreateAlias
    let (status, _) = client
        .call_json(
            "TrentService.CreateAlias",
            json!({
                "AliasName": "alias/prod-key",
                "TargetKeyId": key_id
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // 3. Encrypt via HTTP TrentService.Encrypt using Alias
    let plaintext = base64::engine::general_purpose::STANDARD.encode("SuperSecretPassword123!");
    let (status, enc_val) = client
        .call_json(
            "TrentService.Encrypt",
            json!({
                "KeyId": "alias/prod-key",
                "Plaintext": plaintext
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let ciphertext = enc_val["CiphertextBlob"].as_str().unwrap().to_string();
    assert_eq!(enc_val["KeyId"], key_arn);

    // 4. Decrypt via HTTP TrentService.Decrypt
    let (status, dec_val) = client
        .call_json(
            "TrentService.Decrypt",
            json!({
                "CiphertextBlob": ciphertext
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let recovered_b64 = dec_val["Plaintext"].as_str().unwrap();
    assert_eq!(recovered_b64, plaintext);

    let decoded_str = String::from_utf8(
        base64::engine::general_purpose::STANDARD
            .decode(recovered_b64)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(decoded_str, "SuperSecretPassword123!");

    // 5. GenerateDataKey via HTTP
    let (status, gdk_val) = client
        .call_json(
            "TrentService.GenerateDataKey",
            json!({
                "KeyId": "alias/prod-key",
                "KeySpec": "AES_256"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(gdk_val["Plaintext"].as_str().is_some());
    assert!(gdk_val["CiphertextBlob"].as_str().is_some());
}
