use axum::{http::header::CONTENT_TYPE, response::IntoResponse};

pub async fn frontpage() -> impl IntoResponse {
    const INDEX: &str = include_str!("index.html");
    ([(CONTENT_TYPE, "text/html")], INDEX)
}

pub async fn static_style_css() -> impl IntoResponse {
    const STYLE: &str = include_str!("style.css");
    ([(CONTENT_TYPE, "text/css")], STYLE)
}
