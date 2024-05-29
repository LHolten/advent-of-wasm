use std::fs;

use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::Redirect;
use axum_extra::extract::{cookie::Cookie, CookieJar};
use oauth2::reqwest::async_http_client;
use oauth2::TokenResponse;
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, TokenUrl,
};
use rust_query::value::UnixEpoch;
use serde::Deserialize;

use crate::async_sqlite::DB;
use crate::db::GithubId;
use crate::migration::UserDummy;

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
) -> Result<(CookieJar, Redirect), String> {
    let code = AuthorizationCode::new(auth.code);
    if auth.state != jar.get("state").unwrap().value() {
        return Err("state does not match".into());
    }

    let client = make_client();
    let github_token = client
        .exchange_code(code)
        .request_async(async_http_client)
        .await
        .map_err(|_| "can not get token from github")?;

    jar = jar.add(Cookie::new(
        "access_token",
        github_token.access_token().secret().to_string(),
    ));
    jar = jar.remove(Cookie::from("state"));
    let github_id = safe_login(&mut jar).await?;
    jar = jar.add(Cookie::new("github_id", (github_id.0 as u64).to_string()));

    Ok((jar, Redirect::to("/problem/decimal")))
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

pub async fn fast_login(jar: &CookieJar) -> Option<GithubId> {
    let github_id = jar.get("github_id")?;
    let github_id = github_id.value().parse::<u64>().ok()?;
    Some(GithubId(github_id as i64))
}

pub async fn safe_login(jar: &mut CookieJar) -> Result<GithubId, String> {
    let access_token = jar.get("access_token").ok_or("not logged in")?;

    let response = reqwest::Client::builder()
        .user_agent("wasm-bench")
        .build()
        .unwrap()
        .get("https://api.github.com/user")
        .bearer_auth(access_token.value())
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|_| "error connecting to github")?
        .error_for_status()
        .map_err(|_| "could not get github info, try logging in again")?;
    let text = response.text().await.map_err(|_| "github response error")?;
    println!("{}", text);

    let val: serde_json::Value = serde_json::from_str(&text).unwrap();
    let github_id = val.get("id").unwrap().as_u64().unwrap();
    let github_login = val.get("login").unwrap().as_str().unwrap().to_owned();

    DB.call(move |conn| {
        conn.new_query(|q| {
            q.insert(UserDummy {
                github_id: github_id as i64,
                github_login: github_login.as_str(),
                timestamp: UnixEpoch,
            })
        });
    })
    .await;

    Ok(GithubId(github_id as i64))
}
