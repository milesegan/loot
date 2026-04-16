use lofty::error::LoftyError;

/// Errors shared across metadata and file operations in the CLI.
#[derive(Debug)]
pub enum AppError {
    /// A filesystem operation failed.
    IoError,
    /// A path could not be derived or converted as expected.
    PathError,
    /// Reading tag metadata failed.
    ReadTagError,
    /// Writing tag metadata failed.
    WriteTagError,
}

/// Convenient result alias for crate-local operations.
pub type Result<T> = std::result::Result<T, AppError>;

impl From<std::num::ParseIntError> for AppError {
    fn from(_err: std::num::ParseIntError) -> AppError {
        AppError::ReadTagError
    }
}

impl From<std::io::Error> for AppError {
    fn from(_err: std::io::Error) -> AppError {
        AppError::IoError
    }
}

impl From<LoftyError> for AppError {
    fn from(_err: LoftyError) -> AppError {
        AppError::ReadTagError
    }
}

impl From<std::path::StripPrefixError> for AppError {
    fn from(_err: std::path::StripPrefixError) -> AppError {
        AppError::PathError
    }
}
