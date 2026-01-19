//! Static asset serving for embedded files.

use axum::{
    Router,
    http::{HeaderValue, header},
    response::{IntoResponse, Response},
    routing::get,
};

const THEME_CSS: &str = include_str!("assets/theme.css");
const LANDING_CSS: &str = include_str!("assets/landing.css");
const DASHBOARD_CSS: &str = include_str!("assets/dashboard.css");
const DASHBOARD_JS: &str = include_str!("assets/dashboard.js");

fn css_response(content: &'static str) -> Response {
    (
        [(header::CONTENT_TYPE, HeaderValue::from_static("text/css"))],
        content,
    )
        .into_response()
}

fn js_response(content: &'static str) -> Response {
    (
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/javascript"),
        )],
        content,
    )
        .into_response()
}

async fn theme_css() -> Response {
    css_response(THEME_CSS)
}

async fn landing_css() -> Response {
    css_response(LANDING_CSS)
}

async fn dashboard_css() -> Response {
    css_response(DASHBOARD_CSS)
}

async fn dashboard_js() -> Response {
    js_response(DASHBOARD_JS)
}

/// Creates a router for serving static assets.
pub fn router() -> Router {
    Router::new()
        .route("/theme.css", get(theme_css))
        .route("/landing.css", get(landing_css))
        .route("/dashboard.css", get(dashboard_css))
        .route("/dashboard.js", get(dashboard_js))
}
