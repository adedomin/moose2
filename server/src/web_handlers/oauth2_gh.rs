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

use std::borrow::Cow;

use super::{ApiError, CSRF_COOKIE, LOGIN_COOKIE, MooseWebData, REDIR_COOKIE};
use crate::{
    model::{author::Author, secure_cookies::SecureCookies},
    templates::login::login_choice,
    web_handlers::JSON_TYPE,
};
use axum::{
    Form, Router,
    extract::{Query, State},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use http::{StatusCode, header::LOCATION};
use oauth2::{
    AuthorizationCode, CsrfToken, HttpClientError, RequestTokenError, StandardErrorResponse,
    TokenResponse, basic::BasicErrorResponseType,
};
use serde::{Deserialize, Serialize};
use tower_cookies::{Cookie, Cookies, cookie::Expiration};

#[derive(Deserialize)]
struct GithubUserApi {
    pub login: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthApiError {
    #[error("oauth2 api failure: {0}")]
    Oauth2Err(
        #[from]
        oauth2::RequestTokenError<reqwest::Error, StandardErrorResponse<BasicErrorResponseType>>,
    ),
    #[error("Client error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("[exchange_token] Client error: {0}")]
    CodeTokenHttp(
        #[from]
        RequestTokenError<
            HttpClientError<reqwest::Error>,
            StandardErrorResponse<BasicErrorResponseType>,
        >,
    ),

    #[error("Could not get CSRF from cookie.")]
    SessionGet,

    #[error("CSRF Mismatch")]
    MismatchedCSRF,
}

impl IntoResponse for AuthApiError {
    fn into_response(self) -> Response {
        ApiError::new(self).into_response()
    }
}

#[derive(Deserialize)]
pub struct AliasLogIn {
    alias: String,
}

#[derive(Deserialize)]
pub struct AuthRequest {
    code: String,
    state: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct LogInOutRedir {
    redirect: Option<String>,
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

    let html = login_choice(auth_client.oauth2_client.is_some(), None, None).into_string();
    Ok(Html(html))
}

async fn login_gh(
    State(auth_client): State<MooseWebData>,
    session: SecureCookies,
) -> Result<Response, ApiError> {
    if let Some(oauth2_client) = &auth_client.oauth2_client {
        let session = session.get_inner(&auth_client.cookie_key);
        if let Some(author) = session
            .get(LOGIN_COOKIE)
            .and_then(|c| serde_json::from_str::<Author>(c.value()).ok())
        {
            return Err(ApiError::new_ok(format!(
                "Already logged in as: {author:?}"
            )));
        }

        let (authorize_url, csrf_state) =
            oauth2_client.oa.authorize_url(CsrfToken::new_random).url();

        session.add(new_cookie(CSRF_COOKIE, csrf_state.into_secret()));

        Ok(Response::builder()
            .status(StatusCode::FOUND)
            .header(LOCATION, authorize_url.to_string())
            .body(().into())
            .unwrap())
    } else {
        Err(ApiError::new_with_status(
            StatusCode::NOT_IMPLEMENTED,
            "Authentication is disabled; the admin has to add an OAuth2 provider.",
        ))
    }
}

async fn login_alias(
    State(auth_client): State<MooseWebData>,
    session: SecureCookies,
    Form(AliasLogIn { alias }): Form<AliasLogIn>,
) -> Result<Redirect, Html<String>> {
    let session = session.get_inner(&auth_client.cookie_key);
    let author = Author::new_alias(alias.clone()).map_err(|err_msg| {
        let html = login_choice(
            auth_client.oauth2_client.is_some(),
            Some(&alias),
            Some(err_msg),
        )
        .into_string();
        Html(html)
    })?;
    let redirect = session
        .get(REDIR_COOKIE)
        .and_then(|c| serde_json::from_str::<LogInOutRedir>(c.value()).ok())
        .unwrap_or(LogInOutRedir { redirect: None })
        .redirect
        .unwrap_or_else(|| "/".to_owned());
    session.remove(new_cookie(REDIR_COOKIE, ""));
    session.add(new_cookie(
        LOGIN_COOKIE,
        serde_json::to_string(&author).unwrap(),
    ));

    Ok(Redirect::to(&redirect))
}

async fn auth(
    State(auth_client): State<MooseWebData>,
    session: SecureCookies,
    Query(AuthRequest { code, state }): Query<AuthRequest>,
) -> Result<Response, AuthApiError> {
    if let Some(oauth2_client) = &auth_client.oauth2_client {
        let session = session.get_inner(&auth_client.cookie_key);
        let csrf_val = CsrfToken::new(state.clone());
        let csrf_cookie = session
            .get(CSRF_COOKIE)
            .ok_or(AuthApiError::SessionGet)?
            .value()
            .to_string();
        let csrf_tok = CsrfToken::new(csrf_cookie);

        if csrf_tok.secret() != csrf_val.secret() {
            return Err(AuthApiError::MismatchedCSRF);
        }

        let code = AuthorizationCode::new(code.clone());
        let token = oauth2_client
            .oa
            .exchange_code(code)
            .request_async(&oauth2_client.web)
            .await?
            .access_token()
            .clone();

        // now get user's Login
        let token_secret = token.secret();
        let res = oauth2_client
            .web
            .get("https://api.github.com/user")
            .header("Authorization", format!("BEARER {token_secret}"))
            .send()
            .await?
            .json::<GithubUserApi>()
            .await?;

        let login_name = res.login;
        let redirect = session
            .get(REDIR_COOKIE)
            .and_then(|c| serde_json::from_str::<LogInOutRedir>(c.value()).ok())
            .unwrap_or(LogInOutRedir { redirect: None })
            .redirect
            .unwrap_or_else(|| "/".to_owned());

        session.remove(new_cookie(CSRF_COOKIE, ""));
        session.remove(new_cookie(REDIR_COOKIE, ""));
        #[cfg(debug_assertions)]
        {
            let html = format!(
                r#"<html>
                <head><title>OAuth2 Test</title></head>
                <body>
                    <p>Github returned the following info:</p>
                    <pre>token: {token_secret}</pre>
                    <pre>login: {login_name}</pre>
                    <br>
                    <p>User wanted to redirect to: {redirect}</p>
                </body>
            </html>"#
            );
            session.add(new_cookie(
                LOGIN_COOKIE,
                serde_json::to_string(&Author::GitHub(login_name)).unwrap(),
            ));
            Ok(Response::builder().body(html.into()).unwrap())
        }
        #[cfg(not(debug_assertions))]
        {
            session.add(new_cookie(
                LOGIN_COOKIE,
                serde_json::to_string(&Author::GitHub(login_name)).unwrap(),
            ));
            Ok(Response::builder()
                .status(StatusCode::SEE_OTHER)
                .header(LOCATION, redirect)
                .body(().into())
                .unwrap())
        }
    } else {
        Ok(Response::builder()
            .status(StatusCode::NOT_IMPLEMENTED)
            .body(
                "Authentication is disabled; the admin has to add an OAuth2 provider."
                    .to_owned()
                    .into(),
            )
            .unwrap())
    }
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
    let redir = redirect.unwrap_or_else(|| "/".to_owned());
    session.remove(new_cookie(LOGIN_COOKIE, ""));
    Redirect::to(&redir)
}

pub fn routes() -> Router<MooseWebData> {
    Router::new()
        .route("/login", get(login_get).post(login))
        .route("/login/alias", post(login_alias))
        .route("/login/gh", get(login_gh))
        .route("/auth", get(auth))
        .route("/login/username", post(logged_in))
        .route("/logout", post(logout))
}
