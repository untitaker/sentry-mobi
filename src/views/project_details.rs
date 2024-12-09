use axum::extract::Query;
use axum::response::IntoResponse;
use jiff::Timestamp;
use maud::Markup;
use serde::Deserialize;

use crate::api_helpers::get_next_link;
use crate::routes::{IssueDetails, OrganizationDetails, ProjectDetails};
use crate::views::helpers::{
    breadcrumbs, event_count, html, print_relative_time, wrap_admin_template, Html, LayoutOptions,
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
    #[serde(default)]
    next_link: Option<String>,
}

pub async fn project_details(
    route: ProjectDetails,
    token: SentryToken,
    Query(params): Query<SearchQuery>,
) -> Result<impl IntoResponse, Error> {
    let org = route.org;
    let proj = route.proj;

    let client = token.client()?;
    let query = params
        .query
        .as_deref()
        .unwrap_or("is:unresolved issue.priority:[high, medium]");
    let http_response =
        client
            .get(params.next_link.unwrap_or_else(|| {
                format!("https://sentry.io/api/0/projects/{org}/{proj}/issues/")
            }))
            .query(&[("query", query), ("limit", "25")])
            .send()
            .await?
            .error_for_status()?;
    let next_link = get_next_link(&http_response);
    let response: Vec<ApiIssue> = http_response.json().await?;

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
                a preload="mouseover" href=(OrganizationDetails { org: org.clone() }) { (org) }
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

            form method="get" action=(ProjectDetails { org: org.clone(), proj: proj.clone()}) {
                fieldset role="group" {
                    input type="text" name="query" value=(query);
                    input type="submit" value="filter issues";
                }
            }

            (render_issuestream(&org, &proj, &response, next_link.as_deref()))
        },
    );

    Ok(Html(body))
}

fn render_issuestream(
    org: &str,
    proj: &str,
    response: &[ApiIssue],
    next_link: Option<&str>,
) -> Markup {
    html! {
        @if !response.is_empty() {
            div.page {
                @for issue in response {
                    div.issue-row {
                        a preload="mouseover" href=(IssueDetails { org: org.to_owned(), proj: proj.to_owned(), issue_id: issue.id.clone() }) {
                            span data-level=(issue.level) { (issue.level) ": "}
                            (issue.title)

                            br;

                            small.secondary {
                                (event_count(&issue.count))
                                ", last seen "
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

                @if let Some(link) = next_link {
                    @let href = format!("?{}", url::form_urlencoded::Serializer::new(String::new())
                        .append_pair("next_link", link)
                        .finish());

                    a href=(href) hx-get=(href) hx-trigger="revealed" hx-swap="outerHTML" hx-select=".page > *" {
                        "next page"
                    }
                }
            }
        } @else {
            "nothing found."
        }
    }
}
