//! Доменный слой VEK Torrent: конфигурация, сервисы поиска/раздач/загрузок,
//! sidecar-менеджер qBittorrent.
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
