use axum::debug_handler;
use axum::extract::Path;
use axum::response::IntoResponse;
use maud::html;
use serde::Deserialize;

use crate::views::helpers::{breadcrumbs, wrap_admin_template, LayoutOptions};
use crate::{Error, SentryToken};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiProject {
    name: String,
    slug: String,
    #[serde(default)]
    is_bookmarked: bool,
}

#[debug_handler]
pub async fn organization_details(
    token: SentryToken,
    Path(org): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let client = token.client()?;
    let mut response: Vec<ApiProject> = client
        .get(format!(
            "https://sentry.io/api/0/organizations/{org}/projects/"
        ))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    response.sort_by_key(|p| !p.is_bookmarked);

    let body = wrap_admin_template(
        LayoutOptions {
            title: org.clone(),
            ..Default::default()
        },
        html! {
            (breadcrumbs(&format!("https://sentry.io/{}", org), html! {
                (org) ": projects"
            }))

            ul {
                @for project in response {
                    li {
                        a preload="mouseover" href=(format!("/{}/{}", org, project.slug)) {
                            (project.name)
                        }

                        @if project.is_bookmarked {
                            span title="bookmarked" { "📌" }
                        }

                        small {
                            " (" (org) "/" (project.slug) ")"
                        }
                    }
                }
            }
        },
    );

    let headers = [("Cache-control", "private, max-age=300")];

    Ok((headers, body))
}
