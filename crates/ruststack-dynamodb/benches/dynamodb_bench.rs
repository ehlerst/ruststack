use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ruststack_dynamodb::{AttributeDefinition, AttributeValue, DynamoDbEngine, KeySchemaElement};
use std::collections::HashMap;

fn dynamodb_benchmark(c: &mut Criterion) {
    let ddb = DynamoDbEngine::new("000000000000".to_string(), "us-east-1".to_string());

    ddb.create_table(
        "BenchTable".to_string(),
        vec![
            KeySchemaElement {
                attribute_name: "pk".to_string(),
                key_type: "HASH".to_string(),
            },
            KeySchemaElement {
                attribute_name: "sk".to_string(),
                key_type: "RANGE".to_string(),
            },
        ],
        vec![
            AttributeDefinition {
                attribute_name: "pk".to_string(),
                attribute_type: "S".to_string(),
            },
            AttributeDefinition {
                attribute_name: "sk".to_string(),
                attribute_type: "N".to_string(),
            },
        ],
        Some("PAY_PER_REQUEST".to_string()),
        None,
        None,
    )
    .unwrap();

    // Pre-populate 1000 items
    for i in 0..1000 {
        let mut item = HashMap::new();
        item.insert(
            "pk".to_string(),
            AttributeValue::S(format!("user_{}", i % 10)),
        );
        item.insert("sk".to_string(), AttributeValue::N(i.to_string()));
        item.insert(
            "name".to_string(),
            AttributeValue::S(format!("User Name {}", i)),
        );
        item.insert(
            "email".to_string(),
            AttributeValue::S(format!("user_{}@example.com", i)),
        );
        ddb.put_item("BenchTable", item, None, None, None).unwrap();
    }

    let mut group = c.benchmark_group("dynamodb_operations");

    group.bench_function("put_item", |b| {
        let mut i = 10000;
        b.iter(|| {
            i += 1;
            let mut item = HashMap::new();
            item.insert("pk".to_string(), AttributeValue::S("user_1".to_string()));
            item.insert("sk".to_string(), AttributeValue::N(i.to_string()));
            item.insert(
                "data".to_string(),
                AttributeValue::S("benchmark_payload_val".to_string()),
            );
            let res = ddb.put_item("BenchTable", item, None, None, None);
            black_box(res).unwrap();
        });
    });

    group.bench_function("get_item_pk_sk", |b| {
        let mut key = HashMap::new();
        key.insert("pk".to_string(), AttributeValue::S("user_0".to_string()));
        key.insert("sk".to_string(), AttributeValue::N("100".to_string()));

        b.iter(|| {
            let res = ddb.get_item("BenchTable", black_box(&key), None, None);
            black_box(res).unwrap();
        });
    });

    group.bench_function("query_pk_with_sk_range", |b| {
        let mut attr_values = HashMap::new();
        attr_values.insert(":pk".to_string(), AttributeValue::S("user_0".to_string()));
        attr_values.insert(":sk_min".to_string(), AttributeValue::N("100".to_string()));
        attr_values.insert(":sk_max".to_string(), AttributeValue::N("300".to_string()));

        b.iter(|| {
            let res = ddb.query(
                "BenchTable",
                None,
                black_box("pk = :pk AND sk BETWEEN :sk_min AND :sk_max"),
                None,
                Some(true),
                Some(50),
                None,
                Some(&attr_values),
            );
            black_box(res).unwrap();
        });
    });

    group.finish();
}

criterion_group!(benches, dynamodb_benchmark);
criterion_main!(benches);
