/// Public re-exports for use in integration tests and benchmarks.
pub mod config;
pub mod db;
pub mod errors;
pub mod http_server;
pub mod models;
pub mod utils;

pub use errors::{AppError, Result};
pub use models::{Post, User, UserRole};

/// Package version, sourced from Cargo metadata at compile time.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default page size used across pagination endpoints.
pub const DEFAULT_PAGE_SIZE: u32 = 20;

/// Maximum allowed page size for any paginated endpoint.
pub const MAX_PAGE_SIZE: u32 = 100;

/// Returns `true` when the build was compiled in debug mode.
#[inline]
pub fn is_debug_build() -> bool {
    cfg!(debug_assertions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_set() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn page_size_constants() {
        assert!(DEFAULT_PAGE_SIZE > 0);
        assert!(MAX_PAGE_SIZE >= DEFAULT_PAGE_SIZE);
    }
}
