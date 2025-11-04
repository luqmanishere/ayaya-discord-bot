use axum::{Extension, Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;

use crate::auth::middleware::AuthUser;

#[derive(Serialize)]
pub struct AuthMeResponse {
    pub user_id: String,
    pub is_authenticated: bool,
}

/// GET /api/auth/me - Get current authenticated user info
pub async fn auth_me_handler(Extension(auth_user): Extension<AuthUser>) -> impl IntoResponse {
    let response = AuthMeResponse {
        user_id: auth_user.user_id.to_string(),
        is_authenticated: true,
    };

    (StatusCode::OK, Json(response))
}
