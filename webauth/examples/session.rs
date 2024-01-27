use axum::{response::IntoResponse, routing::get, Router};
use std::net::SocketAddr;
use webauth::session::{Session, SessionManagerLayer};
use webauth_store_memory::SessionStore as MemorySessionStore;

async fn root(session: Session) -> impl IntoResponse {
    //session.insert("test", "value");
    format!("hello world")
}

#[tokio::main]
async fn main() {
    let store = MemorySessionStore::default();
    let layer = SessionManagerLayer::new(store, "uid");

    let app = Router::new().route("/", get(root).layer(layer));

    let addr = SocketAddr::from(([127, 0, 0, 1], 42000));
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
