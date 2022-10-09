pub enum AppError {
    IoError,
    PathError,
    ReadTagError,
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

impl From<lofty::LoftyError> for AppError {
    fn from(_err: lofty::LoftyError) -> AppError {
        AppError::ReadTagError
    }
}

impl From<opus_headers::ParseError> for AppError {
    fn from(_err: opus_headers::ParseError) -> AppError {
        AppError::ReadTagError
    }
}

impl From<metaflac::Error> for AppError {
    fn from(_err: metaflac::Error) -> AppError {
        AppError::ReadTagError
    }
}

impl From<std::path::StripPrefixError> for AppError {
    fn from(_err: std::path::StripPrefixError) -> AppError {
        AppError::PathError
    }
}
