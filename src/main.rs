mod cryptography;
mod mime;
mod providers;
mod routes;

use anyhow::{Context, Result};
use axum::{
    Router,
    extract::{DefaultBodyLimit, Request},
    handler::Handler,
    http::{HeaderValue, header},
    middleware::{self as axum_middleware, Next},
    routing::{delete, get, post},
};
use bytesize::ByteSize;
use clap::Parser;
use clap_duration::duration_range_value_parse;
use cryptography::Cryptography;
use dotenvy::dotenv;
use duration_human::{DurationHuman, DurationHumanValidator};
use mime_guess::{Mime, mime::IMAGE_STAR};
use providers::auth::AuthProvider;
use providers::storage::StorageProvider;
use std::{net::SocketAddr, path::PathBuf, str::FromStr, sync::Arc, time::Duration};
use tokio::{net::TcpListener, signal};
use tower_http::{
    catch_panic::CatchPanicLayer,
    normalize_path::NormalizePathLayer,
    trace::{DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::{Level, debug, info, info_span};
use tracing_subscriber::EnvFilter;
use url::Url;

const UPLOADS_DIRNAME: &str = "uploads";
const PERSISTED_SALT_FILENAME: &str = "persisted_salt";

#[derive(Debug, Clone, Parser)]
#[clap(author, about, version)]
struct Arguments {
    /// Internet socket address that the server should be ran on.
    #[arg(
        long = "address",
        env = "DOLLHOUSE_ADDRESS",
        default_value = "127.0.0.1:8731"
    )]
    address: SocketAddr,

    /// Base url to use when generating links to uploads.
    ///
    /// This is only for link generation, you'll need to handle the reverse proxy yourself.
    #[arg(
        long = "public-url",
        env = "DOLLHOUSE_PUBLIC_URL",
        default_value = "http://127.0.0.1:8731"
    )]
    public_url: Url,

    /// One or more bearer tokens to use when interacting with authenticated endpoints.
    #[clap(
        long = "tokens",
        env = "DOLLHOUSE_TOKENS",
        required = true,
        value_delimiter = ','
    )]
    tokens: Vec<String>,

    /// Path to the directory where data should be stored.
    ///
    /// CAUTION: This directory should not be used for anything else as it and all subdirectories will be automatically managed.
    #[clap(
        long = "data-path", 
        env = "DOLLHOUSE_DATA_PATH",
        default_value = dirs::data_local_dir().unwrap().join("dollhouse").into_os_string()
    )]
    data_path: PathBuf,

    /// Time since since last access before a file is automatically purged from storage.
    #[clap(long = "upload-expiry", env = "DOLLHOUSE_UPLOAD_EXPIRY", value_parser = duration_range_value_parse!(min: 1min, max: 100years))]
    upload_expiry: Option<DurationHuman>,

    /// Maximum file size that can be uploaded.
    #[clap(
        long = "upload-size-limit",
        env = "DOLLHOUSE_UPLOAD_SIZE_LIMIT",
        default_value = "50MB"
    )]
    upload_size_limit: ByteSize,

    /// File mimetypes that can be uploaded.
    /// Supports type wildcards (e.g. 'image/*', '*/*').
    ///
    /// MIME types are determined by the magic numbers of uploaded content, if the mimetype cannot be determined the server will either:
    ///     - Fallback to `application/octet-stream if all mimetypes are allowed (using `*/*`).
    ///     - Reject the upload with an error informing the uploader the mime type could not be determined.
    #[clap(
        long = "upload-mimetypes",
        env = "DOLLHOUSE_UPLOAD_MIMETYPES",
        default_values_t = [
            IMAGE_STAR,
            Mime::from_str("video/*").unwrap()
        ],
        value_delimiter = ','
    )]
    upload_mimetypes: Vec<Mime>,
}

#[derive(Debug, Clone)]
struct AppState {
    storage_provider: Arc<StorageProvider>,
    auth_provider: Arc<AuthProvider>,
    public_base_url: Url,
    upload_allowed_mimetypes: Vec<Mime>,
    persisted_salt: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("info")))
        .init();
    let args = Arguments::parse();

    // Init required state.
    let storage = Arc::new(StorageProvider::new(args.data_path.join(UPLOADS_DIRNAME))?);
    let persisted_salt = {
        let path = args.data_path.join(PERSISTED_SALT_FILENAME);
        if let Some(salt) = Cryptography::get_persisted_salt(&path)? {
            salt
        } else {
            Cryptography::create_persisted_salt(&path)?
        }
    };
    let state = AppState {
        storage_provider: Arc::clone(&storage),
        auth_provider: Arc::new(AuthProvider::new(args.tokens.clone())),
        public_base_url: args.public_url.clone(),
        upload_allowed_mimetypes: args.upload_mimetypes.clone(),
        persisted_salt,
    };

    // Background task for expiring files.
    if let Some(expire_after) = args.upload_expiry.map(|e| Duration::from(&e)) {
        let storage_clone = Arc::clone(&storage);
        tokio::spawn(async move {
            loop {
                debug!("Running file expiry check");
                storage_clone
                    .remove_all_expired_files(expire_after)
                    .unwrap();
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        });
    }

    // Start server.
    let tcp_listener = TcpListener::bind(args.address).await?;
    let router = Router::new()
        .route("/", get(routes::index_handler))
        .route("/index.css", get(routes::index_css_handler))
        .route("/index.js", get(routes::index_js_handler))
        .route("/favicon.ico", get(routes::favicon_handler))
        .route("/health", get(routes::health_handler))
        .route(
            "/statistics",
            get(routes::statistics_handler).layer(axum_middleware::from_fn_with_state(
                state.clone(),
                AuthProvider::valid_auth_middleware,
            )),
        )
        .route("/upload/{id}", get(routes::uploads::get_upload_handler))
        .route(
            "/upload",
            post(
                routes::uploads::create_upload_handler
                    .layer(DefaultBodyLimit::max(
                        args.upload_size_limit
                            .0
                            .try_into()
                            .context("upload limit does not fit into usize")?,
                    ))
                    .layer(axum_middleware::from_fn_with_state(
                        state.clone(),
                        AuthProvider::valid_auth_middleware,
                    )),
            ),
        )
        .route(
            "/upload/{id}",
            delete(routes::uploads::delete_upload_handler).layer(
                axum_middleware::from_fn_with_state(
                    state.clone(),
                    AuthProvider::valid_auth_middleware,
                ),
            ),
        )
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    let uri = request.uri().to_string();
                    let path_without_query = if let Some(query_start) = uri.find('?') {
                        &uri[..query_start]
                    } else {
                        &uri
                    };
                    info_span!(
                        "request",
                        method = ?request.method(),
                        path = path_without_query,
                        version = ?request.version(),
                    )
                })
                .on_request(DefaultOnRequest::default())
                .on_response(DefaultOnResponse::default().level(Level::INFO))
                .on_failure(DefaultOnFailure::default()),
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
                res
            },
        ))
        .with_state(state);
    info!(
        "Internal server started\n\
         * Listening on: http://{}\n\
         * Public URL: {}\n\
         * Data path: {:?}\n\
         * Upload size limit: {}\n\
         * Upload expiry: {}\n\
         * Allowed mimetypes: {:?}\n\
         * Tokens configured: {}",
        args.address,
        args.public_url,
        args.data_path,
        args.upload_size_limit.display().si(),
        args.upload_expiry
            .map_or_else(|| "disabled".to_string(), |v| v.to_string()),
        args.upload_mimetypes,
        args.tokens.len()
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
