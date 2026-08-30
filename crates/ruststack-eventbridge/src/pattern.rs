use serde_json::Value;

pub fn matches_event_pattern(pattern_json: &str, event: &Value) -> bool {
    let pattern: Value = match serde_json::from_str(pattern_json) {
        Ok(v) => v,
        Err(_) => return false,
    };

    match_json_object(&pattern, event)
}

fn match_json_object(pattern: &Value, event: &Value) -> bool {
    let pat_obj = match pattern.as_object() {
        Some(obj) => obj,
        None => return false,
    };

    for (key, pat_val) in pat_obj {
        let event_val = match event.get(key) {
            Some(v) => v,
            None => {
                // If pattern is checking exists: false
                if let Some(arr) = pat_val.as_array() {
                    if arr
                        .iter()
                        .any(|item| item.get("exists").and_then(|b| b.as_bool()) == Some(false))
                    {
                        continue;
                    }
                }
                return false;
            }
        };

        if !match_field(pat_val, event_val) {
            return false;
        }
    }

    true
}

fn match_field(pat_val: &Value, event_val: &Value) -> bool {
    if let Some(arr) = pat_val.as_array() {
        // AWS EventBridge patterns: array means logical OR
        for item in arr {
            if match_single_condition(item, event_val) {
                return true;
            }
        }
        return false;
    }

    if let Some(obj) = pat_val.as_object() {
        if let Some(event_obj) = event_val.as_object() {
            let wrapped_event = Value::Object(event_obj.clone());
            return match_json_object(&Value::Object(obj.clone()), &wrapped_event);
        }
    }

    false
}

fn match_single_condition(condition: &Value, event_val: &Value) -> bool {
    // 1. Literal string match
    if let (Some(cond_str), Some(evt_str)) = (condition.as_str(), event_val.as_str()) {
        return cond_str == evt_str;
    }

    // 2. Literal number match
    if let (Some(cond_num), Some(evt_num)) = (condition.as_f64(), event_val.as_f64()) {
        return (cond_num - evt_num).abs() < f64::EPSILON;
    }

    // 3. Literal boolean match
    if let (Some(cond_b), Some(evt_b)) = (condition.as_bool(), event_val.as_bool()) {
        return cond_b == evt_b;
    }

    // 4. Object conditions (prefix, anything-but, exists, numeric)
    if let Some(obj) = condition.as_object() {
        if let Some(prefix_val) = obj.get("prefix").and_then(|v| v.as_str()) {
            if let Some(evt_str) = event_val.as_str() {
                return evt_str.starts_with(prefix_val);
            }
        }

        if let Some(exists_val) = obj.get("exists").and_then(|v| v.as_bool()) {
            return exists_val != event_val.is_null();
        }

        if let Some(anything_but) = obj.get("anything-but") {
            if let Some(arr) = anything_but.as_array() {
                for excluded in arr {
                    if match_single_condition(excluded, event_val) {
                        return false;
                    }
                }
                return true;
            } else {
                return !match_single_condition(anything_but, event_val);
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_simple_source_match() {
        let pattern = r#"{"source": ["aws.ec2", "my.app"]}"#;
        let event = json!({
            "source": "my.app",
            "detail-type": "EC2 Instance State-change Notification",
            "detail": {
                "state": "running"
            }
        });
        assert!(matches_event_pattern(pattern, &event));

        let non_matching_event = json!({
            "source": "other.app"
        });
        assert!(!matches_event_pattern(pattern, &non_matching_event));
    }

    #[test]
    fn test_nested_detail_and_prefix_match() {
        let pattern = r#"{
            "source": ["ecommerce.orders"],
            "detail-type": ["OrderCreated"],
            "detail": {
                "order_id": [{"prefix": "ORD-"}],
                "status": ["PENDING", "PROCESSING"]
            }
        }"#;

        let event = json!({
            "source": "ecommerce.orders",
            "detail-type": "OrderCreated",
            "detail": {
                "order_id": "ORD-12345",
                "status": "PENDING"
            }
        });
        assert!(matches_event_pattern(pattern, &event));

        let wrong_prefix = json!({
            "source": "ecommerce.orders",
            "detail-type": "OrderCreated",
            "detail": {
                "order_id": "INV-12345",
                "status": "PENDING"
            }
        });
        assert!(!matches_event_pattern(pattern, &wrong_prefix));
    }
}
