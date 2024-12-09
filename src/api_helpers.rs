use regex::Regex;
use std::sync::LazyLock;

// one day we will be be able to use typed headers for pagination:
// https://github.com/hyperium/headers/pull/113
// https://github.com/XAMPPRocky/octocrab/issues/110#issuecomment-1458449662
static LINK_REL_NEXT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<(.+?)>; rel="next"; results="true""#).unwrap());

pub fn get_next_link(res: &reqwest::Response) -> Option<String> {
    let link_header = res.headers().get("Link")?.to_str().ok()?;
    Some(LINK_REL_NEXT_RE.captures(link_header)?[1].to_owned())
}
