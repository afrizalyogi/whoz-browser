use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub scopes: Vec<String>,
}

impl Default for GoogleAuthConfig {
    fn default() -> Self {
        Self {
            client_id: "777569123456-dummy.apps.googleusercontent.com".to_string(),
            client_secret: "".to_string(),
            scopes: vec![
                "https://www.googleapis.com/auth/chromesync".to_string(),
                "email".to_string(),
                "profile".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub struct GoogleToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Instant,
    pub email: Option<String>,
}

impl GoogleToken {
    pub fn is_valid(&self) -> bool {
        Instant::now() < self.expires_at - Duration::from_secs(60)
    }
}
