use std::collections::HashMap;
use std::sync::Arc;

use base64::Engine as _;
use rand::RngCore as _;
use sha2::{Digest as _, Sha256};
use tokio::sync::{Mutex, oneshot};
use warp::Filter as _;

use super::kick_api::{AUTHORIZE_URL, Credentials, exchange_code, fetch_user};
use crate::error;

/// Has to match the redirect URI registered in the Kick application.
pub const DEFAULT_PORT: u16 = 8888;

const CALLBACK_PATH: &str = "kick/callback";
const SCOPES: &str = "user:read chat:write";

const PAGE_OK: &str = "<html><body style='font-family:sans-serif;padding:3rem'>\
     <h1>NOALBS is authorized</h1><p>You can close this tab and go back to the terminal.</p>\
     </body></html>";
const PAGE_BAD: &str = "<html><body style='font-family:sans-serif;padding:3rem'>\
     <h1>Something went wrong</h1><p>Check the terminal.</p></body></html>";

/// Must be `localhost`: Kick rewrites a `127.0.0.1` host on its way through and
/// the exchange then fails. See KickEngineering/KickDevDocs#211
fn redirect_uri(port: u16) -> String {
    format!("http://localhost:{port}/{CALLBACK_PATH}")
}

fn base64_url(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn random_base64(len: usize) -> String {
    let mut bytes = vec![0u8; len];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64_url(&bytes)
}

pub async fn run(port: u16) -> Result<(), error::Error> {
    let credentials = Credentials::from_env().ok_or_else(|| {
        error::Error::KickApi(
            "KICK_CLIENT_ID and KICK_CLIENT_SECRET are not set. Create an application at \
             kick.com (Settings -> Developer) and put them in your .env"
                .into(),
        )
    })?;

    let verifier = random_base64(64);
    let challenge = base64_url(&Sha256::digest(verifier.as_bytes()));
    let state = random_base64(16);
    let redirect = redirect_uri(port);

    let url = reqwest::Url::parse_with_params(
        AUTHORIZE_URL,
        &[
            ("client_id", credentials.client_id.as_str()),
            ("response_type", "code"),
            ("redirect_uri", redirect.as_str()),
            ("scope", SCOPES),
            ("state", state.as_str()),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
        ],
    )
    .map_err(|e| error::Error::KickApi(format!("could not build the authorization URL: {e}")))?;

    println!();
    println!("Register {redirect} as the redirect URI of your Kick application,");
    println!("then open this and approve:");
    println!();
    println!("  {url}");
    println!();
    println!("Waiting for Kick to call back on port {port}...");

    let code = wait_for_callback(port, state).await?;

    let tokens = exchange_code(&credentials, &code, &verifier, &redirect).await?;

    // Kick drops scopes the application lacks without saying so. An empty
    // scope means it told us nothing, not that it granted nothing.
    if !tokens.scope.is_empty() && !tokens.scope.split_whitespace().any(|s| s == "chat:write") {
        return Err(error::Error::KickApi(format!(
            "Kick granted only \"{}\". Your application does not have chat:write \
             enabled, so it was dropped silently. Enable it on the application \
             and run this again",
            tokens.scope
        )));
    }

    let user = fetch_user(&credentials, &tokens.access_token).await?;

    println!();
    println!("Authorized as {} (user_id {}).", user.name, user.user_id);
    println!();
    println!("Put this in the chat section of your config:");
    println!();
    println!("  \"platform\": {{");
    println!("    \"Kick\": {{");
    println!("      \"refreshToken\": \"{}\",", tokens.refresh_token);
    println!("      \"sendAs\": \"bot\"");
    println!("    }}");
    println!("  }},");
    println!("  \"username\": \"{}\",", user.name.to_lowercase());
    println!();
    println!("With sendAs \"bot\" the message carries the bot badge of your");
    println!("application, which needs its bot account created (Settings ->");
    println!("Developer -> your application). Use \"user\" to post as yourself.");
    println!();

    Ok(())
}

/// Serves the redirect URI until Kick calls it with the state we sent.
async fn wait_for_callback(port: u16, state: String) -> Result<String, error::Error> {
    let (tx, rx) = oneshot::channel::<String>();
    let tx = Arc::new(Mutex::new(Some(tx)));

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let shutdown_tx = Arc::new(Mutex::new(Some(shutdown_tx)));

    let route = warp::get()
        .and(warp::path!("kick" / "callback"))
        .and(warp::query::<HashMap<String, String>>())
        .then(move |params: HashMap<String, String>| {
            let tx = tx.clone();
            let shutdown_tx = shutdown_tx.clone();
            let expected = state.clone();

            async move {
                let code = params.get("code").cloned();
                let returned_state = params.get("state").cloned();

                // Anything without our state is not the browser we sent out:
                // ignore it and keep waiting, or any page the user has open
                // could end the flow with an <img> tag.
                let ours = code.zip(returned_state).filter(|(_, s)| *s == expected);

                let Some((code, _)) = ours else {
                    if let Some(error) = params.get("error") {
                        println!("Kick answered with an error: {error}");
                    }
                    return warp::reply::html(PAGE_BAD);
                };

                if let Some(tx) = tx.lock().await.take() {
                    let _ = tx.send(code);
                }

                // Let the response go out before the server stops.
                if let Some(shutdown) = shutdown_tx.lock().await.take() {
                    tokio::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                        let _ = shutdown.send(());
                    });
                }

                warp::reply::html(PAGE_OK)
            }
        });

    // Loopback only: this is a one-shot listener for a credential.
    let (_, server) = warp::serve(route)
        .try_bind_with_graceful_shutdown(([127, 0, 0, 1], port), async move {
            let _ = shutdown_rx.await;
        })
        .map_err(|e| {
            error::Error::KickApi(format!(
                "could not listen on port {port}: {e}. Use `noalbs kick-auth <port>` \
                 to pick another one"
            ))
        })?;

    server.await;

    rx.await
        .map_err(|_| error::Error::KickApi("the browser never came back with a code".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_redirect_uses_localhost_not_the_loopback_address() {
        // Kick rewrites 127.0.0.1 to localhost and the exchange then fails.
        let uri = redirect_uri(1234);

        assert_eq!(uri, "http://localhost:1234/kick/callback");
        assert!(!uri.contains("127.0.0.1"));
    }

    #[test]
    fn the_challenge_is_the_sha256_of_the_verifier() {
        // Vector from RFC 7636, appendix B.
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";

        assert_eq!(
            base64_url(&Sha256::digest(verifier.as_bytes())),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn verifiers_are_not_reused() {
        assert_ne!(random_base64(64), random_base64(64));
    }
}
