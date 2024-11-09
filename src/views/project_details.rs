use axum::debug_handler;
use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use human_repr::HumanCount;
use jiff::Timestamp;
use maud::{Markup, html};
use serde::Deserialize;

use crate::views::helpers::{
    print_relative_time, breadcrumbs, wrap_admin_template, LayoutOptions,
};
use crate::{Error, SentryToken};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiIssue {
    title: String,
    culprit: String,
    last_seen: Timestamp,
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
            (breadcrumbs(&format!("https://sentry.io/issues/?project={project_id}&query={query}&statsPeriod=24h"), html! {
                a preload="mouseover" href=(format!("/{org}")) { (org) }
                (format!("/{proj}"))
                    ": issues"
            }))

            style {
                r#"
                .issue-row {
                    padding: calc(var(--pico-spacing)/ 2) var(--pico-spacing);
                    margin-bottom: 0;
                    border-bottom: var(--pico-border-width) solid var(--pico-table-border-color);
                }

                .issue-row a {
                    text-decoration: none;
                }

                code {
                    word-wrap: anywhere;
                }
                "#
            }

            form method="get" action=(format!("/{org}/{proj}")) {
                fieldset role="group" {
                    input type="text" name="query" value=(query);
                    input type="submit" value="filter issues";
                }
            }

            (render_issuestream(&org, &proj, &response))
        },
    );

    let headers = [("Cache-control", "private, max-age=300")];

    Ok((headers, body))
}

fn render_issuestream(org: &str, proj: &str, response: &[ApiIssue]) -> Markup {
    html! {
        @for issue in response {
            div.issue-row {
                a preload="mouseover" href=(format!("/{org}/{proj}/issues/{}", issue.id)) {
                    span data-level=(issue.level) { (issue.level) ": "}
                    (issue.title)

                    br;

                    small.secondary {
                        (issue.count.parse().map(|x: u128| x.human_count_bare().to_string()).unwrap_or(issue.count.clone()))
                        " events, last seen "
                        (print_relative_time(issue.last_seen))
                        " ago"

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
        }

        @if response.is_empty() {
            p {
                "nothing found."
            }
        }
    }
}
