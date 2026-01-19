//! Dashboard page showing repository CI/CD health.

use axum::{extract::Path, response::IntoResponse};
use bitflags::bitflags;
use jiff::SignedDuration;
use maud::{Markup, PreEscaped, html};
use url::Url;

use crate::layout::{base_layout, check_icon};
use crate::mocks::{find_repo, mock_repos};

/// Status of a pipeline run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineStatus {
    Success,
    Failure,
    Pending,
    Cancelled,
}

impl PipelineStatus {
    /// Returns the CSS class for this status.
    pub fn css_class(self) -> &'static str {
        match self {
            Self::Success => "status-badge status-success",
            Self::Failure => "status-badge status-failure",
            Self::Pending => "status-badge status-pending",
            Self::Cancelled => "status-badge status-cancelled",
        }
    }

    /// Returns the display name for this status.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failure => "failure",
            Self::Pending => "pending",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Renders a status badge with appropriate styling.
fn status_badge(status: PipelineStatus) -> Markup {
    html! { span class=(status.css_class()) { (status.as_str()) } }
}

bitflags! {
    /// Triggers that cause a pipeline to run.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Triggers: u8 {
        /// Runs on pushes to the main branch.
        const MAIN = 0b0001;
        /// Runs on pull requests.
        const PR = 0b0010;
        /// Runs on a schedule.
        const SCHEDULED = 0b0100;
        /// Can be triggered manually.
        const MANUAL = 0b1000;
    }
}

/// Formats a duration for display.
fn format_duration(duration: Option<SignedDuration>) -> String {
    match duration {
        None => "—".into(),
        Some(d) => {
            let total_secs = d.as_secs();
            let mins = total_secs / 60;
            let secs = total_secs % 60;
            format!("{}m {:02}s", mins, secs)
        }
    }
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

fn repo_deps_url(owner: &str, repo: &str) -> Url {
    let mut url = Url::parse("relative:/").expect("valid base");
    url.path_segments_mut()
        .expect("cannot be base")
        .push("dashboard")
        .push("repo")
        .push(owner)
        .push(repo)
        .push("deps");
    url
}

/// Generates the Alpine.js `x-data` attribute value for an expandable component.
fn expandable_directive(url: &Url, element_id: &str) -> String {
    format!("expandable('{}', '{}')", url.path(), element_id)
}

/// Repository pipeline summary for display.
pub struct RepoSummary {
    pub owner: String,
    pub repo: String,
    pub description: String,
    pub success_rate: u8,
    pub last_status: PipelineStatus,
    pub triggers: Triggers,
    pub deps_total: u32,
    pub deps_up_to_date: u32,
    pub pipelines: Vec<PipelineSummary>,
    pub dependencies: Vec<Dependency>,
}

impl RepoSummary {
    /// Whether the pipeline runs on pushes to main.
    pub fn runs_on_main(&self) -> bool {
        self.triggers.contains(Triggers::MAIN)
    }

    /// Whether the pipeline runs on pull requests.
    pub fn runs_on_pr(&self) -> bool {
        self.triggers.contains(Triggers::PR)
    }

    /// Whether the pipeline runs on a schedule.
    pub fn runs_scheduled(&self) -> bool {
        self.triggers.contains(Triggers::SCHEDULED)
    }

    /// Whether the pipeline can be triggered manually.
    pub fn runs_manual(&self) -> bool {
        self.triggers.contains(Triggers::MANUAL)
    }
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

/// Individual pipeline/workflow summary.
pub struct PipelineSummary {
    pub name: String,
    pub status: PipelineStatus,
    pub run_time: Option<SignedDuration>,
    pub github_url: Url,
}

/// Dependency status.
#[derive(Clone)]
pub struct Dependency {
    pub name: String,
    pub current_version: String,
    pub latest_version: String,
    pub is_outdated: bool,
}

fn rate_class(rate: u8) -> &'static str {
    match rate {
        90..=100 => "rate-excellent",
        75..=89 => "rate-good",
        50..=74 => "rate-warning",
        _ => "rate-critical",
    }
}

fn deps_rate(up_to_date: u32, total: u32) -> u8 {
    if total == 0 {
        return 100;
    }
    ((up_to_date as f64 / total as f64) * 100.0).round() as u8
}

fn repo_row(repo: &RepoSummary) -> Markup {
    let dep_rate = deps_rate(repo.deps_up_to_date, repo.deps_total);
    let detail_id = format!("detail-{}-{}", repo.owner, repo.repo);
    let expand_url = repo_expand_url(&repo.owner, &repo.repo);
    html! {
        tbody x-data=(expandable_directive(&expand_url, &detail_id)) {
            tr class="repo-row"
               x-bind:class="{ 'expanded': expanded }"
               x-on:click="toggle()"
            {
                td class="expand-cell" {
                    span class="expand-arrow" { (PreEscaped("▶")) }
                }
                td class="repo-name" { (repo.full_name()) }
                td class="success-rate" {
                    span class=(rate_class(repo.success_rate)) {
                        (repo.success_rate) "%"
                    }
                }
                td class="last-status" { (status_badge(repo.last_status)) }
                td class="trigger-checks" {
                    (check_icon(repo.runs_on_main()))
                }
                td class="trigger-checks" {
                    (check_icon(repo.runs_on_pr()))
                }
                td class="trigger-checks" {
                    (check_icon(repo.runs_scheduled()))
                }
                td class="trigger-checks" {
                    (check_icon(repo.runs_manual()))
                }
                td class="deps-total" { (repo.deps_total) }
                td class="deps-current" {
                    span class=(rate_class(dep_rate)) {
                        (repo.deps_up_to_date)
                    }
                }
            }
            tr class="repo-detail-row" x-show="expanded" x-cloak {
                td colspan="10" class="repo-detail-cell" {
                    div id=(detail_id) {}
                }
            }
        }
    }
}

fn get_sorted_dependencies(deps: &[Dependency]) -> Vec<Dependency> {
    let mut sorted_deps = deps.to_vec();
    sorted_deps.sort_by(|a, b| b.is_outdated.cmp(&a.is_outdated));
    sorted_deps
}

fn render_repo_detail(repo: &RepoSummary) -> Markup {
    let deps = &repo.dependencies;

    html! {
        div class="repo-detail" {
            div class="repo-info" {
                a href=(repo.github_url()) target="_blank" class="repo-github-link" {
                    (PreEscaped("🔗")) " " (repo.github_url())
                }
                p class="repo-description" { (repo.description.as_str()) }
            }

            @if !repo.pipelines.is_empty() {
                div class="pipelines-section" {
                    h3 { "Pipelines" }
                    table class="pipelines-table" {
                        thead {
                            tr {
                                th { "Name" }
                                th { "Status" }
                                th { "Duration" }
                                th { "Link" }
                            }
                        }
                        tbody {
                            @for pipeline in &repo.pipelines {
                                tr {
                                    td { (pipeline.name.as_str()) }
                                    td { (status_badge(pipeline.status)) }
                                    td class="pipeline-time" { (format_duration(pipeline.run_time)) }
                                    td {
                                        a href=(pipeline.github_url) target="_blank" class="pipeline-link" {
                                            "View"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            @if !deps.is_empty() {
                @let deps_list_id = format!("deps-list-{}-{}", repo.owner, repo.repo);
                @let deps_url = repo_deps_url(&repo.owner, &repo.repo);
                div class="deps-section" x-data=(expandable_directive(&deps_url, &deps_list_id)) {
                    h3
                        class="deps-header"
                        x-bind:class="{ 'expanded': expanded }"
                        x-on:click="toggle()"
                    {
                        span class="deps-expand-arrow" { (PreEscaped("▶")) }
                        " Dependencies (" (deps.len()) ")"
                    }
                    div x-show="expanded" x-cloak id=(deps_list_id) class="deps-list" {}
                }
            }
        }
    }
}

fn render_deps_list(repo: &RepoSummary) -> Markup {
    let deps = get_sorted_dependencies(&repo.dependencies);

    html! {
        table class="deps-table" {
            thead {
                tr {
                    th { "Package" }
                    th { "Current" }
                    th { "Latest" }
                    th { "Status" }
                }
            }
            tbody {
                @for dep in &deps {
                    tr class=(if dep.is_outdated { "dep-outdated" } else { "dep-current" }) {
                        td { (dep.name.as_str()) }
                        td class="version" { (dep.current_version.as_str()) }
                        td class="version" { (dep.latest_version.as_str()) }
                        td {
                            @if dep.is_outdated {
                                span class="dep-status outdated" { "Outdated" }
                            } @else {
                                span class="dep-status current" { "Current" }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Returns the expanded detail HTML for a repository row.
pub async fn repo_expand(Path((owner, repo)): Path<(String, String)>) -> impl IntoResponse {
    match find_repo(&owner, &repo) {
        Some(r) => render_repo_detail(&r),
        None => html! { div class="error" { "Repository not found" } },
    }
}

/// Returns the dependencies list HTML for a repository.
pub async fn repo_deps(Path((owner, repo)): Path<(String, String)>) -> impl IntoResponse {
    match find_repo(&owner, &repo) {
        Some(r) => render_deps_list(&r),
        None => html! { div class="error" { "Repository not found" } },
    }
}

/// Renders the main dashboard page.
pub async fn dashboard_page() -> impl IntoResponse {
    let repos = mock_repos();

    let body = html! {
        div class="dashboard-container" {
            header class="dashboard-header" {
                h1 { "Hubdash" }
                a href="/auth/signout" class="sign-out-link" { "Sign out" }
            }
            main class="dashboard-main" {
                table class="repo-table" {
                    thead {
                        tr {
                            th class="expand-header" { }
                            th { "Repository" }
                            th { "Success Rate" }
                            th { "Last Run" }
                            th title="Runs on main branch" { "Main" }
                            th title="Runs on pull requests" { "PR" }
                            th title="Scheduled runs" { "Sched" }
                            th title="Manual trigger" { "Manual" }
                            th title="Total dependencies" { "Deps" }
                            th title="Dependencies up to date" { "Current" }
                        }
                    }
                    @for repo in &repos {
                        (repo_row(repo))
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
