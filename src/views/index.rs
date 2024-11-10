use axum::extract::Query;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::routes::OrganizationDetails;
use crate::views::helpers::{
    html, wrap_admin_template, wrap_template, LayoutOptions, REGION_DOMAINS,
};
use crate::{Error, SentryToken};

#[derive(Deserialize)]
pub struct RedirectTo {
    #[serde(default)]
    redirect_to: Option<String>,
}

pub async fn index(
    _: crate::routes::Index,
    token: SentryToken,
    Query(params): Query<RedirectTo>,
) -> Result<impl IntoResponse, Error> {
    if token.token.is_empty() {
        Ok(wrap_template(
            LayoutOptions::default(),
            html! {
                form.login method="post" action="/auth" {
                    @if let Some(redirect_to) = params.redirect_to {
                        input type="hidden" name="redirect_to" value=(redirect_to);
                    }

                    fieldset role="group" {
                        input type="password" name="token" placeholder="your API token";
                        input type="submit" value="Login";
                    }

                    small {
                        "get a user API token from Sentry to view issues"
                    }
                }
            },
        )
        .into_response())
    } else {
        Ok(organization_overview(token).await?.into_response())
    }
}

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

async fn organization_overview(token: SentryToken) -> Result<impl IntoResponse, Error> {
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
                        a preload="mouseover" href=(OrganizationDetails { org: org.slug.clone() }) {
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
