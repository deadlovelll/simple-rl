pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod presentation;

use axum;
use presentation::router::get_router;
use tokio;

#[tokio::main]
async fn main() {
    let router = get_router().await;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    axum::serve(listener, router).await.unwrap();
}
