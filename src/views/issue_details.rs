use axum::debug_handler;
use axum::extract::Path;
use axum::response::IntoResponse;
use jiff::Timestamp;
use maud::html;
use serde::Deserialize;

use crate::views::helpers::{print_relative_time, wrap_admin_template, LayoutOptions};
use crate::{Error, SentryToken};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiIssue {
    title: String,
    culprit: String,
    first_seen: Timestamp,
    last_seen: Timestamp,
    status: String,
    level: String,
    permalink: String,
    short_id: String,
    #[serde(default)]
    logger: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiEvent {
    #[serde(default)]
    message: String,

    #[serde(default)]
    tags: Vec<ApiTag>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiTag {
    key: String,
    value: String,
}

#[debug_handler]
pub async fn issue_details(
    token: SentryToken,
    Path((org, proj, issue_id)): Path<(String, String, String)>,
) -> Result<impl IntoResponse, Error> {
    let client = token.client()?;

    let (issue_response, event_response) = tokio::try_join!(
        async {
            client
                .get(format!(
                    // XXX: the docs here are out of date: https://docs.sentry.io/api/events/retrieve-an-issue/
                    "https://sentry.io/api/0/organizations/{org}/issues/{issue_id}/"
                ))
                .send()
                .await?
                .error_for_status()?
                .json::<ApiIssue>()
                .await
        },
        async {
            client
                .get(format!(
                    "https://sentry.io/api/0/organizations/{org}/issues/{issue_id}/events/latest/"
                ))
                .send()
                .await?
                .error_for_status()?
                .json::<ApiEvent>()
                .await
        }
    )?;

    let title = issue_response.title;

    let body = wrap_admin_template(
        LayoutOptions {
            title: format!("{title} - {org}/{proj}"),
            ..Default::default()
        },
        html! {
            h2 {
                a href=(format!("/{org}")) { (org) }
                "/"
                a href=(format!("/{org}/{proj}")) { (proj) }
                "/"
                code { (issue_response.short_id) }
            }

            h3 {
                span data-level=(issue_response.level) { (issue_response.level) ": " }
                @if event_response.message.is_empty() {
                    (title)
                } @else {
                    (event_response.message)
                }
            }

            @if !issue_response.culprit.is_empty() {
                p.culprit.prop { span.label { "culprit: " } code { (issue_response.culprit) } }
            }
            p.prop { span.label { "first seen: " } code { (print_relative_time(issue_response.first_seen)) } }
            p.prop { span.label { "last seen: " } code { (print_relative_time(issue_response.last_seen)) } }
            @if let Some(ref logger) = issue_response.logger {
                p.prop { span.label { "logger: " } code { (logger) } }
            }
            p.prop { span.label { "status: " } code { (issue_response.status) } }

            hr;

            p {
                "stacktrace? contexts? not yet. "

                a href=(issue_response.permalink) {
                    "view on sentry for now."
                }
            }

            h3 { "tags" }

            ul {
                @for tag in event_response.tags {
                    li.prop {
                        span.label { (tag.key) ": " } code { (tag.value) }
                        " ("
                        a.secondary href=(format!("/{org}/{proj}?query={}:{}", tag.key, tag.value)) {
                            "more"
                        }
                        ")"
                    }
                }
            }
        },
    );

    let headers = [("Cache-control", "private, max-age=300")];

    Ok((headers, body))
}
