use gloo::utils::window;
use serde::{Deserialize, Serialize};
use web_sys::UrlSearchParams;
use uuid::Uuid;
use wasm_bindgen::JsValue;

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const CLIENT_ID: &str = "126932716262-m3jg96nhn9efg7mkee5k9d9aqnu0282l.apps.googleusercontent.com";

#[cfg(debug_assertions)]
const REDIRECT_URI: &str = "http://localhost:8080/";
#[cfg(not(debug_assertions))]
const REDIRECT_URI: &str = "https://geeom.github.io/personal_assistant/";

#[cfg(debug_assertions)]
pub const BACKEND_URL: &str = "http://localhost:3000";
#[cfg(not(debug_assertions))]
pub const BACKEND_URL: &str = "https://personal-assistant-backend.fly.dev";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i64,
    pub google_id: String,
    pub email: String,
    pub name: String,
}

#[derive(Debug, Clone, Default)]
pub struct AuthState {
    pub token: Option<String>,
    pub user: Option<UserInfo>,
}

impl AuthState {
    pub fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }
}

pub fn generate_state() -> String {
    Uuid::new_v4().to_string()
}

pub fn save_state(state: &str) {
    if let Ok(storage) = window().local_storage() {
        if let Some(storage) = storage {
            let _ = storage.set_item("oauth_state", state);
        }
    }
}

pub fn get_saved_state() -> Option<String> {
    window()
        .local_storage()
        .ok()
        .flatten()
        .and_then(|storage| storage.get_item("oauth_state").ok())
        .flatten()
}

pub fn clear_saved_state() {
    if let Ok(storage) = window().local_storage() {
        if let Some(storage) = storage {
            let _ = storage.remove_item("oauth_state");
        }
    }
}

pub fn initiate_oauth_flow() {
    let state = generate_state();
    save_state(&state);
    
    let params = [
        ("client_id", CLIENT_ID),
        ("redirect_uri", REDIRECT_URI),
        ("response_type", "code"),
        ("scope", "openid email profile"),
        ("state", &state),
        ("access_type", "online"),
    ];
    
    let query_string = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");
    
    let auth_url = format!("{}?{}", GOOGLE_AUTH_URL, query_string);
    
    window().location().set_href(&auth_url).unwrap();
}

pub fn parse_oauth_callback() -> Option<(String, String)> {
    let location = window().location();
    let search = location.search().ok()?;
    
    if search.is_empty() {
        return None;
    }
    
    let params = UrlSearchParams::new_with_str(&search).ok()?;
    
    let code = params.get("code")?;
    let state = params.get("state")?;
    
    Some((code, state))
}

pub fn clear_url_params() {
    let location = window().location();
    if let Ok(path) = location.pathname() {
        let _ = window().history().unwrap()
            .replace_state_with_url(&JsValue::NULL, "", Some(&path));
    }
}

pub async fn exchange_code_for_token(code: String) -> Result<AuthResponse, String> {
    let request_body = AuthRequest { code };
    
    let response = gloo_net::http::Request::post(&format!("{}/auth/google", BACKEND_URL))
        .json(&request_body)
        .map_err(|e| format!("Failed to create request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;
    
    if !response.ok() {
        return Err(format!("Authentication failed: {}", response.status()));
    }
    
    response
        .json::<AuthResponse>()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

mod urlencoding {
    pub fn encode(s: &str) -> String {
        s.chars()
            .map(|c| {
                if c.is_alphanumeric() || "-_.~".contains(c) {
                    c.to_string()
                } else {
                    format!("%{:02X}", c as u8)
                }
            })
            .collect()
    }
}