//! Shared layout components for the Hubdash application.

use maud::{DOCTYPE, Markup, PreEscaped, html};

/// Renders the base HTML layout with common head elements.
pub fn base_layout(title: &str, styles: &[&str], scripts: &[&str], body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) }
                @for style in styles {
                    link rel="stylesheet" href=(style);
                }
                script src="https://unpkg.com/htmx.org@2.0.4" {}
                @for script in scripts {
                    script src=(script) {}
                }
                script defer src="https://unpkg.com/alpinejs@3.14.8/dist/cdn.min.js" {}
            }
            body {
                (body)
            }
        }
    }
}

/// Renders a checkmark or X icon based on a boolean value.
pub fn check_icon(checked: bool) -> Markup {
    if checked {
        html! { span class="check-icon check-yes" { (PreEscaped("✓")) } }
    } else {
        html! { span class="check-icon check-no" { (PreEscaped("✗")) } }
    }
}
