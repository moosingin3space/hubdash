//! Dashboard page showing repository CI/CD health.

use axum::Extension;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use bitflags::bitflags;
use jiff::SignedDuration;
use maud::{Markup, PreEscaped, html};
use tracing::error;
use url::Url;

use crate::github::oauth;
use crate::github::{Repository, WorkflowRun};
use crate::layout::{base_layout, check_icon};
use crate::session::Session;
use crate::{AppState, Platform};

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

fn repo_pipelines_url(owner: &str, repo: &str) -> Url {
    let mut url = Url::parse("relative:/").expect("valid base");
    url.path_segments_mut()
        .expect("cannot be base")
        .push("dashboard")
        .push("repo")
        .push(owner)
        .push(repo)
        .push("pipelines");
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
    pub pipelines: Vec<PipelineSummary>,
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

fn rate_class(rate: u8) -> &'static str {
    match rate {
        90..=100 => "rate-excellent",
        75..=89 => "rate-good",
        50..=74 => "rate-warning",
        _ => "rate-critical",
    }
}

fn repo_summary_url(owner: &str, repo: &str) -> Url {
    let mut url = Url::parse("relative:/").expect("valid base");
    url.path_segments_mut()
        .expect("cannot be base")
        .push("dashboard")
        .push("repo")
        .push(owner)
        .push(repo)
        .push("summary");
    url
}

/// Renders the summary cells (success rate through manual trigger) for a repo row.
fn render_summary_cells(repo: &RepoSummary) -> Markup {
    html! {
        td class="success-rate" {
            @if repo.pipelines.is_empty() {
                "—"
            } @else {
                span class=(rate_class(repo.success_rate)) {
                    (repo.success_rate) "%"
                }
            }
        }
        td class="last-status" {
            @if repo.pipelines.is_empty() {
                span class="status-badge status-unknown" { "N/A" }
            } @else {
                (status_badge(repo.last_status))
            }
        }
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
    }
}

fn repo_row(repo: &RepoSummary, lazy: bool) -> Markup {
    let detail_id = format!("detail-{}-{}", repo.owner, repo.repo);
    let summary_id = format!("summary-{}-{}", repo.owner, repo.repo);
    let expand_url = repo_expand_url(&repo.owner, &repo.repo);
    let summary_url = repo_summary_url(&repo.owner, &repo.repo);
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
                @if lazy {
                    td class="success-rate" colspan="6"
                       id=(summary_id)
                       hx-get=(summary_url.path())
                       hx-trigger="load"
                       hx-swap="outerHTML"
                    {
                        span class="loading-indicator" { "Loading…" }
                    }
                } @else {
                    (render_summary_cells(repo))
                }
            }
            tr class="repo-detail-row" x-show="expanded" x-cloak {
                td colspan="8" class="repo-detail-cell" {
                    div id=(detail_id) {}
                }
            }
        }
    }
}

fn render_repo_detail(repo: &RepoSummary) -> Markup {
    let pipelines_id = format!("pipelines-{}-{}", repo.owner, repo.repo);
    let pipelines_url = repo_pipelines_url(&repo.owner, &repo.repo);

    html! {
        div class="repo-detail" {
            div class="repo-info" {
                a href=(repo.github_url()) target="_blank" class="repo-github-link" {
                    (PreEscaped("🔗")) " " (repo.github_url())
                }
                p class="repo-description" { (repo.description.as_str()) }
            }

            div class="pipelines-section" {
                h3 { "Pipelines" }
                div id=(pipelines_id)
                    hx-get=(pipelines_url.path())
                    hx-trigger="load"
                    hx-swap="innerHTML"
                {
                    span class="loading-indicator" { "Loading…" }
                }
            }
        }
    }
}

/// Renders the pipelines table for a set of pipeline summaries.
fn render_pipelines_table(pipelines: &[PipelineSummary]) -> Markup {
    if pipelines.is_empty() {
        return html! {
            p class="no-pipelines" { "No workflow runs found." }
        };
    }
    html! {
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
                @for pipeline in pipelines {
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

/// Converts a workflow run's conclusion into a `PipelineStatus`.
fn pipeline_status_from_run(run: &WorkflowRun) -> PipelineStatus {
    match run.status.as_str() {
        "completed" => match run.conclusion.as_deref() {
            Some("success") => PipelineStatus::Success,
            Some("failure") | Some("timed_out") => PipelineStatus::Failure,
            Some("cancelled") | Some("skipped") => PipelineStatus::Cancelled,
            _ => PipelineStatus::Pending,
        },
        _ => PipelineStatus::Pending,
    }
}

/// Computes the set of trigger flags observed across workflow runs.
fn triggers_from_runs(runs: &[WorkflowRun]) -> Triggers {
    let mut triggers = Triggers::empty();
    for run in runs {
        match run.event.as_str() {
            "push" => triggers |= Triggers::MAIN,
            "pull_request" | "pull_request_target" => triggers |= Triggers::PR,
            "schedule" => triggers |= Triggers::SCHEDULED,
            "workflow_dispatch" => triggers |= Triggers::MANUAL,
            _ => {}
        }
    }
    triggers
}

/// Computes the success rate (0–100) from completed workflow runs.
fn success_rate_from_runs(runs: &[WorkflowRun]) -> u8 {
    let completed: Vec<_> = runs.iter().filter(|r| r.status == "completed").collect();
    if completed.is_empty() {
        return 0;
    }
    let successes = completed
        .iter()
        .filter(|r| r.conclusion.as_deref() == Some("success"))
        .count();
    ((successes as f64 / completed.len() as f64) * 100.0).round() as u8
}

/// Computes the run duration from `run_started_at` to `updated_at`.
fn run_duration(run: &WorkflowRun) -> Option<SignedDuration> {
    let started: jiff::Timestamp = run.run_started_at.as_deref()?.parse().ok()?;
    let updated: jiff::Timestamp = run.updated_at.parse().ok()?;
    Some(updated.duration_since(started))
}

/// Converts workflow runs into per-workflow `PipelineSummary` entries.
///
/// Groups runs by workflow name, taking only the most recent run per workflow.
fn pipelines_from_runs(runs: &[WorkflowRun]) -> Vec<PipelineSummary> {
    let mut seen = std::collections::HashSet::new();
    let mut pipelines = Vec::new();
    for run in runs {
        let name = run.name.clone().unwrap_or_else(|| "unnamed".into());
        if !seen.insert(name.clone()) {
            continue;
        }
        pipelines.push(PipelineSummary {
            name,
            status: pipeline_status_from_run(run),
            run_time: run_duration(run),
            github_url: Url::parse(&run.html_url).expect("valid GitHub URL"),
        });
    }
    pipelines
}

/// Converts a GitHub API repository and its workflow runs into a `RepoSummary`.
fn repo_summary_from(repo: &Repository, runs: &[WorkflowRun]) -> RepoSummary {
    let last_status = runs
        .first()
        .map(pipeline_status_from_run)
        .unwrap_or(PipelineStatus::Pending);

    RepoSummary {
        owner: repo.owner.login.clone(),
        repo: repo.name.clone(),
        description: repo.description.clone().unwrap_or_default(),
        success_rate: success_rate_from_runs(runs),
        last_status,
        triggers: triggers_from_runs(runs),
        pipelines: pipelines_from_runs(runs),
    }
}

/// Returns the expanded detail HTML for a repository row.
///
/// Renders the detail shell immediately; the pipelines table within it
/// is lazy-loaded via a separate HTMX request.
pub async fn repo_expand(Path((owner, repo)): Path<(String, String)>) -> impl IntoResponse {
    let summary = RepoSummary {
        owner,
        repo,
        description: String::new(),
        success_rate: 0,
        last_status: PipelineStatus::Pending,
        triggers: Triggers::empty(),
        pipelines: vec![],
    };
    render_repo_detail(&summary)
}

/// Returns the pipelines table HTML for a repository (lazy-loaded via HTMX).
pub async fn repo_pipelines<P: Platform>(
    State(state): State<AppState<P>>,
    Extension(session): Extension<Session>,
    Path((owner, repo)): Path<(String, String)>,
) -> impl IntoResponse {
    let http_client = state.plat.create_http_client();
    let runs =
        match oauth::list_workflow_runs(&http_client, &session.access_token, &owner, &repo, 20)
            .await
        {
            Ok(response) => response.workflow_runs,
            Err(e) => {
                error!("Failed to fetch workflow runs for {owner}/{repo}: {e:?}");
                vec![]
            }
        };

    render_pipelines_table(&pipelines_from_runs(&runs))
}

/// Returns the summary cells HTML for a repository row (lazy-loaded via HTMX).
pub async fn repo_summary<P: Platform>(
    State(state): State<AppState<P>>,
    Extension(session): Extension<Session>,
    Path((owner, repo)): Path<(String, String)>,
) -> impl IntoResponse {
    let http_client = state.plat.create_http_client();
    let runs =
        match oauth::list_workflow_runs(&http_client, &session.access_token, &owner, &repo, 20)
            .await
        {
            Ok(response) => response.workflow_runs,
            Err(e) => {
                error!("Failed to fetch workflow runs for {owner}/{repo}: {e:?}");
                vec![]
            }
        };

    let pipelines = pipelines_from_runs(&runs);
    if pipelines.is_empty() {
        let mut headers = HeaderMap::new();
        headers.insert("HX-Reswap", "delete".parse().expect("valid header value"));
        headers.insert(
            "HX-Retarget",
            format!("#tbody-{owner}-{repo}")
                .parse()
                .expect("valid header value"),
        );
        return (headers, html! {}).into_response();
    }

    let summary = RepoSummary {
        owner,
        repo,
        description: String::new(),
        success_rate: success_rate_from_runs(&runs),
        last_status: runs
            .first()
            .map(pipeline_status_from_run)
            .unwrap_or(PipelineStatus::Pending),
        triggers: triggers_from_runs(&runs),
        pipelines,
    };
    render_summary_cells(&summary).into_response()
}

fn repo_table_header() -> Markup {
    html! {
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
                        (repo_row(repo, true))
                    }
                }
            }
        }
    }
}

/// Renders the main dashboard page.
///
/// Only fetches the repo list; per-repo workflow data is lazy-loaded via HTMX.
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
    for repo in api_repos
        .iter()
        .map(|api_repo| repo_summary_from(api_repo, &[]))
    {
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
