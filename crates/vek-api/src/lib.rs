//! Внешний REST API VEK Torrent: axum-роутер, Bearer-авторизация, OpenAPI.
//!
//! Наполняется на этапе 4.

/// Версия крейта (проверка сборки скелета).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_set() {
        assert!(!super::VERSION.is_empty());
    }
}
