use base64::Engine;
use ruststack_kms::{
    CreateAliasRequest, CreateKeyRequest, DecryptRequest, DescribeKeyRequest, EncryptRequest,
    GenerateDataKeyRequest, KmsState, ListAliasesRequest, ListKeysRequest, ScheduleKeyDeletionRequest,
};

#[test]
fn test_kms_key_lifecycle() {
    let state = KmsState::new("000000000000".to_string(), "us-east-1".to_string());

    // 1. Create custom key
    let meta = state
        .create_key(CreateKeyRequest {
            description: Some("My App Master Key".to_string()),
            key_usage: "ENCRYPT_DECRYPT".to_string(),
            key_spec: "SYMMETRIC_DEFAULT".to_string(),
            customer_master_key_spec: "SYMMETRIC_DEFAULT".to_string(),
            tags: None,
        })
        .expect("CreateKey failed");

    assert_eq!(meta.description, "My App Master Key");
    assert!(meta.enabled);
    assert_eq!(meta.key_state, "Enabled");

    // 2. Describe Key by Key ID
    let described = state
        .describe_key(DescribeKeyRequest {
            key_id: meta.key_id.clone(),
        })
        .expect("DescribeKey failed");
    assert_eq!(described.key_id, meta.key_id);

    // 3. Create Alias
    state
        .create_alias("alias/my-app/db-key".to_string(), meta.key_id.clone())
        .expect("CreateAlias failed");

    // 4. Describe Key by Alias Name
    let by_alias = state
        .describe_key(DescribeKeyRequest {
            key_id: "alias/my-app/db-key".to_string(),
        })
        .expect("DescribeKey by alias failed");
    assert_eq!(by_alias.key_id, meta.key_id);

    // 5. List Keys
    let (keys, _) = state.list_keys(ListKeysRequest { limit: None, marker: None }).unwrap();
    assert!(keys.len() >= 3); // 2 default AWS keys + 1 custom key

    // 6. List Aliases
    let aliases = state.list_aliases(ListAliasesRequest { key_id: None, limit: None, marker: None }).unwrap();
    assert!(aliases.iter().any(|a| a.alias_name == "alias/my-app/db-key"));
}

#[test]
fn test_kms_encrypt_decrypt_and_data_key() {
    let state = KmsState::new("000000000000".to_string(), "us-east-1".to_string());

    let meta = state
        .create_key(CreateKeyRequest {
            description: Some("Crypto Test Key".to_string()),
            key_usage: "ENCRYPT_DECRYPT".to_string(),
            key_spec: "SYMMETRIC_DEFAULT".to_string(),
            customer_master_key_spec: "SYMMETRIC_DEFAULT".to_string(),
            tags: None,
        })
        .unwrap();

    let secret_plaintext = "Hello, RustStack ultra-fast encryption!";
    let plain_b64 = base64::engine::general_purpose::STANDARD.encode(secret_plaintext);

    // 1. Encrypt using Alias
    state
        .create_alias("alias/crypto-key".to_string(), meta.key_id.clone())
        .unwrap();

    let (ciphertext, key_arn) = state
        .encrypt(EncryptRequest {
            key_id: "alias/crypto-key".to_string(),
            plaintext: plain_b64.clone(),
            encryption_algorithm: "SYMMETRIC_DEFAULT".to_string(),
            encryption_context: None,
        })
        .expect("Encrypt failed");

    assert_ne!(ciphertext, plain_b64);
    assert_eq!(key_arn, meta.arn);

    // 2. Decrypt without specifying KeyId (envelope contains embedded key ID)
    let (decrypted_b64, dec_arn) = state
        .decrypt(DecryptRequest {
            ciphertext_blob: ciphertext.clone(),
            key_id: None,
            encryption_algorithm: "SYMMETRIC_DEFAULT".to_string(),
            encryption_context: None,
        })
        .expect("Decrypt failed");

    assert_eq!(decrypted_b64, plain_b64);
    assert_eq!(dec_arn, meta.arn);

    let decoded_bytes = base64::engine::general_purpose::STANDARD.decode(decrypted_b64).unwrap();
    let recovered_string = String::from_utf8(decoded_bytes).unwrap();
    assert_eq!(recovered_string, secret_plaintext);

    // 3. GenerateDataKey
    let (data_plain, data_cipher, _) = state
        .generate_data_key(GenerateDataKeyRequest {
            key_id: meta.key_id.clone(),
            key_spec: "AES_256".to_string(),
            number_of_bytes: Some(32),
            encryption_context: None,
        })
        .expect("GenerateDataKey failed");

    assert_ne!(data_plain, data_cipher);
    let (decrypted_data_key, _) = state
        .decrypt(DecryptRequest {
            ciphertext_blob: data_cipher,
            key_id: None,
            encryption_algorithm: "SYMMETRIC_DEFAULT".to_string(),
            encryption_context: None,
        })
        .unwrap();
    assert_eq!(decrypted_data_key, data_plain);

    // 4. Disable Key -> Encrypt/Decrypt should fail with Disabled
    state.disable_key(&meta.key_id).unwrap();
    let enc_fail = state.encrypt(EncryptRequest {
        key_id: meta.key_id.clone(),
        plaintext: plain_b64.clone(),
        encryption_algorithm: "SYMMETRIC_DEFAULT".to_string(),
        encryption_context: None,
    });
    assert!(enc_fail.is_err());

    // 5. Enable Key -> Encrypt should succeed again
    state.enable_key(&meta.key_id).unwrap();
    let enc_ok = state.encrypt(EncryptRequest {
        key_id: meta.key_id.clone(),
        plaintext: plain_b64,
        encryption_algorithm: "SYMMETRIC_DEFAULT".to_string(),
        encryption_context: None,
    });
    assert!(enc_ok.is_ok());

    // 6. Schedule Key Deletion
    let (del_arn, del_date) = state
        .schedule_key_deletion(ScheduleKeyDeletionRequest {
            key_id: meta.key_id.clone(),
            pending_window_in_days: 7,
        })
        .unwrap();
    assert_eq!(del_arn, meta.arn);
    assert!(del_date > 0.0);
}
