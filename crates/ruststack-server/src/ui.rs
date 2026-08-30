use axum::response::{Html, IntoResponse};

pub const EMBEDDED_UI_HTML: &str = include_str!("index.html");

pub async fn serve_ui() -> impl IntoResponse {
    Html(EMBEDDED_UI_HTML)
}
