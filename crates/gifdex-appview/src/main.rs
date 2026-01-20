mod database;
mod routes;

use crate::routes::{
    com_atproto::sync::handle_get_repo_status,
    net_gifdex::actor::{handle_get_actor_posts, handle_get_profile, handle_get_profiles},
};
use anyhow::Result;
use axum::{
    Router,
    extract::Request,
    http::{HeaderValue, header},
    middleware::{self as axum_middleware, Next},
    routing::get,
};
use clap::Parser;
use database::Database;
use dotenvy::dotenv;
use gifdex_lexicons::net_gifdex::actor::{
    get_posts::GetPostsRequest, get_profile::GetProfileRequest, get_profiles::GetProfilesRequest,
};
use jacquard_api::com_atproto::sync::get_repo_status::GetRepoStatusRequest;
use jacquard_axum::IntoRouter;
use jacquard_common::url::Url;
use std::{net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, signal};
use tower_http::{
    catch_panic::CatchPanicLayer,
    normalize_path::NormalizePathLayer,
    trace::{self, DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::{Level, info};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, Parser)]
#[clap(author, about, version)]
struct Arguments {
    #[arg(
        long = "address",
        env = "GIFDEX_APPVIEW_ADDRESS",
        default_value = "127.0.0.1:8255"
    )]
    address: SocketAddr,
    #[arg(long = "database-url", env = "DATABASE_URL")]
    database_url: String,
    #[arg(long = "cdn-url", env = "GIFDEX_CDN_URL")]
    cdn_url: Url,
}

#[derive(Clone)]
struct AppState {
    database: Arc<Database>,
    cdn_url: Url,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("info")))
        .init();
    let args = Arguments::parse();
    let app_state = AppState {
        database: Arc::new(Database::new(&args.database_url).await?),
        cdn_url: args.cdn_url,
    };
    let router = Router::new()
        .route("/", get(async || "Gifdex AppView"))
        .merge(GetProfileRequest::into_router(handle_get_profile))
        .merge(GetProfilesRequest::into_router(handle_get_profiles))
        .merge(GetRepoStatusRequest::into_router(handle_get_repo_status))
        .merge(GetPostsRequest::into_router(handle_get_actor_posts))
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
