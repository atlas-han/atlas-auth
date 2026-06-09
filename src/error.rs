use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("validation_failed")]
    Validation,
    #[error("invalid_credentials")]
    InvalidCredentials,
    #[error("conflict")]
    Conflict,
    #[error("unauthorized")]
    Unauthorized,
    #[error("account_locked")]
    AccountLocked,
    #[error("internal_error")]
    Internal(#[from] anyhow::Error),
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: &'static str,
    pub message: &'static str,
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::Validation => StatusCode::BAD_REQUEST,
            AppError::InvalidCredentials => StatusCode::UNAUTHORIZED,
            AppError::Conflict => StatusCode::CONFLICT,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::AccountLocked => StatusCode::TOO_MANY_REQUESTS,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let (error, message) = match self {
            AppError::Validation => ("validation_failed", "Request validation failed"),
            AppError::InvalidCredentials => ("invalid_credentials", "Invalid email or password"),
            AppError::Conflict => ("conflict", "Resource already exists"),
            AppError::Unauthorized => ("unauthorized", "Authentication is required"),
            AppError::AccountLocked => ("account_locked", "Account is temporarily locked"),
            AppError::Internal(_) => ("internal_error", "Internal server error"),
        };

        HttpResponse::build(self.status_code()).json(ErrorResponse { error, message })
    }
}

impl From<sqlx::Error> for AppError {
    fn from(value: sqlx::Error) -> Self {
        if let sqlx::Error::Database(db_error) = &value {
            if db_error.is_unique_violation() {
                return AppError::Conflict;
            }
        }
        AppError::Internal(anyhow::Error::new(value))
    }
}
