use lofty::error::LoftyError;

#[derive(Debug)]
pub enum AppError {
    IoError,
    PathError,
    ReadTagError,
    WriteTagError,
}

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
