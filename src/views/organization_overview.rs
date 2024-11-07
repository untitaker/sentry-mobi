use axum::debug_handler;
use axum::response::IntoResponse;
use maud::html;
use serde::Deserialize;

use crate::views::helpers::{wrap_admin_template, LayoutOptions, REGION_DOMAINS};
use crate::{Error, SentryToken};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiOrganization {
    name: String,
    slug: String,
    links: Links,
    #[serde(default)]
    is_bookmarked: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Links {
    #[serde(default)]
    region_url: String,
}

impl ApiOrganization {
    fn region_domain(&self) -> &str {
        self.links.region_url.strip_prefix("https://").unwrap_or("")
    }
}

#[debug_handler]
pub async fn organization_overview(token: SentryToken) -> Result<impl IntoResponse, Error> {
    let client = token.client()?;
    let mut response = Vec::new();

    for domain in REGION_DOMAINS {
        let region_response: Vec<ApiOrganization> = client
            .get(format!("https://{domain}/api/0/organizations/"))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        response.extend(region_response);
    }

    response.sort_by_key(|o| !o.is_bookmarked);

    let body = wrap_admin_template(
        LayoutOptions {
            title: "organizations".to_owned(),
            ..Default::default()
        },
        html! {
            h2 { "organizations" }

            ul {
                @for org in response {
                    li {
                        a preload="mouseover" href=(format!("/{}", org.slug)) {
                            (org.name)
                        }

                        @if org.is_bookmarked {
                            span title="bookmarked" { "📌" }
                        }

                        small {
                            " (" (org.region_domain()) "/" (org.slug) ")"
                        }
                    }
                }
            }
        },
    );

    let headers = [("Cache-control", "private, max-age=300")];

    Ok((headers, body))
}
