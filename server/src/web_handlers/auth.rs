/* Copyright (C) 2024  Anthony DeDominic
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::{array, borrow::Cow};

use super::{ApiError, HtmlError, LOGIN_COOKIE, MooseWebData, REDIR_COOKIE};
use crate::{
    db::MooseDB,
    model::{
        author::{Author, User},
        secret::{InviteSecret, Invited},
        secure_cookies::SecureCookies,
    },
    templates::login::login_choice,
    web_handlers::JSON_TYPE,
};
use axum::{
    Form, Router,
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use tower_cookies::{Cookie, Cookies, cookie::Expiration};

#[derive(Deserialize)]
struct LogIn {
    user: String,
    #[serde(default)]
    pass: String,
    #[serde(default)]
    newpass: String,
}

#[derive(Serialize, Deserialize)]
struct LogInOutRedir {
    #[serde(default = "default_redir")]
    redirect: String,
}

fn default_redir() -> String {
    "/".to_string()
}

impl Default for LogInOutRedir {
    fn default() -> Self {
        Self {
            redirect: default_redir(),
        }
    }
}

fn new_cookie<'a, K, V>(key: K, value: V) -> Cookie<'a>
where
    K: Into<Cow<'a, str>>,
    V: Into<Cow<'a, str>>,
{
    Cookie::build((key, value))
        .http_only(true)
        .secure(true)
        .path("/")
        .expires(Expiration::Session)
        .build()
}

// Form type deserializes GET Queries for some reason.
async fn login_get(
    auth_client: State<MooseWebData>,
    session: SecureCookies,
) -> Result<Html<String>, ApiError> {
    login(auth_client, session, Form(LogInOutRedir::default())).await
}

async fn login(
    State(auth_client): State<MooseWebData>,
    session: SecureCookies,
    Form(query): Form<LogInOutRedir>,
) -> Result<Html<String>, ApiError> {
    let session = session.get_inner(&auth_client.cookie_key);
    if let Some(author) = session
        .get(LOGIN_COOKIE)
        .and_then(|c| serde_json::from_str::<Author>(c.value()).ok())
    {
        return Err(ApiError::new_ok(format!(
            "Already logged in as: {author:?}"
        )));
    }

    session.add(new_cookie(
        REDIR_COOKIE,
        serde_json::to_string(&query).unwrap(),
    ));

    let html = login_choice(None, None).into_string();
    Ok(Html(html))
}

fn parse_login_form_fields(user: &str, pass: &str) -> Result<(User, InviteSecret), &'static str> {
    Ok((user.try_into()?, pass.try_into()?))
}
fn parse_change_login_form_fields(
    user: &str,
    pass: &str,
    newpass: &str,
) -> Result<(User, InviteSecret, InviteSecret), &'static str> {
    Ok((user.try_into()?, pass.try_into()?, newpass.try_into()?))
}

async fn login_submit(
    State(webdata): State<MooseWebData>,
    session: SecureCookies,
    Form(LogIn {
        user,
        pass,
        newpass,
    }): Form<LogIn>,
) -> Result<Redirect, HtmlError> {
    macro_rules! handle_auth {
        ($query:expr) => {
            $query
                .await
                .map_err(|e| {
                    log::error!("Failed to check user ( {user} ) password: {e}");
                    HtmlError::new(
                        login_choice(Some(&user), Some("Database Failure.")).into_string(),
                    )
                })
                .and_then(|res| {
                    if res {
                        Ok(())
                    } else {
                        Err(HtmlError::new_auth_req(
                            login_choice(Some(&user), Some("Login failed.")).into_string(),
                        ))
                    }
                })
        };
    }

    let author = if !newpass.is_empty() {
        if pass.is_empty() {
            return Err(HtmlError::new_auth_req(
                login_choice(Some(&user), Some("Can't change an empty password.")).into_string(),
            ));
        }
        let (user_parsed, old_hash, new_hash) =
            parse_change_login_form_fields(&user, &pass, &newpass).map_err(|e| {
                HtmlError::new_auth_req(login_choice(Some(&user), Some(e)).into_string())
            })?;
        let author = Author::from(user_parsed.clone());
        handle_auth!(webdata.db.update_user_hash(user_parsed, old_hash, new_hash))?;
        author
    } else if !pass.is_empty() {
        let (user_parsed, hash) = parse_login_form_fields(&user, &pass).map_err(|e| {
            HtmlError::new_auth_req(login_choice(Some(&user), Some(e)).into_string())
        })?;
        let author = Author::from(user_parsed.clone());
        handle_auth!(webdata.db.check_user_hash(user_parsed, hash))?;
        author
    } else {
        Author::new_alias(user.clone()).map_err(|err_msg| {
            let html = login_choice(Some(&user), Some(err_msg)).into_string();
            HtmlError::new_auth_req(html)
        })?
    };

    let session = session.get_inner(&webdata.cookie_key);
    let redirect = session
        .get(REDIR_COOKIE)
        .and_then(|c| serde_json::from_str::<LogInOutRedir>(c.value()).ok())
        .unwrap_or_default()
        .redirect;
    session.remove(new_cookie(REDIR_COOKIE, ""));
    session.add(new_cookie(
        LOGIN_COOKIE,
        serde_json::to_string(&author).unwrap(),
    ));

    Ok(Redirect::to(&redirect))
}

const NULL_RESP: &[u8] = b"null";

async fn logged_in(username: Author) -> Response {
    let body = username
        .displayable()
        .and_then(|username| serde_json::to_vec(&username).ok())
        .unwrap_or_else(|| NULL_RESP.to_vec());
    Response::builder()
        .header(JSON_TYPE.0, JSON_TYPE.1)
        .body(body.into())
        .unwrap()
}

async fn logout(
    session: Cookies, // we are only deleting cookies here so it doesn't matter if it is encrypted.
    Form(LogInOutRedir { redirect }): Form<LogInOutRedir>,
) -> impl IntoResponse {
    session.remove(new_cookie(LOGIN_COOKIE, ""));
    Redirect::to(&redirect)
}

#[derive(Deserialize)]
struct Invite {
    user: User,
}

// adding this code to invite breaks the method router wrapper type in axum!?
pub fn gen_rand_ascii_pass_plus_hash<const N: usize>() -> (String, InviteSecret) {
    let mut rng = rand::thread_rng();
    let pass: [char; N] = array::from_fn(|_| rng.r#gen_range(char::from(33)..=char::from(126)));
    let pass = pass.iter().collect::<String>();
    let hash = InviteSecret::new(&pass);
    (pass, hash)
}

async fn invite(
    State(webdata): State<MooseWebData>,
    Invited: Invited,
    Form(Invite { user }): Form<Invite>,
) -> ApiError {
    let (rand_pass, hash) = gen_rand_ascii_pass_plus_hash::<16>();
    match webdata.db.invite_user(user, hash).await {
        Ok(()) => ApiError::new_ok(rand_pass),
        Err(e) => ApiError::new(e),
    }
}

pub fn routes() -> Router<MooseWebData> {
    Router::new()
        .route("/login", get(login_get).post(login))
        .route("/login/submit", get(login_get).post(login_submit))
        .route("/login/username", post(logged_in))
        .route("/logout", post(logout))
        .route("/invite", post(invite))
}
