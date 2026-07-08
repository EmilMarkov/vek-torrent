//! Типизированный клиент qBittorrent Web API v2 (qBittorrent 5.x, fallback 4.x).
//!
//! Наполняется на этапе 3.

/// Версия крейта (проверка сборки скелета).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_set() {
        assert!(!super::VERSION.is_empty());
    }
}
