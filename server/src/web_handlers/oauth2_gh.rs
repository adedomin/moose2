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

use super::MooseWebData;
use crate::{model::author::Author, web_handlers::JSON_TYPE};
use actix_session::Session;
use actix_web::{get, http::header, post, web, HttpResponse};
use oauth2::{
    basic::BasicErrorResponseType, AuthorizationCode, CsrfToken, StandardErrorResponse,
    TokenResponse,
};
use serde::{Deserialize, Serialize};

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

    #[error("Could not get CSRF or Login.")]
    SessionGet(#[from] actix_session::SessionGetError),

    #[error("Could not set CSRF or Login.")]
    SessionSet(#[from] actix_session::SessionInsertError),
}

impl actix_web::ResponseError for AuthApiError {}

#[derive(Deserialize)]
pub struct AuthRequest {
    code: String,
    state: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct LogInOutRedir {
    redirect: Option<String>,
}

async fn oa2_reqwest(request: oauth2::HttpRequest) -> Result<oauth2::HttpResponse, reqwest::Error> {
    let client = {
        let builder = reqwest::Client::builder();
        let builder = builder.redirect(reqwest::redirect::Policy::none());
        builder.build()?
    };

    let mut request_builder = client
        .request(request.method, request.url.as_str())
        .body(request.body);
    for (name, value) in &request.headers {
        request_builder = request_builder.header(name.as_str(), value.as_bytes());
    }
    let request = request_builder.build()?;

    let response = client.execute(request).await?;

    let status_code = response.status();
    let headers = response.headers().to_owned();
    let chunks = response.bytes().await?;
    Ok(oauth2::HttpResponse {
        status_code,
        headers,
        body: chunks.to_vec(),
    })
}

#[get("/login")]
pub async fn login(
    auth_client: MooseWebData,
    session: Session,
) -> Result<HttpResponse, AuthApiError> {
    login_real(auth_client, session, LogInOutRedir::default()).await
}

#[post("/login")]
pub async fn login_post(
    auth_client: MooseWebData,
    session: Session,
    params: web::Form<LogInOutRedir>,
) -> Result<HttpResponse, AuthApiError> {
    login_real(auth_client, session, params.into_inner()).await
}

pub async fn login_real(
    auth_client: MooseWebData,
    session: Session,
    query: LogInOutRedir,
) -> Result<HttpResponse, AuthApiError> {
    if let Some(oauth2_client) = &auth_client.oauth2_client {
        match session.get::<Author>("login") {
            Ok(login_info) => {
                if let Some(author) = login_info {
                    return Ok(HttpResponse::Ok().body(format!("Already logged in as: {author:?}")));
                }
            }
            Err(e) => {
                eprintln!("{e}");
            }
        }

        let (authorize_url, csrf_state) = oauth2_client.authorize_url(CsrfToken::new_random).url();

        session.insert("csrf", csrf_state.secret()).unwrap();
        session.insert("redirect", query).unwrap();

        Ok(HttpResponse::Found()
            .insert_header((header::LOCATION, authorize_url.to_string()))
            .body(()))
    } else {
        Ok(HttpResponse::NotImplemented()
            .body("Authentication is disabled; the admin has to add an OAuth2 provider."))
    }
}

#[get("/auth")]
pub async fn auth(
    auth_client: MooseWebData,
    params: web::Query<AuthRequest>,
    session: Session,
) -> Result<HttpResponse, AuthApiError> {
    if let Some(oauth2_client) = &auth_client.oauth2_client {
        let code = AuthorizationCode::new(params.code.clone());
        let csrf_val = CsrfToken::new(params.state.clone());

        let csrf_tok = CsrfToken::new(session.get("csrf")?.unwrap_or_default());

        if csrf_tok.secret() != csrf_val.secret() {
            return Ok(HttpResponse::BadRequest().body("No CSRF"));
        }

        let token = oauth2_client
            .exchange_code(code)
            .request_async(oa2_reqwest)
            .await?
            .access_token()
            .clone();

        // now get user's Login
        let api_client = reqwest::Client::builder()
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()?;

        let token_secret = token.secret();
        let res = api_client
            .get("https://api.github.com/user")
            .header("Authorization", format!("BEARER {token_secret}"))
            .send()
            .await?
            .json::<GithubUserApi>()
            .await?;

        let login_name = res.login;
        let redirect = session
            .get::<LogInOutRedir>("redirect")?
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
            session.insert("login", Author::Oauth2(login_name))?;
            Ok(HttpResponse::Ok().body(html))
        }
        #[cfg(not(debug_assertions))]
        {
            session.insert("login", Author::Oauth2(login_name))?;
            Ok(HttpResponse::Found()
                .insert_header((header::LOCATION, redirect))
                .finish())
        }
    } else {
        Ok(HttpResponse::NotImplemented()
            .body("Authentication is disabled; the admin has to add an OAuth2 provider."))
    }
}

#[post("/login/username")]
pub async fn logged_in(session: Session) -> HttpResponse {
    match session
        .get::<Author>("login")
        .unwrap_or_default()
        .and_then(|author| std::convert::TryInto::<String>::try_into(author).ok())
    {
        Some(username) => HttpResponse::Ok().insert_header(JSON_TYPE).json(username),
        None => HttpResponse::Ok().insert_header(JSON_TYPE).body("null"),
    }
}

#[post("/logout")]
pub async fn logout(session: Session, params: web::Form<LogInOutRedir>) -> HttpResponse {
    let redir = params
        .into_inner()
        .redirect
        .unwrap_or_else(|| "/".to_owned());
    session.purge();
    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, redir))
        .finish()
}
