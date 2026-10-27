pub mod auth;
pub mod db;
pub mod handlers;
pub mod models;
pub mod response;

use std::println;

use axum::{
    Router, middleware,
    routing::{get, post},
};
use sqlx::{Pool, Sqlite};

use crate::{
    auth::{LoginRateLimiter, initialize_dummy_password_hash, require_access_token},
    handlers::{hello_world, login, logout, me, refresh, register},
};

#[derive(Clone)]
pub struct AppState {
    pool: Pool<Sqlite>,
    access_token_secret: String,
    refresh_token_secret: String,
    login_rate_limiter: LoginRateLimiter,
}

fn build_app(state: AppState) -> Router {
    Router::new()
        .route("/", get(hello_world))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/refresh", post(refresh))
        .route(
            "/api/me",
            get(me).route_layer(middleware::from_fn_with_state(
                state.clone(),
                require_access_token,
            )),
        )
        .with_state(state)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set");
    let api_url = std::env::var("API_URL").expect("API_URL is not set");
    let access_token_secret =
        std::env::var("ACCESS_TOKEN_SECRET").expect("ACCESS_TOKEN_SECRET is not set");
    let refresh_token_secret =
        std::env::var("REFRESH_TOKEN_SECRET").expect("REFRESH_TOKEN_SECRET is not set");
    initialize_dummy_password_hash();
    let pool = db::connect(&database_url).await?;
    let state = AppState {
        pool,
        access_token_secret,
        refresh_token_secret,
        login_rate_limiter: LoginRateLimiter::default(),
    };

    let app = build_app(state);

    let listener = tokio::net::TcpListener::bind(&api_url).await?;
    println!("Server is running {}", &api_url);
    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode, header},
    };
    use http_body_util::BodyExt;
    use serde_json::Value;
    use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
    use tower::ServiceExt;

    use super::*;

    const ACCESS_SECRET: &str = "test-access-secret-with-32-bytes!";
    const REFRESH_SECRET: &str = "test-refresh-secret-with-32-bytes";

    async fn test_app() -> (Router, SqlitePool) {
        initialize_dummy_password_hash();
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("test database should connect");
        sqlx::migrate!()
            .run(&pool)
            .await
            .expect("migrations should run");
        let state = AppState {
            pool: pool.clone(),
            access_token_secret: ACCESS_SECRET.to_owned(),
            refresh_token_secret: REFRESH_SECRET.to_owned(),
            login_rate_limiter: LoginRateLimiter::default(),
        };

        (build_app(state), pool)
    }

    fn json_request(method: &str, uri: &str, body: &str) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_owned()))
            .expect("request should build")
    }

    async fn response_json(response: axum::response::Response) -> Value {
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("response body should be readable")
            .to_bytes();
        serde_json::from_slice(&bytes).expect("response should contain JSON")
    }

    #[tokio::test]
    async fn auth_flow_rotates_tokens_and_logout_revokes_access_immediately() {
        let (app, pool) = test_app().await;
        let registration = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/auth/register",
                r#"{"username":"alice","password":"a-long-safe-passphrase"}"#,
            ))
            .await
            .expect("registration request should succeed");

        assert_eq!(registration.status(), StatusCode::CREATED);
        assert_eq!(
            registration.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-store"
        );
        let initial_cookie = registration
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned();
        let registration_json = response_json(registration).await;
        let initial_access_token = registration_json["access_token"]
            .as_str()
            .unwrap()
            .to_owned();
        let initial_claims =
            auth::decode_access_token(&initial_access_token, ACCESS_SECRET.as_bytes())
                .expect("issued access token should validate");
        assert!(
            auth::is_access_session_active(&pool, &initial_claims)
                .await
                .expect("session lookup should succeed")
        );

        let protected = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/me")
                    .header(
                        header::AUTHORIZATION,
                        format!("Bearer {initial_access_token}"),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(protected.status(), StatusCode::OK);

        let refresh = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/refresh")
                    .header(header::COOKIE, &initial_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(refresh.status(), StatusCode::OK);
        assert_eq!(
            refresh.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-store"
        );
        let rotated_cookie = refresh
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned();
        assert_ne!(initial_cookie, rotated_cookie);
        let refresh_json = response_json(refresh).await;
        let refreshed_access_token = refresh_json["access_token"].as_str().unwrap();

        let logout = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/logout")
                    .header(header::COOKIE, rotated_cookie)
                    .header(
                        header::AUTHORIZATION,
                        format!("Bearer {refreshed_access_token}"),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(logout.status(), StatusCode::OK);

        let protected_after_logout = app
            .oneshot(
                Request::builder()
                    .uri("/api/me")
                    .header(
                        header::AUTHORIZATION,
                        format!("Bearer {refreshed_access_token}"),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(protected_after_logout.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn unknown_user_and_wrong_password_have_the_same_response() {
        let (app, pool) = test_app().await;
        let password_hash = auth::hash_password("the-correct-passphrase").unwrap();
        sqlx::query("INSERT INTO users (username, password_hash) VALUES (?, ?)")
            .bind("known-user")
            .bind(password_hash)
            .execute(&pool)
            .await
            .unwrap();

        let wrong_password = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/auth/login",
                r#"{"username":"known-user","password":"the-wrong-passphrase"}"#,
            ))
            .await
            .unwrap();
        let unknown_user = app
            .oneshot(json_request(
                "POST",
                "/auth/login",
                r#"{"username":"unknown-user","password":"the-wrong-passphrase"}"#,
            ))
            .await
            .unwrap();

        assert_eq!(wrong_password.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(unknown_user.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            response_json(wrong_password).await,
            response_json(unknown_user).await
        );
    }

    #[tokio::test]
    async fn login_is_rate_limited_after_repeated_failures() {
        let (app, _) = test_app().await;

        for _ in 0..5 {
            let response = app
                .clone()
                .oneshot(json_request(
                    "POST",
                    "/auth/login",
                    r#"{"username":"rate-limited-user","password":"incorrect-passphrase"}"#,
                ))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }

        let response = app
            .oneshot(json_request(
                "POST",
                "/auth/login",
                r#"{"username":"rate-limited-user","password":"incorrect-passphrase"}"#,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }
}
