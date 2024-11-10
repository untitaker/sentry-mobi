use crate::views;
use axum::Router;
use axum_extra::routing::{RouterExt, TypedPath};
use serde::Deserialize;

#[derive(TypedPath, Deserialize)]
#[typed_path("/")]
pub struct Index;

#[derive(TypedPath, Deserialize)]
#[typed_path("/auth")]
pub struct Auth;

#[derive(TypedPath, Deserialize)]
#[typed_path("/auth/logout")]
pub struct Logout;

#[derive(TypedPath, Deserialize)]
#[typed_path("/:org")]
pub struct OrganizationDetails {
    pub org: String,
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/:org/:proj")]
pub struct ProjectDetails {
    pub org: String,
    pub proj: String,
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/:org/:proj/issues/:issue_id")]
pub struct IssueDetails {
    pub org: String,
    pub proj: String,
    pub issue_id: String,
}

pub fn get_router() -> Router {
    Router::new()
        .typed_get(views::index::index)
        .typed_post(views::auth::auth)
        .typed_post(views::auth::logout)
        .typed_get(views::organization_details::organization_details)
        .typed_get(views::project_details::project_details)
        .typed_get(views::issue_details::issue_details)
        .typed_post(views::issue_details::update_issue_details)
}
