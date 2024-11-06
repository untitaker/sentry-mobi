use maud::{html, Markup};

pub fn wrap_admin_template(content: Markup) -> Markup {
    wrap_template(html! {
        form method="post" action="/auth/logout" {
            input type="submit" value="Logout";
        }

        (content)
    })
}

pub fn wrap_template(content: Markup) -> Markup {
    html! {
        (maud::DOCTYPE)
        h1 {
            "sentry.mobi"
        }

        (content)
    }
}
