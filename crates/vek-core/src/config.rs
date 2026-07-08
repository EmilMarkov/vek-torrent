//! Конфигурация приложения: чтение/запись JSON в каталоге приложения.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Корневая конфигурация приложения.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub rutracker: RutrackerConfig,
    pub engine: EngineConfig,
    pub api: ApiConfig,
    pub downloads: DownloadsConfig,
}

/// Настройки доступа к rutracker.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RutrackerConfig {
    pub username: String,
    /// Пароль хранится локально; наружу (API/логи) не отдаётся.
    pub password: String,
    /// Базовый адрес зеркала.
    pub mirror: String,
    /// Прокси (`socks5://…`, `http://…`) либо пусто — напрямую.
    pub proxy: String,
}

impl Default for RutrackerConfig {
    fn default() -> Self {
        Self {
            username: String::new(),
            password: String::new(),
            mirror: rutracker::DEFAULT_MIRRORS[0].to_owned(),
            proxy: String::new(),
        }
    }
}

/// Настройки встроенного торрент-движка (librqbit).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EngineConfig {
    /// Порт для входящих соединений; 0 — выбрать автоматически.
    pub listen_port: u16,
    /// Запускать движок при старте приложения.
    pub autostart: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            listen_port: 0,
            autostart: true,
        }
    }
}

/// Настройки внешнего REST API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    /// Bearer-токен; пусто — сгенерировать при первом запуске.
    pub token: String,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "127.0.0.1".to_owned(),
            port: 8737,
            token: String::new(),
        }
    }
}

/// Параметры загрузок по умолчанию.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DownloadsConfig {
    /// Каталог сохранения по умолчанию; пусто — настройка qBittorrent.
    pub default_save_path: String,
    /// Добавлять новые торренты на паузе.
    pub add_stopped: bool,
}

impl AppConfig {
    /// Загружает конфигурацию из файла; при отсутствии — значения по умолчанию.
    pub fn load(path: &Path) -> Result<Self> {
        match fs::read_to_string(path) {
            Ok(text) => {
                let config = serde_json::from_str(&text)
                    .map_err(|e| Error::Config(format!("не удалось разобрать конфиг: {e}")))?;
                Ok(config)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(Error::Io(e)),
        }
    }

    /// Атомарно сохраняет конфигурацию (через временный файл + rename).
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, json)?;
        fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Проверяет корректность значений.
    pub fn validate(&self) -> Result<()> {
        if self.rutracker.mirror.trim().is_empty() {
            return Err(Error::Config("не указано зеркало rutracker".into()));
        }
        if self.api.enabled && self.api.port == 0 {
            return Err(Error::Config("некорректный порт API".into()));
        }
        Ok(())
    }

    /// Заданы ли учётные данные rutracker.
    pub fn has_credentials(&self) -> bool {
        !self.rutracker.username.trim().is_empty() && !self.rutracker.password.is_empty()
    }
}

/// Путь к файлу конфигурации внутри каталога приложения.
pub fn config_file(app_dir: &Path) -> PathBuf {
    app_dir.join("config.json")
}

/// Путь к файлу куков rutracker внутри каталога приложения.
pub fn cookies_file(app_dir: &Path) -> PathBuf {
    app_dir.join("rutracker-cookies.json")
}

/// Каталог состояния встроенного движка (персистентность торрентов).
pub fn engine_state_dir(app_dir: &Path) -> PathBuf {
    app_dir.join("engine-state")
}

/// Эффективный каталог загрузок: заданный пользователем либо `<app>/downloads`.
pub fn downloads_dir(app_dir: &Path, config: &AppConfig) -> PathBuf {
    let configured = config.downloads.default_save_path.trim();
    if configured.is_empty() {
        app_dir.join("downloads")
    } else {
        PathBuf::from(configured)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid() {
        let config = AppConfig::default();
        assert!(config.validate().is_ok());
        assert!(!config.has_credentials());
        assert_eq!(config.engine.listen_port, 0);
        assert!(config.rutracker.mirror.starts_with("https://"));
    }

    #[test]
    fn roundtrip_through_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = config_file(dir.path());

        let mut config = AppConfig::default();
        config.rutracker.username = "user".into();
        config.rutracker.password = "secret".into();
        config.api.enabled = true;
        config.save(&path).unwrap();

        let loaded = AppConfig::load(&path).unwrap();
        assert_eq!(loaded.rutracker.username, "user");
        assert!(loaded.has_credentials());
        assert!(loaded.api.enabled);
    }

    #[test]
    fn missing_file_yields_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let loaded = AppConfig::load(&config_file(dir.path())).unwrap();
        assert!(!loaded.has_credentials());
    }

    #[test]
    fn unknown_fields_do_not_break_load() {
        // Совместимость вперёд: лишние ключи игнорируются, отсутствующие — по умолчанию.
        let dir = tempfile::tempdir().unwrap();
        let path = config_file(dir.path());
        fs::write(&path, r#"{"rutracker":{"username":"u"},"unknown_key":42}"#).unwrap();
        let loaded = AppConfig::load(&path).unwrap();
        assert_eq!(loaded.rutracker.username, "u");
        assert!(loaded.rutracker.password.is_empty());
    }
}
