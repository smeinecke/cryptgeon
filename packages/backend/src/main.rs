use axum::{
    Router, ServiceExt,
    extract::{DefaultBodyLimit, Request},
    http::{HeaderValue, header},
    routing::{delete, get, post},
};
use dotenv::dotenv;
use tower::Layer;
use tower_http::{
    compression::CompressionLayer,
    normalize_path::NormalizePathLayer,
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
};

use cryptgeon::{config, health, note, status, store};

async fn api_not_found() -> axum::http::StatusCode {
    axum::http::StatusCode::NOT_FOUND
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    config::validate();

    if !store::can_reach_cache() {
        println!("cannot reach cache");
        panic!("cannot reach cache");
    }

    let notes_routes = Router::new()
        .route("/", post(note::create))
        .route("/{id}", delete(note::view))
        .route("/{id}", get(note::preview));
    let health_routes = Router::new().route("/healthz", get(health::report_health));
    let status_routes = Router::new().route("/status", get(status::get_status));
    let v3_routes = Router::new()
        .nest("/notes", notes_routes)
        .merge(status_routes);

    let api_routes = Router::new().nest("/v3", v3_routes).fallback(api_not_found);

    let index = format!("{}/index.html", config::FRONTEND_PATH.as_str());
    let serve_dir =
        ServeDir::new(config::FRONTEND_PATH.to_string()).fallback(ServeFile::new(index));
    let app = Router::new()
        .nest("/api", api_routes)
        .merge(health_routes)
        .fallback_service(serve_dir)
        .layer(DefaultBodyLimit::max(*config::LIMIT))
        .layer(
            CompressionLayer::new()
                .br(true)
                .deflate(true)
                .gzip(true)
                .zstd(true),
        )
        // script-src 'unsafe-inline' is required by SvelteKit's inline
        // hydration bootstrap. A nonce-based policy is a possible follow-up.
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(
                "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob: https:; font-src 'self'; connect-src 'self'; worker-src 'self' blob:; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'",
            ),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::REFERRER_POLICY,
            HeaderValue::from_static("no-referrer"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ));

    let app = NormalizePathLayer::trim_trailing_slash().layer(app);

    let listener = tokio::net::TcpListener::bind(config::LISTEN_ADDR.to_string())
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, ServiceExt::<Request>::into_make_service(app))
        .await
        .unwrap();
}
