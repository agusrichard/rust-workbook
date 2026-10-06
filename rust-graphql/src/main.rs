pub mod auth;
pub mod db;
pub mod loaders;
pub mod models;
pub mod mutation;
pub mod pagination;
pub mod query;
pub mod subscription;
#[cfg(test)]
mod tests;
pub mod validation;

use async_graphql::dataloader::DataLoader;
use async_graphql::http::{ALL_WEBSOCKET_PROTOCOLS, GraphiQLSource};
use async_graphql::{Data, Schema};
use async_graphql_axum::{GraphQLProtocol, GraphQLRequest, GraphQLResponse, GraphQLWebSocket};
use axum::Router;
use axum::extract::{State, WebSocketUpgrade};
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use sqlx::SqlitePool;
use tokio::net::TcpListener;
use tokio::sync::broadcast;

use crate::auth::{JwtSecret, user_from_connection_params, user_from_headers};
use crate::loaders::{AuthorLoader, BooksByAuthorLoader};
use crate::models::Book;
use crate::mutation::Mutation;
use crate::query::Query;
use crate::subscription::Subscription;

const CHANNEL_CAPACITY: usize = 16;

// Limits on what one request may ask for, checked before any resolver runs.
// Depth is how far fields are nested. Complexity is roughly the number of
// fields the response could contain: a paged field counts once per item.
// The depth cannot go below 15: the query GraphiQL sends to load the schema
// for its docs panel is that deep, and the limit applies to it too.
const MAX_QUERY_DEPTH: usize = 16;
const MAX_QUERY_COMPLEXITY: usize = 2000;

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

// Subscriptions. The client sends its token once, in the message that opens
// the connection, and every subscription on that connection then sees the user.
async fn graphql_ws(
    State(state): State<AppState>,
    protocol: GraphQLProtocol,
    upgrade: WebSocketUpgrade,
) -> Response {
    upgrade
        .protocols(ALL_WEBSOCKET_PROTOCOLS)
        .on_upgrade(move |socket| {
            let secret = state.secret.clone();
            GraphQLWebSocket::new(socket, state.schema.clone(), protocol)
                .on_connection_init(move |params| async move {
                    let mut data = Data::default();
                    if let Some(user) = user_from_connection_params(&params, &secret) {
                        data.insert(user);
                    }
                    Ok(data)
                })
                .serve()
        })
}

fn build_schema(pool: SqlitePool, secret: JwtSecret) -> AppSchema {
    let (events, _) = broadcast::channel::<Book>(CHANNEL_CAPACITY);
    Schema::build(Query, Mutation, Subscription)
        .data(DataLoader::new(AuthorLoader(pool.clone()), tokio::spawn))
        .data(DataLoader::new(BooksByAuthorLoader(pool.clone()), tokio::spawn))
        .data(pool)
        .data(secret)
        .data(events)
        .limit_depth(MAX_QUERY_DEPTH)
        .limit_complexity(MAX_QUERY_COMPLEXITY)
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
        .route("/ws", get(graphql_ws))
        .with_state(AppState { schema, secret });

    let listener = TcpListener::bind("127.0.0.1:8000").await.unwrap();
    println!("GraphiQL: http://localhost:8000");
    axum::serve(listener, app).await.unwrap();
}
