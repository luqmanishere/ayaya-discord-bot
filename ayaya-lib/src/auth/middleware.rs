use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

use crate::AxumState;

/// Authenticated user information
#[derive(Clone, Debug)]
pub struct AuthUser {
    pub user_id: i64,
}

/// Auth middleware that validates Bearer tokens
pub struct AuthMiddleware;

impl AuthMiddleware {
    pub async fn require_auth(
        State(state): State<Arc<TokioMutex<AxumState>>>,
        mut request: Request,
        next: Next,
    ) -> Result<Response, AuthError> {
        // Extract Authorization header
        let auth_header = request
            .headers()
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or(AuthError::MissingToken)?;

        // Check Bearer scheme
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AuthError::InvalidToken)?;

        // Validate token against database
        let state = state.lock().await;
        let user_id = state
            .data_manager
            .validate_dashboard_token(token)
            .await
            .map_err(|_| AuthError::InvalidToken)?
            .ok_or(AuthError::InvalidToken)?;

        // Update last used timestamp
        let _ = state.data_manager.update_token_last_used(token).await;

        // Add user to request extensions
        request.extensions_mut().insert(AuthUser { user_id });

        Ok(next.run(request).await)
    }
}

#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidToken,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::MissingToken => (StatusCode::UNAUTHORIZED, "Missing authorization token"),
            AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid or expired token"),
        };

        (status, message).into_response()
    }
}
