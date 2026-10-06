pub mod db;
pub mod models;
pub mod query;
pub mod mutation;
pub mod validation;
pub mod auth;
pub mod subscription;
#[cfg(test)]
mod tests;

use async_graphql::{Schema};
use async_graphql::http::GraphiQLSource;
use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse};
use axum::Router;
use axum::routing::{get, post};
use sqlx::SqlitePool;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use query::Query;
use mutation::Mutation;
use crate::auth::{user_from_headers, JwtSecret};
use crate::models::Book;
use crate::subscription::Subscription;

const CHANNEL_CAPACITY: usize = 16;

type AppSchema = Schema<Query, Mutation, Subscription>;

#[derive(Clone)]
struct AppState {
    schema: AppSchema,
    secret: JwtSecret,
}

async fn graphiql() -> impl IntoResponse {
    Html(
        GraphiQLSource::build()
            .endpoint("/graphql")
            .subscription_endpoint("/ws")
            .finish(),
    )
}

async fn graphql(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: GraphQLRequest,
) -> GraphQLResponse {
    let mut request = request.into_inner();
    if let Some(user) = user_from_headers(&headers, &state.secret) {
        request = request.data(user);
    }
    state.schema.execute(request).await.into()
}

fn build_schema(pool: SqlitePool, secret: JwtSecret) -> AppSchema {
    let (events, _) = broadcast::channel::<Book>(CHANNEL_CAPACITY);
    Schema::build(Query, Mutation, Subscription)
        .data(pool)
        .data(secret)
        .data(events)
        .finish()
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set");
    let secret = JwtSecret(std::env::var("JWT_SECRET").expect("JWT_SECRET is not set"));

    let pool = db::connect(&database_url).await.unwrap();
    let schema = build_schema(pool, secret.clone());
    let app = Router::new()
        .route("/", get(graphiql))
        .route("/graphql", post(graphql))
        .route_service("/ws", GraphQLSubscription::new(schema.clone()))
        .with_state(AppState { schema, secret });

    let listener = TcpListener::bind("127.0.0.1:8000").await.unwrap();
    println!("GraphiQL: http://localhost:8000");
    axum::serve(listener, app).await.unwrap();
}
