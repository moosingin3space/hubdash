//! Tests for dashboard partials and the dashboard page.
//!
//! Starts the hubdash server with a pre-seeded session, then exercises
//! authenticated routes and inspects the rendered HTML.

mod test_utils;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use hubdash::session::{GitHubUser, Session, SessionId, SessionStore as _};
use hubdash::{AppState, GitHubOAuthConfig, InMemorySessionStore, create_router_with_state};
use test_utils::connector::sim_listen;
use test_utils::platform::SimPlatform;

const APP_PORT: u16 = 3000;
const APP_HOST: &str = "hubdash";

/// Registers a hubdash server host with a single pre-seeded session.
///
/// Returns the `SessionId` so the test client can craft the correct cookie.
fn register_app_with_session(
    sim: &mut turmoil::Sim<'_>,
    session_id: SessionId,
) {
    sim.host(APP_HOST, move || {
        let session_id = session_id.clone();
        async move {
            let sessions = InMemorySessionStore::new();
            sessions
                .put(Session {
                    id: session_id.clone(),
                    user: GitHubUser {
                        id: 1,
                        login: "test-user".into(),
                        name: Some("Test User".into()),
                        avatar_url: None,
                    },
                    access_token: "fake-access-token".into(),
                })
                .unwrap();

            let state = AppState::<SimPlatform> {
                sessions,
                oauth: GitHubOAuthConfig {
                    client_id: "test-client-id".into(),
                    client_secret: "test-client-secret".into(),
                    ..Default::default()
                },
                plat: SimPlatform,
            };

            let router = create_router_with_state(state);
            let listener = sim_listen(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                APP_PORT,
            ))
            .await
            .unwrap();
            axum::serve(listener, router).await.unwrap();
            Ok(())
        }
    });
}

/// Returns the cookie header value for the given session ID.
fn session_cookie(id: &SessionId) -> String {
    format!("hubdash_session={}", id.as_str())
}

// ── repo_expand ──────────────────────────────────────────────────────────────

/// `repo_expand` for a known repo returns detail HTML with pipeline info.
#[test]
fn repo_expand_returns_detail_html() {
    let sid = SessionId::new();
    let mut sim = turmoil::Builder::new().build();
    register_app_with_session(&mut sim, sid.clone());

    let cookie = session_cookie(&sid);
    sim.client("client", async move {
        let (status, body) =
            test_utils::get_with_cookie("/dashboard/repo/example/hubdash/expand", Some(&cookie))
                .await;
        assert_eq!(status, http::StatusCode::OK, "body: {body}");
        assert!(
            body.contains("repo-detail"),
            "expected repo-detail markup, got: {body}"
        );
        assert!(
            body.contains("Pipelines"),
            "expected Pipelines section, got: {body}"
        );
        Ok(())
    });

    sim.run().unwrap();
}

/// `repo_expand` for an unknown repo returns an error message.
#[test]
fn repo_expand_unknown_repo() {
    let sid = SessionId::new();
    let mut sim = turmoil::Builder::new().build();
    register_app_with_session(&mut sim, sid.clone());

    let cookie = session_cookie(&sid);
    sim.client("client", async move {
        let (status, body) =
            test_utils::get_with_cookie("/dashboard/repo/nobody/norepo/expand", Some(&cookie))
                .await;
        assert_eq!(status, http::StatusCode::OK, "body: {body}");
        assert!(
            body.contains("not found"),
            "expected 'not found' message, got: {body}"
        );
        Ok(())
    });

    sim.run().unwrap();
}

// ── repo_deps ────────────────────────────────────────────────────────────────

/// `repo_deps` for a repo with dependencies returns a table with package names.
#[test]
fn repo_deps_returns_deps_table() {
    let sid = SessionId::new();
    let mut sim = turmoil::Builder::new().build();
    register_app_with_session(&mut sim, sid.clone());

    let cookie = session_cookie(&sid);
    sim.client("client", async move {
        let (status, body) =
            test_utils::get_with_cookie("/dashboard/repo/example/hubdash/deps", Some(&cookie))
                .await;
        assert_eq!(status, http::StatusCode::OK, "body: {body}");
        assert!(
            body.contains("deps-table"),
            "expected deps-table class, got: {body}"
        );
        assert!(body.contains("axum"), "expected axum dep, got: {body}");
        assert!(body.contains("tokio"), "expected tokio dep, got: {body}");
        Ok(())
    });

    sim.run().unwrap();
}

/// Outdated packages get `dep-outdated` and current ones get `dep-current`.
#[test]
fn repo_deps_marks_outdated_and_current() {
    let sid = SessionId::new();
    let mut sim = turmoil::Builder::new().build();
    register_app_with_session(&mut sim, sid.clone());

    let cookie = session_cookie(&sid);
    sim.client("client", async move {
        let (status, body) =
            test_utils::get_with_cookie("/dashboard/repo/example/hubdash/deps", Some(&cookie))
                .await;
        assert_eq!(status, http::StatusCode::OK, "body: {body}");
        assert!(
            body.contains("dep-outdated"),
            "expected dep-outdated class, got: {body}"
        );
        assert!(
            body.contains("dep-current"),
            "expected dep-current class, got: {body}"
        );
        Ok(())
    });

    sim.run().unwrap();
}

/// `repo_deps` for a repo with no deps (frontend-app has 0 outdated, but still
/// has deps in mock data — use legacy-service which has no dependencies at all).
/// The endpoint should still return 200 with an empty table body.
#[test]
fn repo_deps_empty_deps() {
    let sid = SessionId::new();
    let mut sim = turmoil::Builder::new().build();
    register_app_with_session(&mut sim, sid.clone());

    let cookie = session_cookie(&sid);
    sim.client("client", async move {
        let (status, body) =
            test_utils::get_with_cookie(
                "/dashboard/repo/example/legacy-service/deps",
                Some(&cookie),
            )
            .await;
        assert_eq!(status, http::StatusCode::OK, "body: {body}");
        // The deps table is still rendered, just with an empty tbody.
        assert!(
            body.contains("deps-table"),
            "expected deps-table class, got: {body}"
        );
        Ok(())
    });

    sim.run().unwrap();
}

/// `repo_deps` for an unknown repo returns an error message.
#[test]
fn repo_deps_unknown_repo() {
    let sid = SessionId::new();
    let mut sim = turmoil::Builder::new().build();
    register_app_with_session(&mut sim, sid.clone());

    let cookie = session_cookie(&sid);
    sim.client("client", async move {
        let (status, body) =
            test_utils::get_with_cookie("/dashboard/repo/nobody/norepo/deps", Some(&cookie))
                .await;
        assert_eq!(status, http::StatusCode::OK, "body: {body}");
        assert!(
            body.contains("not found"),
            "expected 'not found' message, got: {body}"
        );
        Ok(())
    });

    sim.run().unwrap();
}

// ── dashboard page ───────────────────────────────────────────────────────────

/// The dashboard page lists all mock repositories.
#[test]
fn dashboard_page_lists_repos() {
    let sid = SessionId::new();
    let mut sim = turmoil::Builder::new().build();
    register_app_with_session(&mut sim, sid.clone());

    let cookie = session_cookie(&sid);
    sim.client("client", async move {
        let (status, body) =
            test_utils::get_with_cookie("/dashboard", Some(&cookie)).await;
        assert_eq!(status, http::StatusCode::OK, "body: {body}");
        assert!(
            body.contains("repo-table"),
            "expected repo-table, got: {body}"
        );
        assert!(body.contains("hubdash"), "expected hubdash repo row");
        assert!(body.contains("api-gateway"), "expected api-gateway repo row");
        assert!(body.contains("frontend-app"), "expected frontend-app repo row");
        Ok(())
    });

    sim.run().unwrap();
}
