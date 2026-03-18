/// End-to-end tests that exercise the full request lifecycle against a
/// temporary in-process server backed by a real (test) database.
use axum::http::StatusCode;
use axum_test::TestServer;

use myapp::http_server::build_router;
use myapp::AppState;

/// Helper: spin up a fresh test server with an empty test database.
async fn test_app() -> TestServer {
    let state = AppState::new_test().await;
    let router = build_router(state);
    TestServer::new(router).unwrap()
}

/// Helper: obtain a valid JWT for a freshly created test user.
async fn create_and_login(server: &TestServer) -> String {
    let register_body = serde_json::json!({
        "email": "e2e@example.com",
        "username": "e2e_user",
        "password": "SecurePass1"
    });
    let res = server.post("/api/v1/users").json(&register_body).await;
    res.assert_status(StatusCode::CREATED);

    let login_body = serde_json::json!({
        "email": "e2e@example.com",
        "password": "SecurePass1"
    });
    let res = server.post("/auth/login").json(&login_body).await;
    res.assert_status(StatusCode::OK);
    let body: serde_json::Value = res.json();
    body["token"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn user_create_and_fetch_roundtrip() {
    let server = test_app().await;
    let token = create_and_login(&server).await;

    let res = server
        .get("/api/v1/users")
        .add_header("Authorization", format!("Bearer {token}"))
        .await;
    res.assert_status(StatusCode::OK);

    let body: serde_json::Value = res.json();
    assert!(body["data"].as_array().unwrap().len() >= 1);
}

#[tokio::test]
async fn post_lifecycle() {
    let server = test_app().await;
    let token = create_and_login(&server).await;

    // Create
    let create_body = serde_json::json!({
        "title": "E2E Test Post",
        "body": "This post was created by an end-to-end test.",
        "tags": ["test", "e2e"],
        "published": true
    });
    let res = server
        .post("/api/v1/posts")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&create_body)
        .await;
    res.assert_status(StatusCode::CREATED);
    let post: serde_json::Value = res.json();
    let post_id = post["id"].as_str().unwrap();

    // Fetch
    let res = server
        .get(&format!("/api/v1/posts/{post_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;
    res.assert_status(StatusCode::OK);

    // Delete
    let res = server
        .delete(&format!("/api/v1/posts/{post_id}"))
        .add_header("Authorization", format!("Bearer {token}"))
        .await;
    res.assert_status(StatusCode::NO_CONTENT);
}
