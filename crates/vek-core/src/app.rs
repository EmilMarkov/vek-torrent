//! [`AppCore`] — центральный оркестратор: конфигурация, клиент rutracker,
//! sidecar qBittorrent и его клиент, доменные операции для UI и внешнего API.

use std::{
    path::PathBuf,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use rutracker::models::{
    CaptchaAnswer, ForumGroup, SearchPage, SearchRequest, SessionInfo, TopicPage,
};

use crate::{
    config::{self, AppConfig},
    error::{Error, Result},
    models::{AddOptions, AppStatus, DownloadItem, TransferSummary},
    sidecar::{self, SidecarOptions, SidecarProcess},
};

/// Общий доступ к ядру приложения.
pub type SharedCore = Arc<AppCore>;

/// Ядро приложения. Потокобезопасно; клонируется через [`Arc`].
pub struct AppCore {
    app_dir: PathBuf,
    config: RwLock<AppConfig>,
    rutracker: RwLock<rutracker::Client>,
    qbit: RwLock<Option<qbit::Client>>,
    sidecar: tokio::sync::Mutex<Option<SidecarProcess>>,
    api_running: AtomicBool,
}

impl AppCore {
    /// Инициализирует ядро: загружает конфиг, создаёт клиент rutracker,
    /// при необходимости генерирует токен API.
    pub fn new(app_dir: impl Into<PathBuf>) -> Result<SharedCore> {
        let app_dir = app_dir.into();
        std::fs::create_dir_all(&app_dir)?;

        let mut config = AppConfig::load(&config::config_file(&app_dir))?;
        if config.api.token.trim().is_empty() {
            config.api.token = generate_token();
            let _ = config.save(&config::config_file(&app_dir));
        }

        let rutracker = build_rutracker_client(&config, &app_dir)?;

        Ok(Arc::new(Self {
            app_dir,
            config: RwLock::new(config),
            rutracker: RwLock::new(rutracker),
            qbit: RwLock::new(None),
            sidecar: tokio::sync::Mutex::new(None),
            api_running: AtomicBool::new(false),
        }))
    }

    /// Каталог данных приложения.
    pub fn app_dir(&self) -> &PathBuf {
        &self.app_dir
    }

    // ── Конфигурация ────────────────────────────────────────────────────

    /// Текущая конфигурация (копия).
    pub fn config(&self) -> AppConfig {
        self.config.read().expect("config lock").clone()
    }

    /// Конфигурация без секретов (для отдачи во внешний мир).
    pub fn config_redacted(&self) -> AppConfig {
        let mut config = self.config();
        config.rutracker.password = String::new();
        config.api.token = String::new();
        config
    }

    /// Конфигурация для локального UI: пароль скрыт (не покидает ядро),
    /// токен API оставлен — пользователь должен видеть его для интеграций.
    pub fn config_for_ui(&self) -> AppConfig {
        let mut config = self.config();
        config.rutracker.password = String::new();
        config
    }

    /// Обновляет конфигурацию: валидирует, сохраняет, пересоздаёт клиент
    /// rutracker при изменении зеркала или прокси.
    ///
    /// Пустые секреты трактуются как «не менять»: это позволяет UI не хранить
    /// и не пересылать пароль, отправляя пустое поле, когда его не трогали.
    pub fn update_config(&self, mut new_config: AppConfig) -> Result<()> {
        let old = self.config();

        if new_config.rutracker.password.is_empty() {
            new_config.rutracker.password = old.rutracker.password.clone();
        }
        if new_config.api.token.trim().is_empty() {
            new_config.api.token = old.api.token.clone();
        }

        new_config.validate()?;

        let rebuild_rutracker = old.rutracker.mirror != new_config.rutracker.mirror
            || old.rutracker.proxy != new_config.rutracker.proxy;

        new_config.save(&config::config_file(&self.app_dir))?;

        if rebuild_rutracker {
            let client = build_rutracker_client(&new_config, &self.app_dir)?;
            *self.rutracker.write().expect("rutracker lock") = client;
        }
        *self.config.write().expect("config lock") = new_config;
        Ok(())
    }

    fn rutracker(&self) -> rutracker::Client {
        self.rutracker.read().expect("rutracker lock").clone()
    }

    // ── Сессия rutracker ────────────────────────────────────────────────

    /// Проверяет состояние сессии на трекере.
    pub async fn session_status(&self) -> Result<SessionInfo> {
        self.rutracker()
            .session_info()
            .await
            .map_err(Error::from_rutracker)
    }

    /// Выполняет вход, используя учётные данные из конфигурации.
    ///
    /// При требовании капчи возвращает [`Error::CaptchaRequired`].
    pub async fn login(&self, captcha: Option<CaptchaAnswer>) -> Result<()> {
        let config = self.config();
        if !config.has_credentials() {
            return Err(Error::NoCredentials);
        }
        self.rutracker()
            .login(
                &config.rutracker.username,
                &config.rutracker.password,
                captcha.as_ref(),
            )
            .await
            .map_err(Error::from_rutracker)
    }

    /// Локальный выход (очистка куков).
    pub fn logout(&self) -> Result<()> {
        self.rutracker().logout().map_err(Error::from_rutracker)
    }

    /// Загружает изображение (капчу) через сессию и прокси трекера.
    pub async fn fetch_image(&self, url: &str) -> Result<(Vec<u8>, Option<String>)> {
        self.rutracker()
            .fetch_image(url)
            .await
            .map_err(Error::from_rutracker)
    }

    // ── Поиск и раздачи ─────────────────────────────────────────────────

    /// Поиск по трекеру с авто-логином при протухшей сессии.
    pub async fn search(&self, request: SearchRequest) -> Result<SearchPage> {
        self.with_auth_retry(|client| {
            let request = request.clone();
            async move { client.search(&request).await }
        })
        .await
    }

    /// Страница раздачи.
    pub async fn topic(&self, id: u64) -> Result<TopicPage> {
        self.with_auth_retry(|client| async move { client.topic(id).await })
            .await
    }

    /// Дерево категорий (форумов).
    pub async fn categories(&self) -> Result<Vec<ForumGroup>> {
        self.with_auth_retry(|client| async move { client.categories().await })
            .await
    }

    /// Выполняет операцию, при `NotAuthenticated` пытается войти и повторить.
    async fn with_auth_retry<F, Fut, T>(&self, make: F) -> Result<T>
    where
        F: Fn(rutracker::Client) -> Fut,
        Fut: std::future::Future<Output = std::result::Result<T, rutracker::Error>>,
    {
        let client = self.rutracker();
        match make(client.clone()).await {
            Err(rutracker::Error::NotAuthenticated) => {
                self.login(None).await?;
                make(client).await.map_err(Error::from_rutracker)
            }
            other => other.map_err(Error::from_rutracker),
        }
    }

    // ── qBittorrent sidecar ─────────────────────────────────────────────

    /// Гарантирует, что sidecar запущен, и возвращает подключённый клиент.
    pub async fn ensure_qbit(&self) -> Result<qbit::Client> {
        if let Some(client) = self.qbit_client_if_alive().await {
            return Ok(client);
        }

        let mut guard = self.sidecar.lock().await;
        // Повторная проверка под блокировкой (мог подняться параллельно).
        if let Some(process) = guard.as_mut()
            && process.is_running()
        {
            let client = process.client()?;
            if client.is_alive().await {
                *self.qbit.write().expect("qbit lock") = Some(client.clone());
                return Ok(client);
            }
        }

        let config = self.config();
        let options = SidecarOptions {
            binary_path: config.qbittorrent.binary_path.clone(),
            profile_dir: config::qbit_profile_dir(&self.app_dir),
            preferred_port: config.qbittorrent.port,
            default_save_path: config.downloads.default_save_path.clone(),
        };

        let process = sidecar::spawn(&options).await?;
        let client = process.client()?;
        *guard = Some(process);
        *self.qbit.write().expect("qbit lock") = Some(client.clone());
        Ok(client)
    }

    async fn qbit_client_if_alive(&self) -> Option<qbit::Client> {
        let client = self.qbit.read().expect("qbit lock").clone()?;
        if client.is_alive().await {
            Some(client)
        } else {
            None
        }
    }

    /// Останавливает sidecar (если запущен).
    pub async fn stop_qbit(&self) {
        *self.qbit.write().expect("qbit lock") = None;
        let process = self.sidecar.lock().await.take();
        if let Some(process) = process {
            process.shutdown().await;
        }
    }

    // ── Загрузки ────────────────────────────────────────────────────────

    /// Список загрузок.
    pub async fn downloads(&self) -> Result<Vec<DownloadItem>> {
        let client = self.ensure_qbit().await?;
        let torrents = client.torrents(&Default::default()).await?;
        Ok(torrents.into_iter().map(DownloadItem::from).collect())
    }

    /// Глобальная статистика передачи.
    pub async fn transfer(&self) -> Result<TransferSummary> {
        let client = self.ensure_qbit().await?;
        Ok(client.transfer_info().await?.into())
    }

    /// Снимок загрузок и статистики БЕЗ запуска sidecar (для фонового опроса).
    ///
    /// Возвращает `None`, если qBittorrent сейчас не запущен.
    pub async fn snapshot(&self) -> Option<(Vec<DownloadItem>, TransferSummary)> {
        let client = self.qbit_client_if_alive().await?;
        let torrents = client.torrents(&Default::default()).await.ok()?;
        let transfer = client.transfer_info().await.ok()?;
        let items = torrents.into_iter().map(DownloadItem::from).collect();
        Some((items, transfer.into()))
    }

    /// Запущен ли sidecar qBittorrent прямо сейчас.
    pub async fn qbit_running(&self) -> bool {
        self.qbit_client_if_alive().await.is_some()
    }

    /// Категории qBittorrent.
    pub async fn qbit_categories(&self) -> Result<Vec<qbit::models::Category>> {
        let client = self.ensure_qbit().await?;
        client.categories().await.map_err(Error::from)
    }

    /// Добавляет раздачу в загрузки, скачивая `.torrent` или используя magnet.
    pub async fn add_from_topic(&self, topic_id: u64, options: AddOptions) -> Result<()> {
        let config = self.config();
        let qbit_client = self.ensure_qbit().await?;

        let topic = self
            .with_auth_retry(|client| async move { client.topic(topic_id).await })
            .await?;

        let source = self.pick_source(topic_id, &topic, options.prefer_magnet).await?;

        let stopped = options.stopped.unwrap_or(config.downloads.add_stopped);
        let save_path = options
            .save_path
            .filter(|p| !p.trim().is_empty())
            .or_else(|| {
                Some(config.downloads.default_save_path.clone())
                    .filter(|p| !p.trim().is_empty())
            });

        let add = qbit::models::AddTorrent {
            sources: vec![source],
            save_path,
            category: options.category.filter(|c| !c.trim().is_empty()),
            tags: Vec::new(),
            stopped,
            skip_checking: false,
        };
        qbit_client.add_torrent(&add).await.map_err(Error::from)
    }

    /// Выбирает источник: `.torrent`-файл (по умолчанию) или magnet.
    async fn pick_source(
        &self,
        topic_id: u64,
        topic: &TopicPage,
        prefer_magnet: bool,
    ) -> Result<qbit::models::TorrentSource> {
        let use_magnet = prefer_magnet || !topic.has_torrent_file;
        if use_magnet && let Some(magnet) = &topic.magnet {
            return Ok(qbit::models::TorrentSource::Url(magnet.clone()));
        }

        if topic.has_torrent_file {
            let file = self
                .with_auth_retry(|client| async move { client.download_torrent(topic_id).await })
                .await?;
            return Ok(qbit::models::TorrentSource::File {
                filename: file
                    .filename
                    .unwrap_or_else(|| format!("{topic_id}.torrent")),
                bytes: file.bytes,
            });
        }

        // Файла нет — последний шанс — magnet.
        if let Some(magnet) = &topic.magnet {
            return Ok(qbit::models::TorrentSource::Url(magnet.clone()));
        }

        Err(Error::Rutracker(rutracker::Error::AccessDenied(
            "у раздачи нет ни .torrent, ни magnet-ссылки".into(),
        )))
    }

    /// Добавляет торрент по magnet- или http-ссылке напрямую.
    pub async fn add_url(&self, url: String, options: AddOptions) -> Result<()> {
        let config = self.config();
        let client = self.ensure_qbit().await?;
        let add = qbit::models::AddTorrent {
            sources: vec![qbit::models::TorrentSource::Url(url)],
            save_path: options
                .save_path
                .filter(|p| !p.trim().is_empty())
                .or_else(|| {
                    Some(config.downloads.default_save_path.clone())
                        .filter(|p| !p.trim().is_empty())
                }),
            category: options.category.filter(|c| !c.trim().is_empty()),
            tags: Vec::new(),
            stopped: options.stopped.unwrap_or(config.downloads.add_stopped),
            skip_checking: false,
        };
        client.add_torrent(&add).await.map_err(Error::from)
    }

    /// Ставит загрузки на паузу.
    pub async fn pause(&self, hashes: Vec<String>) -> Result<()> {
        self.ensure_qbit()
            .await?
            .stop(&hashes)
            .await
            .map_err(Error::from)
    }

    /// Возобновляет загрузки.
    pub async fn resume(&self, hashes: Vec<String>) -> Result<()> {
        self.ensure_qbit()
            .await?
            .start(&hashes)
            .await
            .map_err(Error::from)
    }

    /// Удаляет загрузки (опционально с файлами).
    pub async fn remove(&self, hashes: Vec<String>, delete_files: bool) -> Result<()> {
        self.ensure_qbit()
            .await?
            .delete(&hashes, delete_files)
            .await
            .map_err(Error::from)
    }

    // ── Статус ──────────────────────────────────────────────────────────

    /// Отмечает состояние внешнего API (устанавливается слоем приложения).
    pub fn set_api_running(&self, running: bool) {
        self.api_running.store(running, Ordering::SeqCst);
    }

    /// Сводный статус подсистем.
    pub async fn status(&self) -> AppStatus {
        let (qbit_running, qbit_version) = match self.qbit_client_if_alive().await {
            Some(client) => (true, client.version().await.ok()),
            None => (false, None),
        };

        let session = self.session_status().await.ok();
        AppStatus {
            qbit_running,
            qbit_version,
            api_running: self.api_running.load(Ordering::SeqCst),
            logged_in: session.as_ref().map(|s| s.logged_in).unwrap_or(false),
            username: session.and_then(|s| s.username),
        }
    }
}

fn build_rutracker_client(
    config: &AppConfig,
    app_dir: &std::path::Path,
) -> Result<rutracker::Client> {
    let proxy = if config.rutracker.proxy.trim().is_empty() {
        None
    } else {
        Some(config.rutracker.proxy.clone())
    };

    rutracker::Client::builder()
        .base_url(config.rutracker.mirror.clone())
        .proxy(proxy)
        .cookie_path(Some(config::cookies_file(app_dir)))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(Error::from_rutracker)
}

/// Генерирует случайный Bearer-токен для внешнего API.
fn generate_token() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn core() -> SharedCore {
        let dir = tempfile::tempdir().unwrap();
        // Каталог живёт до конца процесса теста через leak — достаточно для юнита.
        let path = dir.keep();
        AppCore::new(path).unwrap()
    }

    #[test]
    fn new_generates_api_token() {
        let core = core();
        let config = core.config();
        assert!(!config.api.token.is_empty());
        assert_eq!(config.api.token.len(), 64);
    }

    #[test]
    fn redacted_config_hides_secrets() {
        let core = core();
        let mut config = core.config();
        config.rutracker.username = "user".into();
        config.rutracker.password = "secret".into();
        core.update_config(config).unwrap();

        let redacted = core.config_redacted();
        assert_eq!(redacted.rutracker.username, "user");
        assert!(redacted.rutracker.password.is_empty());
        assert!(redacted.api.token.is_empty());
    }

    #[test]
    fn update_config_persists_and_validates() {
        let core = core();
        let mut config = core.config();
        config.rutracker.mirror = String::new();
        assert!(core.update_config(config).is_err());

        let mut ok = core.config();
        ok.downloads.default_save_path = "/tmp/dl".into();
        core.update_config(ok).unwrap();
        assert_eq!(core.config().downloads.default_save_path, "/tmp/dl");
    }

    #[tokio::test]
    async fn login_without_credentials_fails() {
        let core = core();
        assert!(matches!(core.login(None).await, Err(Error::NoCredentials)));
    }
}
