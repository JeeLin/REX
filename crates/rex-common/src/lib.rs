//! REX 共享通用库：错误类型、版本、配置基础。
//!
//! 2.0 重设计：从 0 重建，按新架构组织。

pub const APP_NAME: &str = "REX Hub";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 统一错误类型。后续按模块扩展。
#[derive(Debug)]
pub enum RExError {
    Io(std::io::Error),
    Message(String),
}

impl std::fmt::Display for RExError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RExError::Io(e) => write!(f, "io error: {e}"),
            RExError::Message(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for RExError {}

impl From<std::io::Error> for RExError {
    fn from(e: std::io::Error) -> Self {
        RExError::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, RExError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rex_error_display() {
        let err = RExError::Message("test error".into());
        assert_eq!(err.to_string(), "test error");
    }

    #[test]
    fn test_rex_error_io_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let rex_err: RExError = io_err.into();
        assert!(rex_err.to_string().contains("not found"));
    }

    #[test]
    fn test_rex_error_is_error() {
        let err = RExError::Message("test".into());
        let _: &dyn std::error::Error = &err;
    }
}

pub mod file_transfer;
pub mod redis;
pub mod sql;
pub mod supervisor;
pub mod update;
