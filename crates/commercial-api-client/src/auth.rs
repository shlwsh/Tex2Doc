//! Authentication API methods.

use crate::client::ApiClient;
use crate::models::{AuthResponse, LoginRequest, RefreshRequest, RegisterRequest, UserProfile};
use crate::ApiError;

impl ApiClient {
    pub async fn register(&self, request: &RegisterRequest) -> Result<AuthResponse, ApiError> {
        self.post("auth/register", request).await
    }

    pub async fn login(&self, request: &LoginRequest) -> Result<AuthResponse, ApiError> {
        self.post("auth/login", request).await
    }

    pub async fn refresh(&self, request: &RefreshRequest) -> Result<AuthResponse, ApiError> {
        self.post("auth/refresh", request).await
    }

    pub async fn me(&self) -> Result<UserProfile, ApiError> {
        self.get("me").await
    }
}
