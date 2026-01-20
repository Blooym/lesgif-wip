mod database;
mod routes;

use crate::routes::{avatar::get_avatar_handler, gif::get_gif_handler};
use anyhow::Result;
use axum::{
    Router,
    extract::Request,
    http::{HeaderValue, StatusCode, header},
    middleware::{self as axum_middleware, Next},
    routing::get,
};
use clap::Parser;
use database::Database;
use dotenvy::dotenv;
use floodgate::{client::TapClient, extern_types::Url};
use std::{net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, signal};
use tower_http::{
    catch_panic::CatchPanicLayer,
    normalize_path::NormalizePathLayer,
    trace::{self, DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::{Level, info};
use tracing_subscriber::EnvFilter;

const MAX_AVATAR_SIZE: usize = 3 * 1024 * 1024; // 3MB
const MAX_BLOB_SIZE: usize = 10 * 1024 * 1024; // 10MB

#[derive(Debug, Clone, Parser)]
#[clap(author, about, version)]
struct Arguments {
    /// Internet socket address that the server should be ran on.
    #[arg(
        long = "address",
        env = "GIFDEX_CDN_ADDRESS",
        default_value = "127.0.0.1:8291"
    )]
    address: SocketAddr,

    #[arg(long = "database-url", env = "DATABASE_URL")]
    database_url: String,

    #[arg(long = "tap-url", env = "GIFDEX_CDN_TAP_URL")]
    tap_url: Url,

    #[arg(long = "tap-password", env = "GIFDEX_CDN_TAP_PASSWORD")]
    tap_password: Option<String>,
}

struct AppState {
    database: Database,
    tap_client: TapClient,
    http_client: reqwest::Client,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("info")))
        .init();
    let args = Arguments::parse();
    let app_state = Arc::new(AppState {
        database: Database::new(&args.database_url).await?,
        tap_client: TapClient::builder(args.tap_url)
            .password(args.tap_password)
            .build()?,
        http_client: reqwest::Client::builder()
            .https_only(true)
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()?,
    });

    let router = Router::new()
        .route("/", get(async || "Gifdex CDN"))
        .route("/media/{did}/{rkey}", get(get_gif_handler))
        .route("/avatar/{did}/{cid}", get(get_avatar_handler))
        .nest(
            "/xrpc",
            Router::new().route("/", get(async || StatusCode::OK)),
        )
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_request(DefaultOnRequest::default().level(Level::INFO))
                .on_response(DefaultOnResponse::default().level(Level::INFO))
                .on_failure(DefaultOnFailure::default().level(Level::INFO)),
        )
        .layer(NormalizePathLayer::trim_trailing_slash())
        .layer(CatchPanicLayer::new())
        .layer(axum_middleware::from_fn(
            async |req: Request, next: Next| {
                let mut res = next.run(req).await;
                let res_headers = res.headers_mut();
                res_headers.insert(
                    header::SERVER,
                    HeaderValue::from_static(env!("CARGO_PKG_NAME")),
                );
                res_headers.insert("X-Robots-Tag", HeaderValue::from_static("none"));
                res_headers.insert(
                    header::ACCESS_CONTROL_ALLOW_ORIGIN,
                    HeaderValue::from_static("*"),
                );
                res
            },
        ))
        .with_state(app_state);

    let tcp_listener = TcpListener::bind(args.address).await?;
    info!(
        "Internal server started - listening on: http://{}",
        args.address,
    );
    axum::serve(tcp_listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

// https://github.com/tokio-rs/axum/blob/15917c6dbcb4a48707a20e9cfd021992a279a662/examples/graceful-shutdown/src/main.rs#L55
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
