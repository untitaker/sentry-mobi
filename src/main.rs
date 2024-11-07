use std::net::SocketAddr;

use time::Duration;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Router,
};
use memory_serve::{load_assets, MemoryServe};
use tower_sessions::{Expiry, MemoryStore, SessionManagerLayer};

mod views;

pub(crate) use views::SentryToken;

#[tokio::main]
async fn main() {
    let static_files = MemoryServe::new(load_assets!("static")).into_router();

    tracing_subscriber::fmt::init();

    let session_store = MemoryStore::default();

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(!cfg!(debug_assertions))
        .with_expiry(Expiry::OnInactivity(Duration::seconds(3600)));

    let app = Router::new()
        .route("/", get(views::index))
        .merge(static_files)
        .route("/auth", post(views::auth))
        .route("/auth/logout", post(views::logout))
        .route("/organizations", get(views::organization_overview))
        .route("/:org", get(views::organization_details))
        .route("/:org/:proj", get(views::project_details))
        .route("/:org/:proj/issues/:id", get(views::issue_details))
        .layer(session_layer);

    let addr = "0.0.0.0:3000";
    tracing::info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("failed to update session")]
    Session(#[from] tower_sessions::session::Error),
    #[error("no token found")]
    NeedsAuth { redirect_to: Option<String> },
    #[error("failed to fetch from sentry api: {0}")]
    Reqwest(#[from] reqwest::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Error::NeedsAuth { redirect_to } => {
                if let Some(redirect_to) = redirect_to {
                    Redirect::to(&format!("/?redirect_to={redirect_to}")).into_response()
                } else {
                    Redirect::to("/").into_response()
                }
            }
            _ => {
                let s = self.to_string();
                tracing::error!("error while serving request: {:?}", s);
                (StatusCode::INTERNAL_SERVER_ERROR, s).into_response()
            }
        }
    }
}
