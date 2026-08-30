use crate::types::AttributeValue;
use std::collections::HashMap;

pub fn evaluate_expression(
    expr: &str,
    item: &HashMap<String, AttributeValue>,
    attr_names: Option<&HashMap<String, String>>,
    attr_values: Option<&HashMap<String, AttributeValue>>,
) -> bool {
    let expr = expr.trim();
    if expr.is_empty() {
        return true;
    }

    // Split on top-level AND (case-insensitive)
    let parts: Vec<&str> = split_conjunctions(expr, " AND ");
    for part in parts {
        if !evaluate_simple_clause(part.trim(), item, attr_names, attr_values) {
            return false;
        }
    }
    true
}

fn split_conjunctions<'a>(expr: &'a str, conj: &str) -> Vec<&'a str> {
    let mut results = Vec::new();
    let mut last = 0;
    let mut paren_depth: usize = 0;

    let bytes = expr.as_bytes();
    let conj_bytes = conj.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'(' {
            paren_depth += 1;
        } else if bytes[i] == b')' {
            paren_depth = paren_depth.saturating_sub(1);
        } else if paren_depth == 0 && i + conj_bytes.len() <= bytes.len() {
            let slice = &bytes[i..i + conj_bytes.len()];
            if slice.eq_ignore_ascii_case(conj_bytes) {
                results.push(&expr[last..i]);
                i += conj_bytes.len();
                last = i;
                continue;
            }
        }
        i += 1;
    }
    results.push(&expr[last..]);
    results
}

fn resolve_name<'a>(name: &'a str, attr_names: Option<&'a HashMap<String, String>>) -> &'a str {
    let trimmed = name.trim();
    if let Some(map) = attr_names {
        if let Some(real_name) = map.get(trimmed) {
            return real_name.as_str();
        }
    }
    trimmed
}

fn resolve_val<'a>(
    placeholder: &'a str,
    attr_values: Option<&'a HashMap<String, AttributeValue>>,
) -> Option<&'a AttributeValue> {
    let trimmed = placeholder.trim();
    if let Some(map) = attr_values {
        return map.get(trimmed);
    }
    None
}

fn evaluate_simple_clause(
    clause: &str,
    item: &HashMap<String, AttributeValue>,
    attr_names: Option<&HashMap<String, String>>,
    attr_values: Option<&HashMap<String, AttributeValue>>,
) -> bool {
    let clause = clause.trim();

    // begins_with(attr, :val)
    if clause.starts_with("begins_with(") && clause.ends_with(')') {
        let inner = &clause[12..clause.len() - 1];
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 2 {
            let field_name = resolve_name(parts[0], attr_names);
            let target_val = resolve_val(parts[1], attr_values);

            if let (Some(AttributeValue::S(item_str)), Some(AttributeValue::S(prefix))) =
                (item.get(field_name), target_val)
            {
                return item_str.starts_with(prefix);
            }
        }
        return false;
    }

    // attr BETWEEN :a AND :b
    if let Some(pos) = clause.to_uppercase().find(" BETWEEN ") {
        let field_part = &clause[..pos];
        let rest = &clause[pos + 9..];
        let and_parts: Vec<&str> = split_conjunctions(rest, " AND ");
        if and_parts.len() == 2 {
            let field_name = resolve_name(field_part, attr_names);
            let val_a = resolve_val(and_parts[0], attr_values);
            let val_b = resolve_val(and_parts[1], attr_values);

            if let (Some(item_val), Some(va), Some(vb)) = (item.get(field_name), val_a, val_b) {
                return item_val >= va && item_val <= vb;
            }
        }
        return false;
    }

    // Comparison operators: <=, >=, <>, !=, <, >, =
    let ops = ["<=", ">=", "<>", "!=", "<", ">", "="];
    for op in ops {
        if let Some(pos) = clause.find(op) {
            let left_str = &clause[..pos];
            let right_str = &clause[pos + op.len()..];

            let field_name = resolve_name(left_str, attr_names);
            let target_val = resolve_val(right_str, attr_values);

            let item_val = item.get(field_name);

            return match (op, item_val, target_val) {
                ("=", Some(iv), Some(tv)) => iv == tv,
                ("<>", Some(iv), Some(tv)) | ("!=", Some(iv), Some(tv)) => iv != tv,
                ("<", Some(iv), Some(tv)) => iv < tv,
                (">", Some(iv), Some(tv)) => iv > tv,
                ("<=", Some(iv), Some(tv)) => iv <= tv,
                (">=", Some(iv), Some(tv)) => iv >= tv,
                ("=", None, None) => true,
                _ => false,
            };
        }
    }

    true
}
