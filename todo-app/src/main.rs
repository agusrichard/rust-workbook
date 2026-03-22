use axum::{Router, routing::get};

pub mod routes;
pub mod models;
pub mod response;
pub mod state;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(routes::hello_world))
        .nest("/todos", routes::todo::router());

    let host = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(host).await.unwrap();
    println!("Listing on {host}");
    axum::serve(listener, app).await.unwrap();
}