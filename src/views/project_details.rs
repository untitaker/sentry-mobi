use axum::debug_handler;
use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use human_repr::HumanCount;
use jiff::Timestamp;
use maud::html;
use serde::Deserialize;

use crate::views::helpers::{
    print_relative_time, to_sentry_link, wrap_admin_template, LayoutOptions,
};
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
    id: String,
    #[serde(default)]
    logger: Option<String>,
    count: String,
    project: ApiProject,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiProject {
    id: String,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    query: Option<String>,
}

#[debug_handler]
pub async fn project_details(
    token: SentryToken,
    Path((org, proj)): Path<(String, String)>,
    Query(params): Query<SearchQuery>,
) -> Result<impl IntoResponse, Error> {
    let client = token.client()?;
    let query = params
        .query
        .as_deref()
        .unwrap_or("is:unresolved issue.priority:[high, medium]");
    let response: Vec<ApiIssue> = client
        .get(format!(
            "https://sentry.io/api/0/projects/{org}/{proj}/issues/"
        ))
        .query(&[("query", query)])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let project_id = response
        .first()
        .map(|x| x.project.id.as_str())
        .unwrap_or("");

    let body = wrap_admin_template(
        LayoutOptions {
            title: format!("{org}/{proj}"),
            ..Default::default()
        },
        html! {
            (to_sentry_link(&format!("https://sentry.io/issues/?project={project_id}&query={query}&statsPeriod=24h")))

            h2 {
                a preload="mouseover" href=(format!("/{org}")) { (org) }
                (format!("/{proj}"))
                ": issues"
            }

            form method="get" action=(format!("/{org}/{proj}")) {
                fieldset role="group" {
                    input type="text" name="query" value=(query);
                    input type="submit" value="filter issues";
                }
            }

            style {
                r#"
                summary::after { margin-top: -0.5rem }
                "#
            }

            @for issue in &response {
                details {
                    summary {
                        span data-level=(issue.level) { (issue.level) ": "}
                        (issue.title)

                        br;

                        small.secondary {
                            span {
                                (issue.count.parse().map(|x: u128| x.human_count_bare().to_string()).unwrap_or(issue.count.clone()))
                                " events, last seen "
                                (print_relative_time(issue.last_seen))
                                " ago"
                            }
                            span {
                                @if !issue.culprit.is_empty() {
                                    ", in "
                                    code { (issue.culprit) }
                                } @else if let Some(ref logger) = issue.logger {
                                    ", logged via "
                                    code { (logger) }
                                }
                            }
                        }
                    }

                    table {
                        tr {
                            td { "first seen: " }
                            td { code { (print_relative_time(issue.first_seen)) } }
                        }

                        @if !issue.culprit.is_empty() {
                            tr {
                                td { "culprit: " }
                                td { code { (issue.culprit) } }
                            }
                        }

                        @if let Some(ref logger) = issue.logger {
                            tr {
                                td { "logger: " }
                                td { code { (logger) } }
                            }
                        }

                        tr {
                            td { "status: " }
                            td { code { (issue.status) } }
                        }
                    }

                    div style="text-align: right" {
                        a role="button" preload="mouseover" href=(format!("/{org}/{proj}/issues/{}", issue.id)) { "view details" }
                    }
                }
            }

            @if response.is_empty() {
                p {
                    "nothing found."
                }
            }
        },
    );

    let headers = [("Cache-control", "private, max-age=300")];

    Ok((headers, body))
}
