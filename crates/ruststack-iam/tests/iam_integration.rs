use ruststack_iam::state::IamState;

#[tokio::test]
async fn test_iam_lifecycle() {
    let state = IamState::new("000000000000".to_string());

    // 1. Create Role
    let role = state
        .create_role(
            "LambdaExecRole".to_string(),
            r#"{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Principal":{"Service":"lambda.amazonaws.com"},"Action":"sts:AssumeRole"}]}"#.to_string(),
            None,
            Some("Role for Lambda execution".to_string()),
        )
        .expect("create role");
    assert_eq!(role.role_name, "LambdaExecRole");
    assert!(role.arn.contains(":role/LambdaExecRole"));

    // 2. Create Policy
    let policy = state
        .create_policy(
            "CustomS3WritePolicy".to_string(),
            r#"{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Action":["s3:PutObject"],"Resource":"*"}]}"#.to_string(),
            None,
            Some("Write access to S3".to_string()),
        )
        .expect("create policy");
    assert_eq!(policy.policy_name, "CustomS3WritePolicy");
    assert_eq!(policy.attachment_count, 0);

    // 3. Attach Policy to Role
    state
        .attach_role_policy("LambdaExecRole", &policy.arn)
        .expect("attach policy");

    let pol_check = state.get_policy(&policy.arn).expect("get policy");
    assert_eq!(pol_check.attachment_count, 1);

    let attached = state
        .list_attached_role_policies("LambdaExecRole")
        .expect("list attached policies");
    assert_eq!(attached.len(), 1);
    assert_eq!(attached[0].0, "CustomS3WritePolicy");

    // 4. Inline policy on role
    state
        .put_role_policy(
            "LambdaExecRole",
            "InlineKmsPolicy",
            r#"{"Effect":"Allow","Action":"kms:*"}"#,
        )
        .expect("put role policy");
    let inline_doc = state
        .get_role_policy("LambdaExecRole", "InlineKmsPolicy")
        .expect("get role policy");
    assert_eq!(inline_doc, r#"{"Effect":"Allow","Action":"kms:*"}"#);

    // 5. Create User & Access Key
    let user = state
        .create_user("ci-deployer".to_string(), None)
        .expect("create user");
    assert_eq!(user.user_name, "ci-deployer");

    let key = state
        .create_access_key("ci-deployer")
        .expect("create access key");
    assert!(key.access_key_id.starts_with("AKIA"));
    assert_eq!(key.status, "Active");

    let keys = state.list_access_keys("ci-deployer");
    assert_eq!(keys.len(), 1);

    // 6. Snapshot and restore
    let snap = state.export_snapshot();
    let new_state = IamState::new("000000000000".to_string());
    new_state.import_snapshot(snap);

    let restored_role = new_state.get_role("LambdaExecRole").expect("restored role");
    assert_eq!(restored_role.role_name, "LambdaExecRole");
    assert_eq!(restored_role.attached_policies.len(), 1);

    // 7. Reset
    new_state.reset();
    assert!(new_state.get_role("LambdaExecRole").is_err());
}
