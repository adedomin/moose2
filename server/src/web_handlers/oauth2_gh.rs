use super::MooseWebData;
use crate::{model::author::Author, web_handlers::api::JSON_TYPE};
use actix_session::Session;
use actix_web::{get, http::header, post, web, HttpResponse};
use oauth2::{
    basic::BasicErrorResponseType, AuthorizationCode, CsrfToken, StandardErrorResponse,
    TokenResponse,
};
use serde::Deserialize;

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
    // query: web::Query<LoginRedir>,
) -> Result<HttpResponse, AuthApiError> {
    if let Some(oauth2_client) = &auth_client.oauth2_client {
        match session.get::<Author>("login") {
            Ok(login) => {
                if let Some(author) = login {
                    return Ok(HttpResponse::Ok().body(format!("Already logged in as: {author:?}")));
                }
            }
            Err(e) => {
                eprintln!("{e}");
            }
        }

        let (authorize_url, csrf_state) = oauth2_client.authorize_url(CsrfToken::new_random).url();

        session.insert("csrf", csrf_state.secret()).unwrap();
        // let LoginRedir { redir, debug } = query.into_inner();
        // session.insert("redir", redir).unwrap();
        // session.insert("login_debug", debug).unwrap();

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

        #[cfg(debug_assertions)]
        {
            let html = format!(
                r#"<html>
                <head><title>OAuth2 Test</title></head>
                <body>
                    Github returned the following info:
                    <pre>token: {token_secret}</pre>
                    <pre>login: {login_name}</pre>
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
                .insert_header((header::LOCATION, "/"))
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
pub async fn logout(session: Session) -> HttpResponse {
    session.purge();
    HttpResponse::Ok()
        .insert_header((header::LOCATION, "/"))
        .finish()
}
