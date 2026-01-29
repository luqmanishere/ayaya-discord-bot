use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

use ayaya_core::auth::token::verify_token;

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
        let tokens = state
            .data_manager
            .list_active_tokens()
            .await
            .map_err(|_| AuthError::InvalidToken)?;

        let mut authed_user = None;
        let mut authed_token_id = None;
        for token_model in tokens {
            if verify_token(token, &token_model.token_hash) {
                if state
                    .data_manager
                    .is_allowlisted(token_model.user_id)
                    .await
                    .map_err(|_| AuthError::InvalidToken)?
                {
                    authed_user = Some(token_model.user_id);
                    authed_token_id = Some(token_model.token_id);
                    break;
                }
            }
        }

        let user_id = authed_user.ok_or(AuthError::InvalidToken)?;

        // Update last used timestamp
        if let Some(token_id) = authed_token_id {
            let _ = state.data_manager.update_token_last_used_by_id(token_id).await;
        }

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
