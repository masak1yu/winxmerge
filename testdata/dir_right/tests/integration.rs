use axum::http::StatusCode;
use axum_test::TestServer;

use myapp::http_server::build_router;
use myapp::AppState;

async fn test_app() -> TestServer {
    let state = AppState::new_test().await;
    let router = build_router(state);
    TestServer::new(router).unwrap()
}

#[tokio::test]
async fn health_check_returns_ok() {
    let server = test_app().await;
    let res = server.get("/health").await;
    res.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn health_check_includes_version() {
    let server = test_app().await;
    let res = server.get("/health").await;
    let body: serde_json::Value = res.json();
    assert!(body["version"].is_string());
}

#[tokio::test]
async fn list_users_requires_auth() {
    let server = test_app().await;
    let res = server.get("/api/v1/users").await;
    res.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn v2_users_endpoint_exists() {
    let server = test_app().await;
    let res = server.get("/api/v2/users").await;
    // Should return 401, not 404 — the route exists.
    res.assert_status(StatusCode::UNAUTHORIZED);
}
