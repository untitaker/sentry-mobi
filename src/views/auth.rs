use async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum::http::{request::Parts, StatusCode, Uri};
use axum::response::{IntoResponse, Redirect};
use axum::Form;
use serde::Deserialize;
use tower_sessions::Session;

use crate::Error;

#[derive(Deserialize)]
pub struct AuthParams {
    token: String,
    #[serde(default)]
    redirect_to: Option<String>,
}

pub async fn auth(
    _: crate::routes::Auth,
    session: Session,
    Form(params): Form<AuthParams>,
) -> Result<impl IntoResponse, Error> {
    session.insert(SESSION_COOKIE_KEY, params.token).await?;
    Ok(Redirect::to(params.redirect_to.as_deref().unwrap_or("/")))
}

pub async fn logout(
    _: crate::routes::Logout,
    session: Session,
) -> Result<impl IntoResponse, Error> {
    session.remove::<String>(SESSION_COOKIE_KEY).await?;
    Ok(Redirect::to("/"))
}

const SESSION_COOKIE_KEY: &str = "sentry_token";

/// A sentry API token. may be empty, in which case self.client will redirect to login
#[derive(Default)]
pub struct SentryToken {
    pub(super) token: String,
    redirect_to: Uri,
}

impl SentryToken {
    pub(super) fn client(&self) -> Result<reqwest::Client, Error> {
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
