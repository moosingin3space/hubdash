//! Mock data for testing and development.

use jiff::SignedDuration;
use url::Url;

use crate::dashboard::{Dependency, PipelineStatus, PipelineSummary, RepoSummary, Triggers};

fn github_actions_url(owner: &str, repo: &str, run_id: u64) -> Url {
    let mut url = Url::parse("https://github.com").expect("valid base URL");
    url.path_segments_mut()
        .expect("cannot be base")
        .push(owner)
        .push(repo)
        .push("actions")
        .push("runs")
        .push(&run_id.to_string());
    url
}

/// Returns mock repository data for development and testing.
pub fn mock_repos() -> Vec<RepoSummary> {
    vec![
        RepoSummary {
            owner: "example".into(),
            repo: "hubdash".into(),
            description: "A GitHub dashboard for monitoring CI/CD pipeline health".into(),
            success_rate: 95,
            last_status: PipelineStatus::Success,
            triggers: Triggers::MAIN | Triggers::PR | Triggers::SCHEDULED,
            deps_total: 42,
            deps_up_to_date: 40,
            pipelines: vec![
                PipelineSummary {
                    name: "CI".into(),
                    status: PipelineStatus::Success,
                    run_time: Some(SignedDuration::new(154, 0)),
                    github_url: github_actions_url("example", "hubdash", 123),
                },
                PipelineSummary {
                    name: "Deploy".into(),
                    status: PipelineStatus::Success,
                    run_time: Some(SignedDuration::new(312, 0)),
                    github_url: github_actions_url("example", "hubdash", 124),
                },
            ],
            dependencies: vec![
                Dependency {
                    name: "axum".into(),
                    current_version: "0.7.5".into(),
                    latest_version: "0.8.0".into(),
                    is_outdated: true,
                },
                Dependency {
                    name: "tokio".into(),
                    current_version: "1.40.0".into(),
                    latest_version: "1.40.0".into(),
                    is_outdated: false,
                },
                Dependency {
                    name: "maud".into(),
                    current_version: "0.26.0".into(),
                    latest_version: "0.27.0".into(),
                    is_outdated: true,
                },
            ],
        },
        RepoSummary {
            owner: "example".into(),
            repo: "api-gateway".into(),
            description: "API gateway service for routing and authentication".into(),
            success_rate: 87,
            last_status: PipelineStatus::Failure,
            triggers: Triggers::MAIN | Triggers::PR | Triggers::MANUAL,
            deps_total: 78,
            deps_up_to_date: 65,
            pipelines: vec![PipelineSummary {
                name: "Build".into(),
                status: PipelineStatus::Failure,
                run_time: Some(SignedDuration::new(105, 0)),
                github_url: github_actions_url("example", "api-gateway", 456),
            }],
            dependencies: vec![Dependency {
                name: "express".into(),
                current_version: "4.18.0".into(),
                latest_version: "4.21.0".into(),
                is_outdated: true,
            }],
        },
        RepoSummary {
            owner: "example".into(),
            repo: "frontend-app".into(),
            description: "React frontend application".into(),
            success_rate: 100,
            last_status: PipelineStatus::Success,
            triggers: Triggers::MAIN | Triggers::PR | Triggers::SCHEDULED | Triggers::MANUAL,
            deps_total: 156,
            deps_up_to_date: 156,
            pipelines: vec![PipelineSummary {
                name: "Test".into(),
                status: PipelineStatus::Success,
                run_time: Some(SignedDuration::new(202, 0)),
                github_url: github_actions_url("example", "frontend-app", 789),
            }],
            dependencies: vec![],
        },
        RepoSummary {
            owner: "example".into(),
            repo: "data-pipeline".into(),
            description: "ETL data processing pipeline".into(),
            success_rate: 72,
            last_status: PipelineStatus::Pending,
            triggers: Triggers::MAIN | Triggers::SCHEDULED,
            deps_total: 34,
            deps_up_to_date: 22,
            pipelines: vec![PipelineSummary {
                name: "ETL".into(),
                status: PipelineStatus::Pending,
                run_time: None,
                github_url: github_actions_url("example", "data-pipeline", 101),
            }],
            dependencies: vec![Dependency {
                name: "pandas".into(),
                current_version: "1.5.0".into(),
                latest_version: "2.2.0".into(),
                is_outdated: true,
            }],
        },
        RepoSummary {
            owner: "example".into(),
            repo: "legacy-service".into(),
            description: "Legacy monolith service (deprecated)".into(),
            success_rate: 45,
            last_status: PipelineStatus::Cancelled,
            triggers: Triggers::MAIN,
            deps_total: 89,
            deps_up_to_date: 31,
            pipelines: vec![],
            dependencies: vec![],
        },
    ]
}

/// Finds a repository by owner and name from the mock data.
pub fn find_repo(owner: &str, repo: &str) -> Option<RepoSummary> {
    mock_repos()
        .into_iter()
        .find(|r| r.owner == owner && r.repo == repo)
}
