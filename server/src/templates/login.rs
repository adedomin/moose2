use maud::{DOCTYPE, Markup, html};

use crate::templates::{header, navbar};

fn alias_input(alias: Option<&str>) -> Markup {
    // TODO: consider using pattern attribute? https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Attributes/pattern
    html! {
        input #alias name="alias" type="text" maxlength="39" placeholder="Alias" value=(alias.unwrap_or(""));
    }
}

pub fn login_choice(
    _is_gh_enabled: bool,
    alias: Option<&str>,
    err_msg: Option<&'static str>,
) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            (header("Login Choices", "/public/login/login.css"))
            body {
                .divider {
                    (navbar(false, None))
                    .center-me {
                        a #gh-login .btn href="/login/gh" { "Login with GitHub" }
                        p .choice {"OR"}
                        form #alias-form method="post" action="/login/alias" {
                            .btn-grp {
                                (alias_input(alias))
                                input .btn #submit type="submit" value="Submit";
                            }
                            @if let Some(err_msg) = err_msg {
                                p.err-text { (err_msg) }
                            }
                        }
                    }
                }
            }
        }
    }
}
