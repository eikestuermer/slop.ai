//! Self-hosted Slop AI sync server.
//!
//! Run a single binary on a Raspberry Pi or any cheap VM to back a team's
//! collaborative projects. Stateless beyond a `sled` key-value store at
//! `--data-dir`. Each project is keyed by `project_id` (UUID) and exposes
//! a single WebSocket endpoint at `/ws/<project_id>` that speaks the
//! Automerge sync protocol.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
    routing::{any, get},
    Router,
};
use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use slop_sync::{SyncSession, TimelineDoc};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Bind address.
    #[arg(long, default_value = "0.0.0.0:7878")]
    bind: String,
    /// Data directory for the embedded sled database.
    #[arg(long, default_value = "./slop-sync-data")]
    data_dir: String,
}

#[derive(Clone)]
struct AppState {
    db: Arc<sled::Db>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    let args = Args::parse();
    let db = Arc::new(sled::open(&args.data_dir)?);
    let state = AppState { db };

    let app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/ws/:project_id", any(ws_handler))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    tracing::info!("slop-sync-server listening on {}", args.bind);
    let listener = tokio::net::TcpListener::bind(&args.bind).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn ws_handler(
    Path(project_id): Path<String>,
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, project_id, state))
}

async fn handle_socket(socket: WebSocket, project_id: String, state: AppState) {
    let (mut tx, mut rx) = socket.split();

    // Load (or create) the project document.
    let key = format!("project:{}", project_id);
    let bytes = state.db.get(key.as_bytes()).ok().flatten();
    let mut doc = match bytes {
        Some(b) => match TimelineDoc::load(&b) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!(?e, "failed to load doc; creating fresh");
                TimelineDoc::empty().expect("empty doc")
            }
        },
        None => TimelineDoc::empty().expect("empty doc"),
    };
    let session = Arc::new(Mutex::new(SyncSession::new()));

    // Initial server-side push if there is any state to send.
    if let Some(initial) = session
        .lock()
        .await
        .generate_message(&mut doc)
        .ok()
        .flatten()
    {
        if tx.send(Message::Binary(initial)).await.is_err() {
            return;
        }
    }

    while let Some(msg) = rx.next().await {
        match msg {
            Ok(Message::Binary(bytes)) => {
                if let Err(e) = session.lock().await.receive_message(&mut doc, &bytes) {
                    tracing::warn!(?e, "bad sync message");
                    continue;
                }
                // Persist after each successful receive.
                let snapshot = doc.save();
                let _ = state.db.insert(key.as_bytes(), snapshot);
                let _ = state.db.flush_async().await;

                while let Some(out) = session
                    .lock()
                    .await
                    .generate_message(&mut doc)
                    .ok()
                    .flatten()
                {
                    if tx.send(Message::Binary(out)).await.is_err() {
                        return;
                    }
                }
            }
            Ok(Message::Close(_)) | Err(_) => break,
            _ => {}
        }
    }
}
