//! Landing page for the Hubdash application.

use axum::response::IntoResponse;
use maud::html;

use crate::layout::base_layout;

/// Renders the landing page.
pub async fn landing_page() -> impl IntoResponse {
    let body = html! {
        div class="container" {
            h1 { "Hubdash" }
            p { "Monitor your GitHub repositories' CI/CD pipeline health and dependency freshness in one place." }
            a href="/auth/signin" class="sign-in-btn" { "Sign in with GitHub" }
        }
    };

    base_layout(
        "Hubdash",
        &["/assets/theme.css", "/assets/landing.css"],
        &[],
        body,
    )
}
