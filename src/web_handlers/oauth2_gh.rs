use crate::config::{self, GitHubOauth2};
use actix_web::{get, http::header, web, HttpResponse};
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    PkceCodeChallenge, TokenUrl,
};
use serde::Deserialize;

#[get("/login")]
pub async fn login() -> HttpResponse {
    let oauth2_client = match &config::get_config().github_oauth2 {
        Some(GitHubOauth2 { id, secret }) => {
            let client_id = ClientId::new(id.to_string());
            let secret = Some(ClientSecret::new(secret.to_string()));
            let auth_url =
                AuthUrl::new("https://github.com/login/oauth/authorize".to_string()).unwrap();
            let token_url = Some(
                TokenUrl::new("https://github.com/login/oauth/access_token".to_string()).unwrap(),
            );
            BasicClient::new(client_id, secret, auth_url, token_url)
        }
        None => return HttpResponse::NotAcceptable().body("Authentication Disabled."),
    };

    let (pkce_code_challenge, _pkce_code_verifier) = PkceCodeChallenge::new_random_sha256();

    let (authorize_url, _csrf_state) = oauth2_client
        .authorize_url(CsrfToken::new_random)
        .set_pkce_challenge(pkce_code_challenge)
        .url();

    HttpResponse::Found()
        .insert_header((header::LOCATION, authorize_url.to_string()))
        .body(())
}

#[derive(Deserialize)]
pub struct AuthRequest {
    code: String,
    state: String,
}

#[get("/auth")]
pub async fn auth(params: web::Query<AuthRequest>) -> HttpResponse {
    let code = AuthorizationCode::new(params.code.clone());
    let state = CsrfToken::new(params.state.clone());

    let html = format!(
        r#"<html>
        <head><title>OAuth2 Test</title></head>
        <body>
            Github returned the following state:
            <pre>{}</pre>
            Github returned the following info:
            <pre>{:?}</pre>
            <pre>{:?}</pre>
        </body>
    </html>"#,
        state.secret(),
        state,
        code,
    );
    HttpResponse::Ok().body(html)
}
