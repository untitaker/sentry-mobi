use std::collections::BTreeMap;

use axum::response::{IntoResponse, Redirect};
use axum::Form;
use axum_htmx::HxRequest;
use jiff::Timestamp;
use maud::{html, Markup, PreEscaped};
use serde::{Deserialize, Serialize};

use crate::routes::IssueDetails;
use crate::views::helpers::{
    breadcrumbs, event_count, print_relative_time, wrap_admin_template, Html, LayoutOptions,
};
use crate::{Error, SentryToken};

const MAX_BREADCRUMBS: usize = 20;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiIssue {
    title: String,
    first_seen: Timestamp,
    last_seen: Timestamp,
    status: String,
    level: String,
    permalink: String,
    short_id: String,
    #[serde(default)]
    logger: Option<String>,
    count: String,
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
    Request { data: RequestData },
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
    in_app: bool,

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

#[derive(Deserialize)]
struct RequestData {
    method: String,
    url: String,
    //#[serde(default)]
    //query: Vec<String>,
    //#[serde(default)]
    //fragment: Option<String>,
    #[serde(default)]
    headers: Vec<(String, String)>,
    #[serde(default)]
    env: BTreeMap<String, String>,
    #[serde(default)]
    cookies: Vec<(String, String)>,
    #[serde(default)]
    data: Option<serde_json::Value>,
}

pub async fn issue_details(
    IssueDetails {
        org,
        proj,
        issue_id,
    }: IssueDetails,
    token: SentryToken,
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

            div.grid {
                h2 style="grid-column-end: span 2" {
                    span data-level=(issue_response.level) { (issue_response.level) ": " }
                    (title)
                }

                div {
                    (render_button_status(&issue_response.status))
                }
            }

            p { i {
                (event_count(&issue_response.count))
                ", showing latest event."
                br;
                "first seen "
                code { (print_relative_time(issue_response.first_seen)) } " ago, "
                br;
                "last seen "
                code { (print_relative_time(issue_response.last_seen)) } " ago. "
            } }

            style {
                (PreEscaped(r#"
                :scope {
                    .event-entries > details {
                        padding: 0.5rem;
                        padding-left: 1rem;
                        border-radius: var(--pico-border-radius);
                    }

                    .event-entries > details[open] {
                        background: var(--pico-card-background-color);
                    }

                    .event-entries > details > summary {
                        margin-left: -0.5rem;
                    }

                    .event-entries > details > summary::after {
                        margin-top: 0.33rem;
                    }

                    .event-entries > details > summary > h3 {
                        display: inline;
                    }
                }
                "#))
            }

            div.event-entries {
                @for entry in event_response.entries {
                    @match entry {
                        ApiEventEntry::Known(KnownEventEntry::Exception { data }) => {
                            @for exception in data.values {
                                details open="" {
                                    summary { h3 {
                                        "exception: "
                                        code { (exception.ty) }
                                    } }
                                    p.help {
                                        "the reported exception stack. a reported exception may contain another \"root cause\" exception, in which case multiple will be printed."
                                    }

                                    pre {
                                        (exception.value)
                                    }

                                    @if let Some(ref stacktrace) = exception.stacktrace {
                                        (render_stacktrace(stacktrace))
                                    }
                                }
                            }
                        }
                        ApiEventEntry::Known(KnownEventEntry::Threads { data }) => {
                            details {
                                summary { h3 { "threads and stacktraces" } }
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
                            details {
                                summary { h3 {
                                    "message: "
                                    code { (data.formatted) }
                                } }
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
                                summary { h3 { "breadcrumbs" } }
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
                        ApiEventEntry::Known(KnownEventEntry::Request { data }) => {
                            details {
                                summary {
                                    h3 {
                                        "request: "
                                        code {
                                            (data.method)
                                            " "
                                            (data.url)
                                        }
                                    }
                                }
                                p.help {
                                    "information about the ingoing HTTP request that the crashing code was handling."
                                }

                                @if !data.headers.is_empty() {
                                    h4 { "headers" }

                                    table {
                                        @for (key, value) in data.headers {
                                            tr {
                                                td { code { (key) } }
                                                td { code { (value) } }
                                            }
                                        }
                                    }
                                }

                                @if !data.cookies.is_empty() {
                                    h4 { "cookies" }

                                    table {
                                        @for (key, value) in data.cookies {
                                            tr {
                                                td { code { (key) } }
                                                td { code { (value) } }
                                            }
                                        }
                                    }
                                }

                                @if !data.env.is_empty() {
                                    h4 { "env" }

                                    table {
                                        @for (key, value) in data.env {
                                            tr {
                                                td { code { (key) } }
                                                td { code { (value) } }
                                            }
                                        }
                                    }
                                }

                                @if let Some(ref data) = data.data {
                                    h3 { "body data" }
                                    pre { (data) }
                                }
                            }
                        }
                        ApiEventEntry::Other { ty, attributes } => {
                            details {
                                summary { h3 { code { (ty) } } }

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
                    summary { h3 { "tags" } }

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
            }
        },
    );

    Ok(Html(body))
}

fn render_stacktrace(stacktrace: &Stacktrace) -> Markup {
    html! {
        i { "most recent (crashing frame) to least recent (main function)" }

        style {
            (PreEscaped(r#"
            :scope {
                .system-frame {
                    font-style: italic;
                    display: none;
                }

                .show-system-frames-chk:has(:checked) + ul > .system-frame {
                    display: block;
                }

                .stacktrace > li {
                    font-size: 0.7em;
                    list-style: none;
                }
            }
            "#))
        }

        label.show-system-frames-chk {
            input type="checkbox" switch="";
            "show system frames"
        }

        ul.stacktrace {
            @for frame in stacktrace.frames.iter().rev() {
                li.system-frame[!frame.in_app] {
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

/// the status as sent from the frontend
#[derive(Deserialize, Copy, Clone)]
#[serde(rename_all = "snake_case")]
pub enum StatusParam {
    Unresolved,
    Resolved,
    ResolvedInNextRelease,
    ArchivedUntilEscalating,
    ArchivedForever,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ApiUpdate {
    status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    substatus: Option<String>,
    status_details: BTreeMap<String, bool>,
}

#[derive(Deserialize)]
pub struct UpdateParams {
    status: StatusParam,
}

impl UpdateParams {
    /// Convert to the structure that our API expects for status updates.
    fn to_api(self) -> ApiUpdate {
        match self.status {
            StatusParam::Unresolved => ApiUpdate {
                status: "unresolved".to_string(),
                ..Default::default()
            },

            // resolved
            StatusParam::Resolved => ApiUpdate {
                status: "resolved".to_string(),
                ..Default::default()
            },
            StatusParam::ResolvedInNextRelease => ApiUpdate {
                status: "resolved".to_string(),
                status_details: [("inNextRelease".to_string(), true)].into_iter().collect(),
                ..Default::default()
            },

            // archived
            // i guess somebody decided statusDetails is no longer good and just started adding
            // substatus?
            StatusParam::ArchivedUntilEscalating => ApiUpdate {
                status: "ignored".to_string(),
                substatus: Some("archived_until_escalating".to_string()),
                ..Default::default()
            },
            StatusParam::ArchivedForever => ApiUpdate {
                status: "ignored".to_string(),
                substatus: Some("archived_forever".to_string()),
                ..Default::default()
            },
        }
    }
}

pub async fn update_issue_details(
    IssueDetails {
        org,
        proj,
        issue_id,
    }: IssueDetails,
    HxRequest(is_hx): HxRequest,
    token: SentryToken,
    Form(params): Form<UpdateParams>,
) -> Result<impl IntoResponse, Error> {
    let client = token.client()?;

    let api_update = params.to_api();

    client
        .put(format!(
            // XXX: outdated docs: https://docs.sentry.io/api/events/update-an-issue/
            "https://sentry.io/api/0/organizations/{org}/issues/{issue_id}/"
        ))
        .json(&api_update)
        .send()
        .await?
        .error_for_status()?;

    if is_hx {
        Ok(Html(render_button_status(&api_update.status)).into_response())
    } else {
        Ok(Redirect::to(
            &IssueDetails {
                org,
                proj,
                issue_id,
            }
            .to_string(),
        )
        .into_response())
    }
}

fn render_button_status(status: &str) -> Markup {
    let default_form = |content| {
        html! {
            form
                method="post"
                action=""
                hx-post=""
                hx-target="#issue-status"
                hx-select="#issue-status"
                hx-swap="show:none"
                onsubmit="event.submitter.setAttribute('aria-busy', 'true')" {

                (content)
            }
        }
    };

    html! {
        div id="issue-status" {
            @match status {
                "unresolved" => div.grid {
                    (detail_button_fixes())

                    (default_form(html! {
                        details.dropdown data-tooltip="change status to archived/ignored" {

                            summary.outline.secondary role="button" {
                                "archive"
                            }

                            ul {

                            li { button.secondary type="submit" name="status" value="archived_until_escalating" {
                                "until escalating"
                            } }
                            li { button.outline.secondary type="submit" name="status" value="archived_forever" {
                                "forever"
                            } }

                            }
                        }
                    }))

                    (default_form(html! {
                        details.dropdown data-tooltip="change status to resolved" {
                            summary.outline role="button" {
                                "resolve"
                            }

                            ul {

                            li { button type="submit" name="status" value="resolved" {
                                "globally"
                            } }

                            li { button.outline type="submit" name="status" value="resolved_in_next_release" {
                                "in next release"
                            } }

                            }
                        }
                    }))
                },
                "resolved" => (default_form(html! {
                    (tooltip("change status to unresolved", html! {
                        button type="submit" name="status" value="unresolved" title="issue is resolved. click to unresolve." {
                            "resolved"
                        }
                    }))
                })),
                "ignored" => (default_form(html! {
                    (tooltip("change status to unresolved", html! {
                        button.secondary type="submit" name="status" value="unresolved" title="issue is archived/ignored. click to unresolve." {
                            "archived"
                        }
                    }))
                })),
                x => "unknown status: "(x)
            }
        }
    }
}

// workaround to give tooltips to elements that can't have tooltips.
// inputs can't have tooltips in picocss (due to limitations of ::before)
// everything else can't have tooltips and loading indicators at the same time
fn tooltip(tooltip: &str, content: Markup) -> Markup {
    html! {
        div data-tooltip=(tooltip) style="border-bottom: none; cursor: inherit" {
            (content)
        }
    }
}

// a bunch of picocss bugfixes for a button dropdown
fn detail_button_fixes() -> Markup {
    html! {
        style {
            r#"
            :scope {
                /* disagree with the decision to left-align button text if it's a dropdown. the
                 * next two rules fix that */
                details summary[role=button] {
                    text-align: center;
                }

                details summary[role=button]::after {
                    margin-left: -1rem;
                }

                /* bug in picocss: tooltips on detail elements show the help cursor */
                details[data-tooltip] {
                    cursor: inherit;
                    border-bottom: none;
                }

                /* some weird padding issue */
                details > ul > li > button {
                    margin-bottom: 0;
                }

                /* shift dropdown to be anchored on the right, because the buttons are already
                 * touching the edge of the screen */
                details.dropdown[open] summary + ul {
                    left: unset;
                    right: 0;
                }
            }
            "#
        }
    }
}
