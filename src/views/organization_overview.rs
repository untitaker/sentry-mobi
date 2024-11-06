use axum::debug_handler;
use axum::response::IntoResponse;
use maud::html;
use serde::Deserialize;

use crate::views::helpers::wrap_admin_template;
use crate::{Error, SentryToken};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiOrganization {
    name: String,
    slug: String,
    #[serde(default)]
    is_bookmarked: bool,
}

#[debug_handler]
pub async fn organization_overview(token: SentryToken) -> Result<impl IntoResponse, Error> {
    let client = token.client()?;
    let mut response: Vec<ApiOrganization> = client
        .get("https://sentry.io/api/0/organizations/")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    response.sort_by_key(|o| !o.is_bookmarked);

    Ok(wrap_admin_template(html! {
        ul {
            @for org in response {
                li {
                    a href=(format!("/organizations/{}", org.slug)) {
                        (org.name)
                    }
                }
            }
        }
    }))
}
