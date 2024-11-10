use jiff::{SpanRound, Timestamp, Unit};
use maud::Markup;

pub use maud::html;

#[derive(Default)]
pub struct LayoutOptions {
    pub title: String,
    pub header: Option<Markup>,
}

pub fn wrap_admin_template(mut opt: LayoutOptions, content: Markup) -> Markup {
    opt.header = Some(html! {
        form method="post" action="/auth/logout" {
            // https://github.com/picocss/pico/issues/496
            input.outline.secondary type="submit" value="Logout" style="margin: 0; width: auto; float: right";
        }
    });
    wrap_template(opt, content)
}

pub fn wrap_template(opt: LayoutOptions, content: Markup) -> Markup {
    html! {
        (maud::DOCTYPE)
        head {
            title {
                @if opt.title.is_empty() {
                    "sentry.mobi"
                } @else {
                    (opt.title) " - sentry.mobi"
                }
            }
            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1";
            meta name="color-scheme" content="light dark";
            link rel="stylesheet" href="/style.css";

            script src="/htmx.js" {}
            script src="/htmx.preload.js" {}
        }

        body hx-boost="true" hx-indicator="#spinner" hx-ext="preload" {
            header.container {
                div.grid {
                    div {
                        h1 {
                            a.secondary preload="mouseover" href="/" { "sentry.mobi" }
                            " "
                                small.htmx-indicator id="spinner" aria-busy="true" {
                                    span style="display: none" {
                                        "is loading"
                                    }
                                }
                        }
                    }

                    @if let Some(header) = opt.header {
                        (header)
                    }
                }
            }

            main.container {
                (content)
            }
        }
    }
}

pub const REGION_DOMAINS: &[&str] = &["us.sentry.io", "de.sentry.io"];

pub fn print_relative_time(ts: Timestamp) -> Markup {
    html! {
        time datetime=(ts) title=(ts) data-tooltip=(ts) {
            @if let Ok(x) = ts.until(Timestamp::now()) {
                @let x = x.round(SpanRound::new().largest(Unit::Day)).unwrap_or(x);
                @if x.get_days() > 0 {
                    (x.get_days()) " days "
                }
                @if x.get_hours() > 0 {
                    (x.get_hours()) " hours "
                }
                @if x.get_minutes() > 0 {
                    (x.get_minutes()) " minutes "
                }
                @if x.get_seconds() > 0 {
                    (x.get_seconds()) " seconds "
                }
            } @else {
                (ts)
            }
        }
    }
}

pub fn breadcrumbs(url: &str, h2_content: Markup) -> Markup {
    html! {
        div style="margin-bottom: var(--pico-typography-spacing-vertical)" {
            h2 style="font-size: 1em; display: inline" {
                (h2_content)
            }

            " "

            a.secondary href=(url) { "open in sentry" }
        }
    }
}
