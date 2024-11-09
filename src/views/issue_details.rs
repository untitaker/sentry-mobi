use std::collections::BTreeMap;

use axum::response::IntoResponse;
use jiff::Timestamp;
use maud::{html, Markup};
use serde::Deserialize;

use crate::views::helpers::{breadcrumbs, print_relative_time, wrap_admin_template, LayoutOptions};
use crate::{Error, SentryToken};

const MAX_BREADCRUMBS: usize = 20;

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
    #[serde(rename = "dateCreated")]
    timestamp: Timestamp,

    #[serde(default)]
    tags: Vec<ApiTag>,

    entries: Vec<ApiEventEntry>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ApiEventEntry {
    Known(KnownEventEntry),
    Other {
        #[serde(rename = "type")]
        ty: String,
        #[serde(flatten)]
        attributes: BTreeMap<String, serde_json::Value>,
    },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum KnownEventEntry {
    Message { data: MessageData },
    Breadcrumbs { data: BreadcrumbData },
    Threads { data: ThreadsData },
    Exception { data: ExceptionData },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MessageData {
    formatted: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BreadcrumbData {
    values: Vec<Breadcrumb>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Breadcrumb {
    timestamp: Timestamp,
    level: String,
    message: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExceptionData {
    values: Vec<Exception>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Exception {
    #[serde(rename = "type", default)]
    ty: String,
    #[serde(default)]
    value: String,
    #[serde(default)]
    stacktrace: Option<Stacktrace>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThreadsData {
    values: Vec<Thread>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Thread {
    #[serde(default)]
    crashed: bool,
    #[serde(default)]
    current: bool,
    #[serde(default)]
    stacktrace: Option<Stacktrace>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Stacktrace {
    frames: Vec<Frame>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Frame {
    #[serde(default)]
    filename: Option<String>,
    #[serde(default)]
    function: Option<String>,
    #[serde(default)]
    line_no: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiTag {
    key: String,
    value: String,
}

pub async fn issue_details(
    route: crate::routes::IssueDetails,
    token: SentryToken,
) -> Result<impl IntoResponse, Error> {
    let org = route.org;
    let proj = route.proj;
    let issue_id = route.id;

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
            (breadcrumbs(&issue_response.permalink, html! {
                a href=(crate::routes::OrganizationDetails { org: org.clone() }) {
                    (org)
                }
                "/"
                a href=(crate::routes::ProjectDetails { org: org.clone(), proj: proj.clone() }) {
                    (proj)
                }
                "/"
                (issue_response.short_id)
            }))

            h3 {
                span data-level=(issue_response.level) { (issue_response.level) ": " }
                (title)
            }

            p { i {
                "first seen "
                code { (print_relative_time(issue_response.first_seen)) } " ago, "
                "last seen "
                code { (print_relative_time(issue_response.last_seen)) } " ago, "
                "status "
                code { (issue_response.status) }
                br;

                "showing latest event."
            } }

            @if !issue_response.culprit.is_empty() {
                table {
                    tr {
                        td { "culprit: " }
                        td { code { (issue_response.culprit) } }
                    }
                }
            }


            @for entry in event_response.entries {
                @match entry {
                    ApiEventEntry::Known(KnownEventEntry::Exception { data }) => {
                        @for exception in data.values {
                            details {
                                summary { "exception: " i { (exception.ty) } }
                                p.help {
                                    "the reported exception stack. a reported exception may contain another \"root cause\" exception, in which case multiple will be printed."
                                }

                                pre {
                                    (exception.value)
                                }

                                @if let Some(ref stacktrace) = exception.stacktrace {
                                    (render_stacktrace(stacktrace))
                                }

                                hr;
                            }
                        }
                    }
                    ApiEventEntry::Known(KnownEventEntry::Threads { data }) => {
                        details {
                            summary { "threads and stacktraces" }
                            p.help {
                                "threads and stacktraces in sentry show the current callstack from when the event was captured. this is not necessarily the same thing as the exception stacktrace, or where the exception was originally raised."
                            }
                            p.help {
                                "in sentry, you will also sometimes find callstacks from all running threads of the process, but sentry.mobi only shows you one thread."
                            }
                            p.help {
                                "threads can be 'crashing' (in which case most likely the exception did originate from there), and 'current' (in which case the code that captured and sent the error to sentry most likely ran there)"
                            }

                            @for thread in data.values {
                                @if !thread.crashed && !thread.current {
                                    continue;
                                }

                                p {
                                    "crashed: " code { (thread.crashed) }
                                    "; current: " code { (thread.crashed) }
                                }

                                @if let Some(ref stacktrace) = thread.stacktrace {
                                    (render_stacktrace(stacktrace))
                                }

                                hr;
                            }
                        }
                    }
                    ApiEventEntry::Known(KnownEventEntry::Message { data }) => {
                        details open="" {
                            summary { "message" }
                            p.help {
                                "the log message, for example X in "
                                code { "myLogger.error(X)" }
                                ". distinct from the exception's "
                                em { "value" }
                                ", and one may appear with or without the other."
                            }

                            table {
                                @if let Some(ref logger) = issue_response.logger {
                                    tr {
                                        td { "logger: " }
                                        td { code { (logger) } }
                                    }
                                }

                                tr {
                                    td { "formatted: " }
                                    td { code { (data.formatted) } }
                                }
                            }
                        }
                    }
                    ApiEventEntry::Known(KnownEventEntry::Breadcrumbs { data }) => {
                        details {
                            summary { "breadcrumbs" }
                            p.help {
                                "log messages from before the crash happened, most recent messages first. usually from the same thread that the error was reported from. may or may not be relevant."
                            }

                            table.overflow-auto {
                                tr {
                                    td { code { (event_response.timestamp) } }
                                    td { " this event happened" }
                                }

                                @for crumb in data.values.iter().rev().take(MAX_BREADCRUMBS){
                                    tr {
                                        td { code { (crumb.timestamp) } }
                                        td {
                                            span data-level=(crumb.level) { (crumb.level) ": "}
                                            (crumb.message)
                                        }
                                    }
                                }
                            }

                            @if data.values.len() > MAX_BREADCRUMBS {
                                p { em {
                                    "showed " (MAX_BREADCRUMBS) " out of " (data.values.len())
                                        " breadcrumbs. for more, "
                                        a href=(issue_response.permalink) {
                                            "go to the real sentry."
                                        }
                                } }
                            }
                        }
                    }
                    ApiEventEntry::Other { ty, attributes } => {
                        details {
                            summary { i { (ty) } }

                            p.help {
                                "cannot show this section. "

                                a href=(issue_response.permalink) {
                                    "view on sentry for now."
                                }
                            }

                            pre { (serde_json::to_string(&attributes).unwrap()) }
                        }
                    }
                }
            }

            details open="" {
                summary { "tags" }

                p.help {
                    "a mix of user-defined and inferred attributes that events can be searched for."
                }

                table {
                    @for tag in event_response.tags {
                        tr {
                            td { (tag.key) ": " }
                            td {
                                code { (tag.value) }
                                " ("
                                a.secondary href=(
                                    format!("{}?query={}:{}",
                                        crate::routes::ProjectDetails { org: org.clone(), proj: proj.clone() },
                                        tag.key, tag.value
                                    )
                                ) {
                                    "more"
                                }
                                ")"
                            }
                        }
                    }
                }
            }
        },
    );

    let headers = [("Cache-control", "private, max-age=300")];

    Ok((headers, body))
}

fn render_stacktrace(stacktrace: &Stacktrace) -> Markup {
    html! {
        ul {
            @for frame in &stacktrace.frames {
                li {
                    code { (frame.function.as_deref().unwrap_or_default()) }
                    @if frame.filename.is_some() {
                        " in "
                        code {
                            (frame.filename.as_deref().unwrap_or_default())
                                ":"
                                (frame.line_no.map(|x| x.to_string()).unwrap_or_default())
                        }
                    }
                }
            }
        }
    }
}
