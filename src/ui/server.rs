#[cfg(not(target_arch = "wasm32"))]
use axum::{
    Router,
    extract::Path,
    http::{StatusCode, header},
    response::IntoResponse,
    routing::get,
};
#[cfg(not(target_arch = "wasm32"))]
use rust_embed::RustEmbed;
#[cfg(not(target_arch = "wasm32"))]
use std::net::SocketAddr;
#[cfg(not(target_arch = "wasm32"))]
use tracing::info;

#[cfg(not(target_arch = "wasm32"))]
#[derive(RustEmbed)]
#[folder = "dist/"]
struct WebAssets;

#[cfg(not(target_arch = "wasm32"))]
pub async fn serve(port: u16) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/{*file}", get(static_handler));

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    info!("Serving web UI on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| anyhow::anyhow!("failed to bind to {}: {}", addr, e))?;
    axum::serve(listener, app)
        .await
        .map_err(|e| anyhow::anyhow!("axum server error: {}", e))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
async fn index_handler() -> impl IntoResponse {
    static_handler(Path("index.html".to_string())).await
}

#[cfg(not(target_arch = "wasm32"))]
async fn static_handler(Path(path): Path<String>) -> impl IntoResponse {
    let path = path.trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match WebAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => {
            if path != "index.html"
                && let Some(content) = WebAssets::get("index.html")
            {
                let mime = mime_guess::from_path("index.html").first_or_octet_stream();
                return ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response();
            }
            (StatusCode::NOT_FOUND, "404 Not Found").into_response()
        }
    }
}
