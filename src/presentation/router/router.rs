use axum::{Router, routing::post};

pub async fn get_router() -> Router {
    let router = Router::new().route("/process", post(|| async { "Processing request" }));
    router
}
