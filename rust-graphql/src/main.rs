pub mod db;
pub mod models;
pub mod query;

use async_graphql::{EmptyMutation, EmptySubscription, Schema};
use async_graphql::http::GraphiQLSource;
use async_graphql_axum::GraphQL;
use axum::response::{Html, IntoResponse};
use axum::Router;
use axum::routing::get;
use tokio::net::TcpListener;
use query::Query;
async fn graphiql() -> impl IntoResponse {
    Html(GraphiQLSource::build().endpoint("/graphql").finish())
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set");

    let pool = db::connect(&database_url).await.unwrap();
    let schema = Schema::build(Query, EmptyMutation, EmptySubscription).data(pool).finish();
    let app = Router::new()
        .route("/", get(graphiql))
        .route_service("/graphql", GraphQL::new(schema));

    let listener = TcpListener::bind("127.0.0.1:8000").await.unwrap();
    println!("GraphiQL: http://localhost:8000");
    axum::serve(listener, app).await.unwrap();
}
