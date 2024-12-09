use axum::extract::Query;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::api_helpers::get_next_link;
use crate::views::helpers::html;
use crate::views::helpers::{
    breadcrumbs, paginated_response, wrap_admin_template, Html, LayoutOptions,
};
use crate::{Error, SentryToken};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiProject {
    name: String,
    slug: String,
    #[serde(default)]
    is_bookmarked: bool,
}

#[derive(Deserialize)]
pub struct Params {
    #[serde(default)]
    next_link: Option<String>,
}

pub async fn organization_details(
    route: crate::routes::OrganizationDetails,
    token: SentryToken,
    Query(params): Query<Params>,
) -> Result<impl IntoResponse, Error> {
    let org = route.org;

    let client = token.client()?;
    let http_response =
        client
            .get(params.next_link.unwrap_or_else(|| {
                format!("https://sentry.io/api/0/organizations/{org}/projects/")
            }))
            .send()
            .await?
            .error_for_status()?;
    let next_link = get_next_link(&http_response);
    let mut response: Vec<ApiProject> = http_response.json().await?;

    response.sort_by_key(|p| !p.is_bookmarked);

    let body = wrap_admin_template(
        LayoutOptions {
            title: org.clone(),
            ..Default::default()
        },
        html! {
            (breadcrumbs(&format!("https://sentry.io/{}", org), html! {
                (org) ": projects"
            }))

            (paginated_response(next_link.as_deref(), html! {
                ul {
                    @for project in response {
                        li {
                            a preload="mouseover" href=(
                                crate::routes::ProjectDetails { org: org.clone(), proj: project.slug.clone() }
                            ) {
                                (project.name)
                            }

                            @if project.is_bookmarked {
                                span title="bookmarked" { "📌" }
                            }

                            small {
                                " (" (org) "/" (project.slug) ")"
                            }
                        }
                    }
                }
            }))
        },
    );

    Ok(Html(body))
}
