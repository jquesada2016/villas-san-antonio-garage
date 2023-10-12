#[macro_use]
extern crate askama;

use axum::response::IntoResponse;
use axum::routing::get;

#[tokio::main]
async fn main() {
  let app = axum::Router::new().route("/", get(home_handler));

  axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
    .serve(app.into_make_service())
    .await
    .unwrap();
}

async fn home_handler() -> impl IntoResponse {
  #[derive(Template)]
  #[template(path = "index.html")]
  struct HomePage;

  HomePage
}
