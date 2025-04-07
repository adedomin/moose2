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

use super::{CSRF_COOKIE, LOGIN_COOKIE, MooseWebData, REDIR_COOKIE, get_login};
use crate::{model::author::Author, web_handlers::JSON_TYPE};
use axum::{
    Form, Router,
    extract::{Query, State},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use http::{StatusCode, header::LOCATION};
use oauth2::{
    AuthorizationCode, CsrfToken, HttpClientError, RequestTokenError, StandardErrorResponse,
    TokenResponse, basic::BasicErrorResponseType,
};
use serde::{Deserialize, Serialize};
use tower_cookies::{Cookie, Cookies, Key};

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

    #[error("Could not get CSRF or Login.")]
    SessionGet,

    #[error("Could not set CSRF or Login.")]
    SessionSet,
}

impl IntoResponse for AuthApiError {
    fn into_response(self) -> Response {
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(self.to_string().into())
            .unwrap()
    }
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

async fn login(
    State(auth_client): State<MooseWebData>,
    session: Cookies,
    Form(query): Form<LogInOutRedir>,
) -> Response {
    if let Some(oauth2_client) = &auth_client.oauth2_client {
        if let Some(author) = get_login(&session, &auth_client.cookie_key) {
            return Response::builder()
                .body(format!("Already logged in as: {author:?}").into())
                .unwrap();
        }

        let (authorize_url, csrf_state) =
            oauth2_client.oa.authorize_url(CsrfToken::new_random).url();

        session.add(Cookie::new(CSRF_COOKIE, csrf_state.into_secret()));
        session.add(Cookie::new(
            REDIR_COOKIE,
            serde_json::to_string(&query).unwrap(),
        ));

        Response::builder()
            .status(StatusCode::FOUND)
            .header(LOCATION, authorize_url.to_string())
            .body(().into())
            .unwrap()
    } else {
        Response::builder()
            .status(StatusCode::NOT_IMPLEMENTED)
            .body("Authentication is disabled; the admin has to add an OAuth2 provider.".into())
            .unwrap()
    }
}

fn get_csrf(c: &Cookies, k: &Key) -> Result<String, AuthApiError> {
    match c.private(k).get(CSRF_COOKIE) {
        Some(c) => Ok(c.value().to_string()),
        None => Err(AuthApiError::SessionGet),
    }
}

async fn auth(
    State(auth_client): State<MooseWebData>,
    session: Cookies,
    Query(AuthRequest { code, state }): Query<AuthRequest>,
) -> Result<Response, AuthApiError> {
    if let Some(oauth2_client) = &auth_client.oauth2_client {
        let code = AuthorizationCode::new(code.clone());
        let csrf_val = CsrfToken::new(state.clone());

        let csrf_tok = CsrfToken::new(get_csrf(&session, &auth_client.cookie_key)?);

        if csrf_tok.secret() != csrf_val.secret() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body("No CSRF".to_owned().into())
                .unwrap());
        }

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
            session.add(Cookie::new(
                LOGIN_COOKIE,
                serde_json::to_string(&Author::Oauth2(login_name)).unwrap(),
            ));
            Ok(Response::builder().body(html.into()).unwrap())
        }
        #[cfg(not(debug_assertions))]
        {
            session.add(Cookie::new(
                LOGIN_COOKIE,
                serde_json::to_string(&Author::Oauth2(login_name)).unwrap(),
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

async fn logged_in(State(webdata): State<MooseWebData>, session: Cookies) -> Response {
    let body = match get_login(&session, &webdata.cookie_key) {
        Some(Author::Oauth2(username)) => serde_json::to_vec(&username).unwrap(),
        Some(Author::Anonymous) => {
            // how?
            session.remove(Cookie::new(LOGIN_COOKIE, ""));
            b"null".to_vec()
        }
        _ => b"null".to_vec(),
    };
    Response::builder()
        .header(JSON_TYPE.0, JSON_TYPE.1)
        .body(body.into())
        .unwrap()
}

async fn logout(
    session: Cookies,
    Form(LogInOutRedir { redirect }): Form<LogInOutRedir>,
) -> impl IntoResponse {
    let redir = redirect.unwrap_or_else(|| "/".to_owned());
    session.remove(Cookie::new(LOGIN_COOKIE, ""));
    Redirect::to(&redir)
}

pub fn routes() -> Router<MooseWebData> {
    Router::new()
        .route("/login", get(login).post(login))
        .route("/auth", get(auth))
        .route("/login/username", post(logged_in))
        .route("/logout", post(logout))
}
