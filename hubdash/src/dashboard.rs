//! Dashboard page showing repository CI/CD health.

use axum::Extension;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use maud::{Markup, PreEscaped, html};
use tracing::error;
use url::Url;

use crate::github::oauth;
use crate::github::{CheckStatus, Repository};
use crate::layout::base_layout;
use crate::session::Session;
use crate::{AppState, Platform};

/// Status of a CI check run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckState {
    Success,
    Failure,
    Pending,
}

impl CheckState {
    pub fn css_class(self) -> &'static str {
        match self {
            Self::Success => "status-badge status-success",
            Self::Failure => "status-badge status-failure",
            Self::Pending => "status-badge status-pending",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failure => "failure",
            Self::Pending => "pending",
        }
    }
}

impl From<CheckStatus> for CheckState {
    fn from(s: CheckStatus) -> Self {
        match s {
            CheckStatus::Success => Self::Success,
            CheckStatus::Failure => Self::Failure,
            CheckStatus::Pending => Self::Pending,
        }
    }
}

/// Renders a status badge with appropriate styling.
fn status_badge(state: CheckState) -> Markup {
    html! { span class=(state.css_class()) { (state.as_str()) } }
}

fn repo_expand_url(owner: &str, repo: &str) -> Url {
    let mut url = Url::parse("relative:/").expect("valid base");
    url.path_segments_mut()
        .expect("cannot be base")
        .push("dashboard")
        .push("repo")
        .push(owner)
        .push(repo)
        .push("expand");
    url
}

/// Generates the Alpine.js `x-data` attribute value for an expandable component.
fn expandable_directive(url: &Url, element_id: &str) -> String {
    format!("expandable('{}', '{}')", url.path(), element_id)
}

/// Repository summary for display on the dashboard.
pub struct RepoSummary {
    pub owner: String,
    pub repo: String,
    pub description: String,
    pub main_status: CheckState,
}

impl RepoSummary {
    fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }

    fn github_url(&self) -> Url {
        let mut url = Url::parse("https://github.com").expect("valid base URL");
        url.path_segments_mut()
            .expect("cannot be base")
            .push(&self.owner)
            .push(&self.repo);
        url
    }
}

/// Converts a GitHub API repository into a [`RepoSummary`].
fn repo_summary_from(repo: &Repository) -> RepoSummary {
    RepoSummary {
        owner: repo.owner.login.clone(),
        repo: repo.name.clone(),
        description: repo.description.clone().unwrap_or_default(),
        main_status: repo
            .main_check_status
            .map(CheckState::from)
            .unwrap_or(CheckState::Pending),
    }
}

/// Returns the expanded detail HTML for a repository row.
pub async fn repo_expand(Path((owner, repo)): Path<(String, String)>) -> impl IntoResponse {
    let summary = RepoSummary {
        owner: owner.clone(),
        repo: repo.clone(),
        description: String::new(),
        main_status: CheckState::Pending,
    };

    let github_url = summary.github_url();
    html! {
        div class="repo-detail" {
            div class="repo-info" {
                a href=(github_url) target="_blank" class="repo-github-link" {
                    (PreEscaped("🔗")) " " (github_url)
                }
                @if !summary.description.is_empty() {
                    p class="repo-description" { (summary.description) }
                }
            }
        }
    }
}

fn repo_table_header() -> Markup {
    html! {
        thead {
            tr {
                th class="expand-header" { }
                th { "Repository" }
                th title="Check status of the default branch HEAD" { "Main" }
            }
        }
    }
}

fn repo_row(repo: &RepoSummary) -> Markup {
    let detail_id = format!("detail-{}-{}", repo.owner, repo.repo);
    let expand_url = repo_expand_url(&repo.owner, &repo.repo);
    html! {
        tbody id=(format!("tbody-{}-{}", repo.owner, repo.repo))
              x-data=(expandable_directive(&expand_url, &detail_id)) {
            tr class="repo-row"
               x-bind:class="{ 'expanded': expanded }"
               x-on:click="toggle()"
            {
                td class="expand-cell" {
                    span class="expand-arrow" { (PreEscaped("▶")) }
                }
                td class="repo-name" { (repo.full_name()) }
                td class="main-status" { (status_badge(repo.main_status)) }
            }
            tr class="repo-detail-row" x-show="expanded" x-cloak {
                td colspan="3" class="repo-detail-cell" {
                    div id=(detail_id) {}
                }
            }
        }
    }
}

fn repo_group(owner: &str, repos: &[RepoSummary], is_personal: bool) -> Markup {
    let label = if is_personal {
        format!("{owner} (personal)")
    } else {
        owner.to_string()
    };
    html! {
        div class="repo-group" x-data="{ open: true }" {
            div class="repo-group-header" x-on:click="open = !open" {
                span class="group-arrow" x-bind:class="{ 'expanded': open }" {
                    (PreEscaped("▶"))
                }
                span class="group-name" { (label) }
                span class="group-count" { "(" (repos.len()) ")" }
            }
            div x-show="open" {
                table class="repo-table" {
                    (repo_table_header())
                    @for repo in repos {
                        (repo_row(repo))
                    }
                }
            }
        }
    }
}

/// Renders the main dashboard page.
///
/// Fetches the repo list with check status via GraphQL; no further per-repo
/// requests are needed.
pub async fn dashboard_page<P: Platform>(
    State(state): State<AppState<P>>,
    Extension(session): Extension<Session>,
) -> impl IntoResponse {
    let http_client = state.plat.create_http_client();
    let api_repos = match oauth::list_user_repos(
        &http_client,
        &state.oauth.api_base_url,
        &session.access_token,
    )
    .await
    {
        Ok(repos) => repos,
        Err(e) => {
            error!("Failed to fetch repositories: {e:?}");
            vec![]
        }
    };

    let username = &session.user.login;
    let mut personal: Vec<RepoSummary> = Vec::new();
    let mut by_org: std::collections::BTreeMap<String, Vec<RepoSummary>> = Default::default();
    for repo in api_repos.iter().map(repo_summary_from) {
        if repo.owner == *username {
            personal.push(repo);
        } else {
            by_org.entry(repo.owner.clone()).or_default().push(repo);
        }
    }

    let has_repos = !personal.is_empty() || !by_org.is_empty();

    let body = html! {
        div class="dashboard-container" {
            header class="dashboard-header" {
                h1 { "Hubdash" }
                a href="/auth/signout" class="sign-out-link" { "Sign out" }
            }
            main class="dashboard-main" {
                @if !has_repos {
                    p class="no-repos" { "No repositories found." }
                } @else {
                    div class="repo-groups" {
                        @if !personal.is_empty() {
                            (repo_group(username, &personal, true))
                        }
                        @for (owner, org_repos) in &by_org {
                            (repo_group(owner, org_repos, false))
                        }
                    }
                }
            }
        }
    };

    base_layout(
        "Dashboard | Hubdash",
        &["/assets/theme.css", "/assets/dashboard.css"],
        &["/assets/dashboard.js"],
        body,
    )
}
