use anyhow::Error;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    InvalidInput(Error),
    #[error(transparent)]
    UnreadablePath(Error),
    #[error(transparent)]
    UnsupportedExplainTarget(Error),
    #[error(transparent)]
    Runtime(Error),
}

impl AppError {
    pub fn invalid_input(error: impl Into<Error>) -> Self {
        Self::InvalidInput(error.into())
    }

    pub fn unreadable_path(error: impl Into<Error>) -> Self {
        Self::UnreadablePath(error.into())
    }

    pub fn unsupported_explain_target(error: impl Into<Error>) -> Self {
        Self::UnsupportedExplainTarget(error.into())
    }

    pub fn runtime(error: impl Into<Error>) -> Self {
        Self::Runtime(error.into())
    }

    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Runtime(_) => 1,
            Self::InvalidInput(_) => 2,
            Self::UnreadablePath(_) => 3,
            Self::UnsupportedExplainTarget(_) => 4,
        }
    }
}
