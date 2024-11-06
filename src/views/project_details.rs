use axum::debug_handler;
use axum::extract::Path;
use axum::response::IntoResponse;
use maud::html;
use serde::Deserialize;

use crate::views::helpers::wrap_admin_template;
use crate::{Error, SentryToken};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiIssue {
    title: String,
    culprit: String,
    first_seen: String,
    last_seen: String,
    status: String,
    level: String,
    permalink: String,
}

#[debug_handler]
pub async fn project_details(
    token: SentryToken,
    Path((org, proj)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    let client = token.client()?;
    let response: Vec<ApiIssue> = client
        .get(format!(
            "https://sentry.io/api/0/projects/{org}/{proj}/issues/"
        ))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(wrap_admin_template(html! {
        h2 {
            a href=(format!("/organizations/{org}")) { (org) }
            (format!("/{proj}: issues"))
        }
        ul {
            @for issue in response {
                li {
                    a href=(issue.permalink) {
                        small.level { (issue.level) "; " (issue.status) }
                        strong { (issue.title) }
                        small { (issue.culprit) "; " (issue.first_seen) "-" (issue.last_seen) }
                    }
                }
            }
        }
    }))
}
