use async_trait::async_trait;
use axum::extract::{FromRequestParts, Query};
use axum::http::{request::Parts, StatusCode, Uri};
use axum::response::{IntoResponse, Redirect};
use axum::{debug_handler, Form};
use maud::html;
use serde::Deserialize;
use tower_sessions::Session;

use crate::Error;

mod helpers;
mod organization_details;
mod organization_overview;
mod project_details;

pub use organization_details::organization_details;
pub use organization_overview::organization_overview;
pub use project_details::project_details;

use helpers::wrap_template;

#[derive(Deserialize)]
pub struct RedirectTo {
    #[serde(default)]
    redirect_to: Option<String>,
}

#[debug_handler]
pub async fn index(
    token: SentryToken,
    Query(params): Query<RedirectTo>,
) -> Result<impl IntoResponse, Error> {
    if token.token.is_empty() {
        Ok(wrap_template(html! {
            form method="post" action="/auth" {
                input type="password" name="token";
                @if let Some(redirect_to) = params.redirect_to {
                    input type="hidden" name="redirect_to" value=(redirect_to);
                }
                input type="submit" value="Login";
            }
        })
        .into_response())
    } else {
        Ok(Redirect::to("/organizations").into_response())
    }
}

#[derive(Deserialize)]
pub struct AuthParams {
    token: String,
    #[serde(default)]
    redirect_to: Option<String>,
}

#[debug_handler]
pub async fn auth(
    session: Session,
    Form(params): Form<AuthParams>,
) -> Result<impl IntoResponse, Error> {
    session.insert(SESSION_COOKIE_KEY, params.token).await?;
    Ok(Redirect::to(params.redirect_to.as_deref().unwrap_or("/")))
}

#[debug_handler]
pub async fn logout(session: Session) -> Result<impl IntoResponse, Error> {
    session.remove::<String>(SESSION_COOKIE_KEY).await?;
    Ok(Redirect::to("/"))
}

const SESSION_COOKIE_KEY: &str = "sentry_token";

/// A sentry API token. may be empty, in which case self.client will redirect to login
#[derive(Default)]
pub struct SentryToken {
    token: String,
    redirect_to: Uri,
}

impl SentryToken {
    fn client(&self) -> Result<reqwest::Client, Error> {
        if self.token.is_empty() {
            return Err(Error::NeedsAuth {
                redirect_to: Some(self.redirect_to.to_string()),
            });
        }

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", self.token)).unwrap(),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        Ok(client)
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for SentryToken
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(req: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let redirect_to = Uri::from_request_parts(req, state).await.unwrap();
        let session = Session::from_request_parts(req, state).await?;
        let token: String = session
            .get(SESSION_COOKIE_KEY)
            .await
            .unwrap()
            .unwrap_or_default();
        Ok(SentryToken { token, redirect_to })
    }
}
