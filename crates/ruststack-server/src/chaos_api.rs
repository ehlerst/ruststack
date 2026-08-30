use crate::app::AppState;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{Request, Response, StatusCode};
use http_body_util::BodyExt;
use ruststack_core::ChaosRule;
use serde_json::json;

pub async fn list_chaos_rules_handler(State(state): State<AppState>) -> Response<Body> {
    let rules = state.chaos_engine.get_rules();
    let resp = json!({
        "status": "ok",
        "enabled": state.chaos_engine.is_enabled(),
        "rules_count": rules.len(),
        "rules": rules,
    });

    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Body::from(resp.to_string()))
        .unwrap()
}

pub async fn add_chaos_rule_handler(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Response<Body> {
    let body_bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(e.to_string()))
                .unwrap();
        }
    };

    let rule: ChaosRule = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(e) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(format!("Invalid ChaosRule JSON: {}", e)))
                .unwrap();
        }
    };

    let id = state.chaos_engine.add_rule(rule);

    let resp = json!({
        "status": "ok",
        "message": "Chaos rule registered successfully",
        "rule_id": id
    });

    Response::builder()
        .status(StatusCode::CREATED)
        .header("content-type", "application/json")
        .body(Body::from(resp.to_string()))
        .unwrap()
}

pub async fn delete_chaos_rule_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response<Body> {
    let removed = state.chaos_engine.remove_rule(&id);
    if removed {
        let resp = json!({
            "status": "ok",
            "message": format!("Chaos rule {} deleted", id)
        });
        Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(Body::from(resp.to_string()))
            .unwrap()
    } else {
        let resp = json!({
            "status": "error",
            "message": format!("Chaos rule {} not found", id)
        });
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("content-type", "application/json")
            .body(Body::from(resp.to_string()))
            .unwrap()
    }
}

pub async fn clear_chaos_rules_handler(State(state): State<AppState>) -> Response<Body> {
    state.chaos_engine.clear_rules();
    let resp = json!({
        "status": "ok",
        "message": "All chaos rules cleared"
    });
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Body::from(resp.to_string()))
        .unwrap()
}

pub async fn enable_chaos_handler(State(state): State<AppState>) -> Response<Body> {
    state.chaos_engine.set_enabled(true);
    let resp = json!({
        "status": "ok",
        "message": "Chaos engine enabled",
        "enabled": true
    });
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Body::from(resp.to_string()))
        .unwrap()
}

pub async fn disable_chaos_handler(State(state): State<AppState>) -> Response<Body> {
    state.chaos_engine.set_enabled(false);
    let resp = json!({
        "status": "ok",
        "message": "Chaos engine disabled",
        "enabled": false
    });
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Body::from(resp.to_string()))
        .unwrap()
}
