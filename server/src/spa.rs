use axum::{http::header::CONTENT_TYPE, response::IntoResponse};
use std::borrow::Cow;

fn is_release() -> bool {
    std::env::var_os("CARGO_PKG_NAME").is_none()
}

struct HotResource {
    content: &'static str,
    from_path: &'static str,
    mime: &'static str,
}

macro_rules! HotResource {
    (path: $path:literal, mime: $mime:literal $(,)*) => {
        HotResource {
            content: include_str!($path),
            from_path: concat!(env!("CARGO_MANIFEST_DIR"), "/src/", $path),
            mime: $mime,
        }
    };
}

pub async fn frontpage() -> impl IntoResponse {
    HotResource! {
        path: "index.html",
        mime: "text/html",
    }
    .or_load()
    .await
}

pub async fn static_style_css() -> impl IntoResponse {
    HotResource! {
        path: "style.css",
        mime: "text/css",
    }
    .or_load()
    .await
}

impl HotResource {
    async fn or_load(self) -> impl IntoResponse {
        let headers = [(CONTENT_TYPE, self.mime)];

        if is_release() {
            return (headers, Cow::Borrowed(self.content));
        }

        let content = std::fs::read_to_string(self.from_path).unwrap();
        return (headers, Cow::Owned(content));
    }
}
