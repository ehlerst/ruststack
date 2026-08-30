use axum::http::StatusCode;
use ruststack_compat_tests::RustStackTestClient;
use serde_json::json;

#[tokio::test]
async fn test_dynamodb_table_crud_and_item_operations() {
    let client = RustStackTestClient::new();

    // 1. CreateTable
    let (status, val) = client
        .call_json(
            "DynamoDB_20120810.CreateTable",
            json!({
                "TableName": "Users",
                "KeySchema": [
                    { "AttributeName": "userId", "KeyType": "HASH" },
                    { "AttributeName": "timestamp", "KeyType": "RANGE" }
                ],
                "AttributeDefinitions": [
                    { "AttributeName": "userId", "AttributeType": "S" },
                    { "AttributeName": "timestamp", "AttributeType": "N" }
                ],
                "BillingMode": "PAY_PER_REQUEST"
            }),
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        val["TableDescription"]["TableStatus"].as_str().unwrap(),
        "ACTIVE"
    );
    assert_eq!(
        val["TableDescription"]["TableName"].as_str().unwrap(),
        "Users"
    );

    // 2. DescribeTable
    let (status, val) = client
        .call_json(
            "DynamoDB_20120810.DescribeTable",
            json!({ "TableName": "Users" }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(val["Table"]["ItemCount"].as_i64().unwrap(), 0);

    // 3. ListTables
    let (status, val) = client
        .call_json("DynamoDB_20120810.ListTables", json!({}))
        .await;
    assert_eq!(status, StatusCode::OK);
    let tables = val["TableNames"].as_array().unwrap();
    assert!(tables.iter().any(|t| t.as_str().unwrap() == "Users"));

    // 4. PutItem with various data types (S, N, BOOL, L, M)
    let (status, _) = client
        .call_json(
            "DynamoDB_20120810.PutItem",
            json!({
                "TableName": "Users",
                "Item": {
                    "userId": { "S": "usr_abc" },
                    "timestamp": { "N": "1700000000" },
                    "name": { "S": "Alice" },
                    "active": { "BOOL": true },
                    "tags": { "L": [{ "S": "admin" }, { "S": "developer" }] },
                    "profile": {
                        "M": {
                            "age": { "N": "30" },
                            "city": { "S": "Seattle" }
                        }
                    }
                }
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // 5. GetItem with projection
    let (status, val) = client
        .call_json(
            "DynamoDB_20120810.GetItem",
            json!({
                "TableName": "Users",
                "Key": {
                    "userId": { "S": "usr_abc" },
                    "timestamp": { "N": "1700000000" }
                },
                "ProjectionExpression": "name, active, profile"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(val["Item"]["name"]["S"].as_str().unwrap(), "Alice");
    assert!(val["Item"]["active"]["BOOL"].as_bool().unwrap());
    assert_eq!(
        val["Item"]["profile"]["M"]["city"]["S"].as_str().unwrap(),
        "Seattle"
    );

    // 6. UpdateItem (SET & ReturnValues ALL_NEW)
    let (status, val) = client
        .call_json(
            "DynamoDB_20120810.UpdateItem",
            json!({
                "TableName": "Users",
                "Key": {
                    "userId": { "S": "usr_abc" },
                    "timestamp": { "N": "1700000000" }
                },
                "UpdateExpression": "SET #loc = :new_loc, #s = :status",
                "ExpressionAttributeNames": {
                    "#loc": "city",
                    "#s": "user_status"
                },
                "ExpressionAttributeValues": {
                    ":new_loc": { "S": "San Francisco" },
                    ":status": { "S": "ACTIVE_VERIFIED" }
                },
                "ReturnValues": "ALL_NEW"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        val["Attributes"]["city"]["S"].as_str().unwrap(),
        "San Francisco"
    );
    assert_eq!(
        val["Attributes"]["user_status"]["S"].as_str().unwrap(),
        "ACTIVE_VERIFIED"
    );

    // 7. DeleteItem with ReturnValues ALL_OLD
    let (status, val) = client
        .call_json(
            "DynamoDB_20120810.DeleteItem",
            json!({
                "TableName": "Users",
                "Key": {
                    "userId": { "S": "usr_abc" },
                    "timestamp": { "N": "1700000000" }
                },
                "ReturnValues": "ALL_OLD"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(val["Attributes"]["name"]["S"].as_str().unwrap(), "Alice");

    // 8. DeleteTable
    let (status, val) = client
        .call_json(
            "DynamoDB_20120810.DeleteTable",
            json!({ "TableName": "Users" }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        val["TableDescription"]["TableStatus"].as_str().unwrap(),
        "DELETING"
    );
}

#[tokio::test]
async fn test_dynamodb_batch_and_query_filtering() {
    let client = RustStackTestClient::new();

    // Create Products table
    client
        .call_json(
            "DynamoDB_20120810.CreateTable",
            json!({
                "TableName": "Products",
                "KeySchema": [
                    { "AttributeName": "category", "KeyType": "HASH" },
                    { "AttributeName": "productId", "KeyType": "RANGE" }
                ],
                "AttributeDefinitions": [
                    { "AttributeName": "category", "AttributeType": "S" },
                    { "AttributeName": "productId", "AttributeType": "S" }
                ],
                "BillingMode": "PAY_PER_REQUEST"
            }),
        )
        .await;

    // BatchWriteItem
    let (status, val) = client
        .call_json(
            "DynamoDB_20120810.BatchWriteItem",
            json!({
                "RequestItems": {
                    "Products": [
                        {
                            "PutRequest": {
                                "Item": {
                                    "category": { "S": "electronics" },
                                    "productId": { "S": "prod_001" },
                                    "name": { "S": "Laptop" },
                                    "price": { "N": "1200" }
                                }
                            }
                        },
                        {
                            "PutRequest": {
                                "Item": {
                                    "category": { "S": "electronics" },
                                    "productId": { "S": "prod_002" },
                                    "name": { "S": "Headphones" },
                                    "price": { "N": "150" }
                                }
                            }
                        },
                        {
                            "PutRequest": {
                                "Item": {
                                    "category": { "S": "books" },
                                    "productId": { "S": "book_101" },
                                    "name": { "S": "Rust Book" },
                                    "price": { "N": "45" }
                                }
                            }
                        }
                    ]
                }
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(val["UnprocessedItems"].as_object().unwrap().is_empty());

    // BatchGetItem
    let (status, val) = client
        .call_json(
            "DynamoDB_20120810.BatchGetItem",
            json!({
                "RequestItems": {
                    "Products": {
                        "Keys": [
                            {
                                "category": { "S": "electronics" },
                                "productId": { "S": "prod_001" }
                            },
                            {
                                "category": { "S": "books" },
                                "productId": { "S": "book_101" }
                            }
                        ]
                    }
                }
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let items = val["Responses"]["Products"].as_array().unwrap();
    assert_eq!(items.len(), 2);

    // Query with begins_with and price filter
    let (status, val) = client
        .call_json(
            "DynamoDB_20120810.Query",
            json!({
                "TableName": "Products",
                "KeyConditionExpression": "category = :cat AND begins_with(productId, :pfx)",
                "FilterExpression": "price > :min_price",
                "ExpressionAttributeValues": {
                    ":cat": { "S": "electronics" },
                    ":pfx": { "S": "prod_" },
                    ":min_price": { "N": "500" }
                }
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let query_items = val["Items"].as_array().unwrap();
    assert_eq!(query_items.len(), 1);
    assert_eq!(query_items[0]["name"]["S"].as_str().unwrap(), "Laptop");
}
