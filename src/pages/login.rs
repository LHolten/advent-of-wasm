use std::fs;

use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::Redirect;
use axum_extra::extract::{cookie::Cookie, CookieJar};
use oauth2::reqwest::async_http_client;
use oauth2::TokenResponse;
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl,
    TokenUrl,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Auth {
    code: String,
    state: String,
}

#[derive(Deserialize)]
struct GithubConfig {
    github_client_id: String,
    github_client_secret: String,
}

thread_local! {
    static CONFIG: GithubConfig = {
        let config = fs::read_to_string("config/oauth.toml").expect("could not read oath config");
        toml::from_str(&config).expect("could not parse oauth config")
    }
}

fn make_client() -> BasicClient {
    let github_client_id = CONFIG.with(|x| ClientId::new(x.github_client_id.clone()));
    let github_client_secret = CONFIG.with(|x| ClientSecret::new(x.github_client_secret.clone()));

    let auth_url = AuthUrl::new("https://github.com/login/oauth/authorize".to_string())
        .expect("Invalid authorization endpoint URL");
    let token_url = TokenUrl::new("https://github.com/login/oauth/access_token".to_string())
        .expect("Invalid token endpoint URL");

    // Set up the config for the Github OAuth2 process.

    BasicClient::new(
        github_client_id,
        Some(github_client_secret),
        auth_url,
        Some(token_url),
    )
}

pub async fn redirect(
    Query(auth): Query<Auth>,
    mut jar: CookieJar,
) -> Result<(CookieJar, Redirect), StatusCode> {
    let code = AuthorizationCode::new(auth.code);
    if auth.state != jar.get("state").unwrap().value() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let client = make_client();
    let token_res = client
        .exchange_code(code)
        .request_async(async_http_client)
        .await;

    println!("Github returned the following token:\n{:?}\n", token_res);

    if let Ok(token) = token_res {
        let token_str = token.access_token().secret();
        jar = jar.add(Cookie::new("access_token", token_str.to_owned()));
        jar = jar.remove(Cookie::from("state"));
    }
    Ok((jar, Redirect::to("/problem/parse")))
}

pub async fn login(mut jar: CookieJar) -> Result<(CookieJar, Redirect), StatusCode> {
    let client = make_client();

    // Generate the authorization URL to which we'll redirect the user.
    let (authorize_url, csrf_state) = client
        .authorize_url(CsrfToken::new_random)
        // This example is requesting access to the user's public repos and email.
        .url();
    jar = jar.add(Cookie::new("state", csrf_state.secret().to_owned()));

    Ok((jar, Redirect::to(authorize_url.as_str())))
}
