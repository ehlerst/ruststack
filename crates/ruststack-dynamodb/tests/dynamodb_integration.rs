use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use ruststack_dynamodb::{handle_dynamodb_request, DynamoDbEngine};
use serde_json::Value;
use std::sync::Arc;

fn setup_dynamodb() -> Arc<DynamoDbEngine> {
    Arc::new(DynamoDbEngine::new(
        "000000000000".to_string(),
        "us-east-1".to_string(),
    ))
}

#[tokio::test]
async fn test_dynamodb_table_lifecycle_and_item_crud() {
    let ddb = setup_dynamodb();

    // 1. Create Table
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "DynamoDB_20120810.CreateTable")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({
                "TableName": "UsersTable",
                "KeySchema": [
                    { "AttributeName": "userId", "KeyType": "HASH" },
                    { "AttributeName": "timestamp", "KeyType": "RANGE" }
                ],
                "AttributeDefinitions": [
                    { "AttributeName": "userId", "AttributeType": "S" },
                    { "AttributeName": "timestamp", "AttributeType": "N" }
                ],
                "BillingMode": "PAY_PER_REQUEST"
            })
            .to_string(),
        ))
        .unwrap();

    let resp = handle_dynamodb_request(ddb.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        val["TableDescription"]["TableStatus"].as_str().unwrap(),
        "ACTIVE"
    );

    // 2. PutItem
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "DynamoDB_20120810.PutItem")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({
                "TableName": "UsersTable",
                "Item": {
                    "userId": { "S": "usr_1001" },
                    "timestamp": { "N": "1700000000" },
                    "email": { "S": "alex@example.com" },
                    "active": { "BOOL": true }
                }
            })
            .to_string(),
        ))
        .unwrap();

    let resp = handle_dynamodb_request(ddb.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. GetItem
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "DynamoDB_20120810.GetItem")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({
                "TableName": "UsersTable",
                "Key": {
                    "userId": { "S": "usr_1001" },
                    "timestamp": { "N": "1700000000" }
                }
            })
            .to_string(),
        ))
        .unwrap();

    let resp = handle_dynamodb_request(ddb.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        val["Item"]["email"]["S"].as_str().unwrap(),
        "alex@example.com"
    );
    assert!(val["Item"]["active"]["BOOL"].as_bool().unwrap());

    // 4. UpdateItem
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "DynamoDB_20120810.UpdateItem")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({
                "TableName": "UsersTable",
                "Key": {
                    "userId": { "S": "usr_1001" },
                    "timestamp": { "N": "1700000000" }
                },
                "UpdateExpression": "SET email = :new_email, plan = :plan",
                "ExpressionAttributeValues": {
                    ":new_email": { "S": "alex_updated@example.com" },
                    ":plan": { "S": "PRO" }
                },
                "ReturnValues": "ALL_NEW"
            })
            .to_string(),
        ))
        .unwrap();

    let resp = handle_dynamodb_request(ddb.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        val["Attributes"]["email"]["S"].as_str().unwrap(),
        "alex_updated@example.com"
    );
    assert_eq!(val["Attributes"]["plan"]["S"].as_str().unwrap(), "PRO");

    // 5. DeleteItem
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "DynamoDB_20120810.DeleteItem")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({
                "TableName": "UsersTable",
                "Key": {
                    "userId": { "S": "usr_1001" },
                    "timestamp": { "N": "1700000000" }
                }
            })
            .to_string(),
        ))
        .unwrap();

    let resp = handle_dynamodb_request(ddb.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_dynamodb_query_and_scan() {
    let ddb = setup_dynamodb();

    // Create Orders table
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "DynamoDB_20120810.CreateTable")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({
                "TableName": "OrdersTable",
                "KeySchema": [
                    { "AttributeName": "customerId", "KeyType": "HASH" },
                    { "AttributeName": "orderId", "KeyType": "RANGE" }
                ],
                "AttributeDefinitions": [
                    { "AttributeName": "customerId", "AttributeType": "S" },
                    { "AttributeName": "orderId", "AttributeType": "S" }
                ],
                "BillingMode": "PAY_PER_REQUEST"
            })
            .to_string(),
        ))
        .unwrap();
    let _ = handle_dynamodb_request(ddb.clone(), req).await;

    // Seed 4 orders for cust_A and 1 for cust_B
    let orders = vec![
        ("cust_A", "ord_101", 50.0, "PENDING"),
        ("cust_A", "ord_102", 150.0, "SHIPPED"),
        ("cust_A", "ord_103", 250.0, "DELIVERED"),
        ("cust_A", "ord_201", 30.0, "PENDING"),
        ("cust_B", "ord_301", 99.0, "SHIPPED"),
    ];

    for (cust, ord, amount, status) in orders {
        let req = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header("x-amz-target", "DynamoDB_20120810.PutItem")
            .header("content-type", "application/x-amz-json-1.0")
            .body(Body::from(
                serde_json::json!({
                    "TableName": "OrdersTable",
                    "Item": {
                        "customerId": { "S": cust },
                        "orderId": { "S": ord },
                        "amount": { "N": amount.to_string() },
                        "status": { "S": status }
                    }
                })
                .to_string(),
            ))
            .unwrap();
        let _ = handle_dynamodb_request(ddb.clone(), req).await;
    }

    // Query: customerId = :cid and begins_with(orderId, :prefix)
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "DynamoDB_20120810.Query")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({
                "TableName": "OrdersTable",
                "KeyConditionExpression": "customerId = :cid AND begins_with(orderId, :prefix)",
                "ExpressionAttributeValues": {
                    ":cid": { "S": "cust_A" },
                    ":prefix": { "S": "ord_1" }
                }
            })
            .to_string(),
        ))
        .unwrap();

    let resp = handle_dynamodb_request(ddb.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    let items = val["Items"].as_array().unwrap();
    assert_eq!(items.len(), 3);

    // Scan: filter by status = SHIPPED
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "DynamoDB_20120810.Scan")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({
                "TableName": "OrdersTable",
                "FilterExpression": "#s = :target_status",
                "ExpressionAttributeNames": {
                    "#s": "status"
                },
                "ExpressionAttributeValues": {
                    ":target_status": { "S": "SHIPPED" }
                }
            })
            .to_string(),
        ))
        .unwrap();

    let resp = handle_dynamodb_request(ddb.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    let items = val["Items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
}

#[tokio::test]
async fn test_dynamodb_streams_lifecycle() {
    let ddb = setup_dynamodb();

    // 1. Create Table with Streams Enabled
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "DynamoDB_20120810.CreateTable")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({
                "TableName": "StreamedTable",
                "KeySchema": [
                    { "AttributeName": "pk", "KeyType": "HASH" }
                ],
                "AttributeDefinitions": [
                    { "AttributeName": "pk", "AttributeType": "S" }
                ],
                "StreamSpecification": {
                    "StreamEnabled": true,
                    "StreamViewType": "NEW_AND_OLD_IMAGES"
                }
            })
            .to_string(),
        ))
        .unwrap();

    let resp = handle_dynamodb_request(ddb.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    let stream_arn = val["TableDescription"]["LatestStreamArn"]
        .as_str()
        .unwrap()
        .to_string();

    // 2. Put Item
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "DynamoDB_20120810.PutItem")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({
                "TableName": "StreamedTable",
                "Item": {
                    "pk": { "S": "item-1" },
                    "data": { "S": "initial" }
                }
            })
            .to_string(),
        ))
        .unwrap();
    let _ = handle_dynamodb_request(ddb.clone(), req).await;

    // 3. Update Item
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "DynamoDB_20120810.PutItem")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({
                "TableName": "StreamedTable",
                "Item": {
                    "pk": { "S": "item-1" },
                    "data": { "S": "updated" }
                }
            })
            .to_string(),
        ))
        .unwrap();
    let _ = handle_dynamodb_request(ddb.clone(), req).await;

    // 4. List Streams
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "DynamoDBStreams_20120810.ListStreams")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({ "TableName": "StreamedTable" }).to_string(),
        ))
        .unwrap();
    let resp = handle_dynamodb_request(ddb.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(val["Streams"].as_array().unwrap().len(), 1);

    // 5. Describe Stream
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "DynamoDBStreams_20120810.DescribeStream")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({ "StreamArn": stream_arn }).to_string(),
        ))
        .unwrap();
    let resp = handle_dynamodb_request(ddb.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    let shard_id = val["StreamDescription"]["Shards"][0]["ShardId"]
        .as_str()
        .unwrap()
        .to_string();

    // 6. Get Shard Iterator
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "DynamoDBStreams_20120810.GetShardIterator")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({
                "StreamArn": stream_arn,
                "ShardId": shard_id,
                "ShardIteratorType": "TRIM_HORIZON"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = handle_dynamodb_request(ddb.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    let iterator = val["ShardIterator"].as_str().unwrap();

    // 7. Get Records
    let req = Request::builder()
        .method(Method::POST)
        .uri("/")
        .header("x-amz-target", "DynamoDBStreams_20120810.GetRecords")
        .header("content-type", "application/x-amz-json-1.0")
        .body(Body::from(
            serde_json::json!({ "ShardIterator": iterator }).to_string(),
        ))
        .unwrap();
    let resp = handle_dynamodb_request(ddb.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    let records = val["Records"].as_array().unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0]["eventName"].as_str().unwrap(), "INSERT");
    assert_eq!(records[1]["eventName"].as_str().unwrap(), "MODIFY");
}
