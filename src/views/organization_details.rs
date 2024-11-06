use axum::debug_handler;
use axum::extract::Path;
use axum::response::IntoResponse;
use maud::html;
use serde::Deserialize;

use crate::views::helpers::wrap_admin_template;
use crate::{Error, SentryToken};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiProject {
    name: String,
    slug: String,
}

#[debug_handler]
pub async fn organization_details(
    token: SentryToken,
    Path(org): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let client = token.client()?;
    let response: Vec<ApiProject> = client
        .get(format!(
            "https://sentry.io/api/0/organizations/{org}/projects/"
        ))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(wrap_admin_template(html! {
        a href="/organizations" { "back to organizations" }
        h2 { (format!("{org}: projects")) }
        ul {
            @for project in response {
                li {
                    a href=(format!("/projects/{}/{}", org, project.slug)) {
                        (project.name)
                    }
                }
            }
        }
    }))
}
