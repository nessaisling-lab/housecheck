use axum::{routing::get, Router};

/// Build the router. Takes a DB path so tests can point at an in-memory/fixture DB.
pub fn app() -> Router {
    Router::new().route("/health", get(|| async { "ok" }))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8787").await.unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app()).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;

    #[tokio::test]
    async fn health_returns_ok() {
        let server = TestServer::new(app()).unwrap();
        let res = server.get("/health").await;
        res.assert_status_ok();
        res.assert_text("ok");
    }
}
