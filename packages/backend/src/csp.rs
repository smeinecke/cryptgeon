use axum::{
    http::{HeaderValue, StatusCode, header},
    response::{Html, IntoResponse, Response},
};
use ring::rand::SecureRandom;
use std::sync::OnceLock;

// 'self' covers the same-origin module chunks; the nonce unlocks the inline
// bootstrap script emitted by SvelteKit. 'unsafe-inline' is kept only for
// styles because Svelte emits style attributes.
const CSP_POLICY: &str = "default-src 'self'; script-src 'self' 'nonce-{nonce}'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob: https:; font-src 'self'; connect-src 'self'; worker-src 'self' blob:; object-src 'none'; base-uri 'self'; frame-ancestors 'none'; form-action 'self'; manifest-src 'self'";

// index.html is baked into the deployment image, so caching it is safe.
fn index_html() -> Option<&'static str> {
    static HTML: OnceLock<Option<String>> = OnceLock::new();
    HTML.get_or_init(|| {
        let path = format!("{}{}", *crate::config::FRONTEND_PATH, "/index.html");
        std::fs::read_to_string(path).ok()
    })
    .as_deref()
}

fn generate_nonce() -> String {
    let rng = ring::rand::SystemRandom::new();
    let mut bytes = [0u8; 32];
    rng.fill(&mut bytes).expect("system RNG failed");
    bs62::encode_data(&bytes)
}

// Serves index.html with a per-request CSP nonce injected into every
// <script> tag and the matching Content-Security-Policy header.
pub async fn spa_index() -> Response {
    let Some(html) = index_html() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let nonce = generate_nonce();
    let html = html.replace("<script", &format!("<script nonce=\"{nonce}\""));
    let mut response = Html(html).into_response();
    let csp = CSP_POLICY.replace("{nonce}", &nonce);
    if let Ok(csp) = HeaderValue::from_str(&csp) {
        response
            .headers_mut()
            .insert(header::CONTENT_SECURITY_POLICY, csp);
    }
    response
}
