//! Tests for dashboard partials and the dashboard page.
//!
//! Each test pre-seeds a valid session via [`run_authed_sim`], which passes
//! the ready-to-use `Cookie` header value to the client closure.

use super::test_utils::{get_with_cookie, run_authed_sim};

// ── repo_expand ──────────────────────────────────────────────────────────────

/// `repo_expand` for a known repo returns detail HTML with pipeline info.
#[test]
fn repo_expand_returns_detail_html() {
    run_authed_sim(|cookie| async move {
        let (status, body) =
            get_with_cookie("/dashboard/repo/example/hubdash/expand", Some(&cookie)).await;
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
}

/// `repo_expand` for an unknown repo returns an error message.
#[test]
fn repo_expand_unknown_repo() {
    run_authed_sim(|cookie| async move {
        let (status, body) =
            get_with_cookie("/dashboard/repo/nobody/norepo/expand", Some(&cookie)).await;
        assert_eq!(status, http::StatusCode::OK, "body: {body}");
        assert!(
            body.contains("not found"),
            "expected 'not found' message, got: {body}"
        );
        Ok(())
    });
}

// ── repo_deps ────────────────────────────────────────────────────────────────

/// `repo_deps` for a repo with dependencies returns a table with package names.
#[test]
fn repo_deps_returns_deps_table() {
    run_authed_sim(|cookie| async move {
        let (status, body) =
            get_with_cookie("/dashboard/repo/example/hubdash/deps", Some(&cookie)).await;
        assert_eq!(status, http::StatusCode::OK, "body: {body}");
        assert!(
            body.contains("deps-table"),
            "expected deps-table class, got: {body}"
        );
        assert!(body.contains("axum"), "expected axum dep, got: {body}");
        assert!(body.contains("tokio"), "expected tokio dep, got: {body}");
        Ok(())
    });
}

/// Outdated packages get `dep-outdated` and current ones get `dep-current`.
#[test]
fn repo_deps_marks_outdated_and_current() {
    run_authed_sim(|cookie| async move {
        let (status, body) =
            get_with_cookie("/dashboard/repo/example/hubdash/deps", Some(&cookie)).await;
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
}

/// A repo with no dependencies still returns 200 with a (empty) table.
#[test]
fn repo_deps_empty_deps() {
    run_authed_sim(|cookie| async move {
        let (status, body) =
            get_with_cookie("/dashboard/repo/example/legacy-service/deps", Some(&cookie)).await;
        assert_eq!(status, http::StatusCode::OK, "body: {body}");
        assert!(
            body.contains("deps-table"),
            "expected deps-table class, got: {body}"
        );
        Ok(())
    });
}

/// `repo_deps` for an unknown repo returns an error message.
#[test]
fn repo_deps_unknown_repo() {
    run_authed_sim(|cookie| async move {
        let (status, body) =
            get_with_cookie("/dashboard/repo/nobody/norepo/deps", Some(&cookie)).await;
        assert_eq!(status, http::StatusCode::OK, "body: {body}");
        assert!(
            body.contains("not found"),
            "expected 'not found' message, got: {body}"
        );
        Ok(())
    });
}

// ── dashboard page ───────────────────────────────────────────────────────────

/// The dashboard page lists all mock repositories.
#[test]
fn dashboard_page_lists_repos() {
    run_authed_sim(|cookie| async move {
        let (status, body) = get_with_cookie("/dashboard", Some(&cookie)).await;
        assert_eq!(status, http::StatusCode::OK, "body: {body}");
        assert!(
            body.contains("repo-table"),
            "expected repo-table, got: {body}"
        );
        assert!(body.contains("hubdash"), "expected hubdash repo row");
        assert!(
            body.contains("api-gateway"),
            "expected api-gateway repo row"
        );
        assert!(
            body.contains("frontend-app"),
            "expected frontend-app repo row"
        );
        Ok(())
    });
}
