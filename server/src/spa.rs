use axum::{
    http::header::CONTENT_TYPE,
    response::{AppendHeaders, IntoResponse},
};

pub async fn frontpage() -> impl IntoResponse {
    const INDEX: &'static str = include_str!("index.html");
    (AppendHeaders([(CONTENT_TYPE, "text/html")]), INDEX)
}

pub async fn static_style_css() -> impl IntoResponse {
    const STYLE: &'static str = include_str!("style.css");
    (AppendHeaders([(CONTENT_TYPE, "text/css")]), STYLE)
}
