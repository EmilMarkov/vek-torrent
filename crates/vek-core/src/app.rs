//! [`AppCore`] — центральный оркестратор: конфигурация, клиент rutracker,
//! встроенный торрент-движок, доменные операции для UI и внешнего API.

use std::{
    path::PathBuf,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use engine::{AddParams, Engine, Source};
use rutracker::models::{
    CaptchaAnswer, ForumGroup, SearchPage, SearchRequest, SessionInfo, TopicPage,
};

use crate::{
    config::{self, AppConfig},
    error::{Error, Result},
    library::{self, FavoriteRecord, HistoryRecord, Library, now_unix},
    models::{
        AddOptions, AppStatus, DownloadItem, FavoriteItem, HistoryItem, TorrentFilesPreview,
        TransferSummary,
    },
};

/// Общий доступ к ядру приложения.
pub type SharedCore = Arc<AppCore>;

/// Ядро приложения. Потокобезопасно; клонируется через [`Arc`].
pub struct AppCore {
    app_dir: PathBuf,
    config: RwLock<AppConfig>,
    rutracker: RwLock<rutracker::Client>,
    engine: tokio::sync::Mutex<Option<Engine>>,
    library: RwLock<Library>,
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
        let library = Library::load(&library::library_file(&app_dir));

        Ok(Arc::new(Self {
            app_dir,
            config: RwLock::new(config),
            rutracker: RwLock::new(rutracker),
            engine: tokio::sync::Mutex::new(None),
            library: RwLock::new(library),
            api_running: AtomicBool::new(false),
        }))
    }

    fn save_library(&self) {
        let path = library::library_file(&self.app_dir);
        let snapshot = self.library.read().expect("library lock").clone();
        if let Err(e) = snapshot.save(&path) {
            tracing::warn!("не удалось сохранить библиотеку: {e}");
        }
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

    // ── Торрент-движок ──────────────────────────────────────────────────

    /// Гарантирует, что встроенный движок запущен, и возвращает его.
    pub async fn ensure_engine(&self) -> Result<Engine> {
        let mut guard = self.engine.lock().await;
        if let Some(existing) = guard.as_ref() {
            return Ok(existing.clone());
        }
        let config = self.config();
        let engine_config = engine::EngineConfig {
            download_dir: config::downloads_dir(&self.app_dir, &config),
            state_dir: config::engine_state_dir(&self.app_dir),
            listen_port: (config.engine.listen_port != 0).then_some(config.engine.listen_port),
        };
        let started = Engine::start(engine_config).await?;
        *guard = Some(started.clone());
        Ok(started)
    }

    async fn engine_if_running(&self) -> Option<Engine> {
        self.engine.lock().await.clone()
    }

    /// Останавливает движок (сбрасывает сессию; персистентность сохранена).
    pub async fn stop_engine(&self) {
        *self.engine.lock().await = None;
    }

    // ── Загрузки ────────────────────────────────────────────────────────

    /// Список загрузок.
    pub async fn downloads(&self) -> Result<Vec<DownloadItem>> {
        let engine = self.ensure_engine().await?;
        Ok(engine
            .torrents()
            .into_iter()
            .map(DownloadItem::from)
            .collect())
    }

    /// Глобальная статистика передачи.
    pub async fn transfer(&self) -> Result<TransferSummary> {
        let engine = self.ensure_engine().await?;
        Ok(summarize(&engine.torrents()))
    }

    /// Снимок загрузок и статистики БЕЗ запуска движка (для фонового опроса).
    ///
    /// Возвращает `None`, если движок сейчас не запущен.
    pub async fn snapshot(&self) -> Option<(Vec<DownloadItem>, TransferSummary)> {
        let engine = self.engine_if_running().await?;
        let torrents = engine.torrents();
        let transfer = summarize(&torrents);
        let items = torrents.into_iter().map(DownloadItem::from).collect();
        Some((items, transfer))
    }

    /// Запущен ли движок прямо сейчас.
    pub async fn engine_running(&self) -> bool {
        self.engine_if_running().await.is_some()
    }

    /// Готовит источник для добавления: `.torrent`-файл (по умолчанию) или magnet.
    async fn pick_source(
        &self,
        topic_id: u64,
        topic: &TopicPage,
        prefer_magnet: bool,
    ) -> Result<Source> {
        let use_magnet = prefer_magnet || !topic.has_torrent_file;
        if use_magnet && let Some(magnet) = &topic.magnet {
            return Ok(Source::Url(magnet.clone()));
        }
        if topic.has_torrent_file {
            let file = self
                .with_auth_retry(|client| async move { client.download_torrent(topic_id).await })
                .await?;
            return Ok(Source::TorrentBytes(file.bytes));
        }
        if let Some(magnet) = &topic.magnet {
            return Ok(Source::Url(magnet.clone()));
        }
        Err(Error::Rutracker(rutracker::Error::AccessDenied(
            "у раздачи нет ни .torrent, ни magnet-ссылки".into(),
        )))
    }

    fn add_params(&self, config: &AppConfig, options: &AddOptions) -> AddParams {
        let output_folder = options.save_path.clone().filter(|p| !p.trim().is_empty());
        AddParams {
            output_folder,
            paused: options.stopped.unwrap_or(config.downloads.add_stopped),
            only_files: options.only_files.clone().filter(|f| !f.is_empty()),
        }
    }

    /// Возвращает список файлов раздачи (для выбора перед скачиванием).
    ///
    /// Источники пробуются по очереди: сначала `.torrent`-файл (метаданные уже
    /// внутри — быстро, без сети), затем magnet (метаданные тянутся из роя —
    /// медленно, с таймаутом). Если движок отверг один источник, пробуем
    /// следующий, чтобы не падать на нестандартных раздачах. Наверх уходит
    /// реальная последняя ошибка, а не общее «движок недоступен».
    pub async fn preview_topic(&self, topic_id: u64) -> Result<TorrentFilesPreview> {
        let engine = self.ensure_engine().await?;
        let topic = self
            .with_auth_retry(|client| async move { client.topic(topic_id).await })
            .await?;

        let mut last_err: Option<Error> = None;
        let mut sources: Vec<Source> = Vec::new();

        if topic.has_torrent_file {
            match self
                .with_auth_retry(|client| async move { client.download_torrent(topic_id).await })
                .await
            {
                Ok(file) => sources.push(Source::TorrentBytes(file.bytes)),
                // Не удалось скачать .torrent — попробуем magnet ниже.
                Err(e) => last_err = Some(e),
            }
        }
        if let Some(magnet) = &topic.magnet {
            sources.push(Source::Url(magnet.clone()));
        }

        for source in sources {
            match engine.preview(source).await {
                Ok(preview) => {
                    return Ok(TorrentFilesPreview {
                        hash: preview.hash,
                        name: if preview.name.is_empty() {
                            topic.title.clone()
                        } else {
                            preview.name
                        },
                        total_size: preview.total_size,
                        files: preview.files.into_iter().map(Into::into).collect(),
                    });
                }
                Err(e) => last_err = Some(Error::from(e)),
            }
        }

        Err(last_err.unwrap_or_else(|| {
            Error::Rutracker(rutracker::Error::AccessDenied(
                "у раздачи нет ни .torrent, ни magnet-ссылки".into(),
            ))
        }))
    }

    /// Добавляет раздачу в загрузки, скачивая `.torrent` или используя magnet.
    pub async fn add_from_topic(&self, topic_id: u64, options: AddOptions) -> Result<String> {
        let config = self.config();
        let engine = self.ensure_engine().await?;
        let topic = self
            .with_auth_retry(|client| async move { client.topic(topic_id).await })
            .await?;
        let source = self
            .pick_source(topic_id, &topic, options.prefer_magnet)
            .await?;

        // Если каталог не задан явно — используем каталог категории раздачи.
        let mut params = self.add_params(&config, &options);
        if params.output_folder.is_none()
            && let Some(category) = category_for_topic(&topic)
        {
            params.output_folder = config
                .downloads
                .category_paths
                .get(category)
                .map(str::to_owned);
        }

        let hash = engine.add(source, params).await.map_err(Error::from)?;
        self.record_history(topic_id, &topic.title, &hash);
        Ok(hash)
    }

    /// Добавляет торрент по magnet- или http-ссылке напрямую.
    pub async fn add_url(&self, url: String, options: AddOptions) -> Result<String> {
        let config = self.config();
        let engine = self.ensure_engine().await?;
        engine
            .add(Source::Url(url), self.add_params(&config, &options))
            .await
            .map_err(Error::from)
    }

    /// Ставит загрузку на паузу.
    pub async fn pause(&self, hashes: Vec<String>) -> Result<()> {
        let engine = self.ensure_engine().await?;
        for hash in &hashes {
            engine.pause(hash).await?;
        }
        Ok(())
    }

    /// Возобновляет загрузку.
    pub async fn resume(&self, hashes: Vec<String>) -> Result<()> {
        let engine = self.ensure_engine().await?;
        for hash in &hashes {
            engine.resume(hash).await?;
        }
        Ok(())
    }

    /// Удаляет загрузки (опционально с файлами).
    pub async fn remove(&self, hashes: Vec<String>, delete_files: bool) -> Result<()> {
        let engine = self.ensure_engine().await?;
        for hash in &hashes {
            engine.remove(hash, delete_files).await?;
        }
        Ok(())
    }

    // ── Избранное и история ─────────────────────────────────────────────

    /// Список избранных раздач.
    pub fn favorites(&self) -> Vec<FavoriteItem> {
        self.library
            .read()
            .expect("library lock")
            .favorites
            .iter()
            .cloned()
            .map(Into::into)
            .collect()
    }

    /// В избранном ли раздача.
    pub fn is_favorite(&self, topic_id: u64) -> bool {
        self.library
            .read()
            .expect("library lock")
            .is_favorite(topic_id)
    }

    /// Добавляет раздачу в избранное (запоминает сигнатуру для детекта обновлений).
    pub async fn add_favorite(&self, topic_id: u64) -> Result<()> {
        let topic = self
            .with_auth_retry(|client| async move { client.topic(topic_id).await })
            .await?;
        let now = now_unix();
        let record = FavoriteRecord {
            topic_id,
            title: topic.title.clone(),
            added_at: now,
            last_checked: now,
            signature: topic_signature(&topic),
            has_update: false,
        };
        self.library
            .write()
            .expect("library lock")
            .add_favorite(record);
        self.save_library();
        Ok(())
    }

    /// Убирает раздачу из избранного.
    pub fn remove_favorite(&self, topic_id: u64) {
        self.library
            .write()
            .expect("library lock")
            .remove_favorite(topic_id);
        self.save_library();
    }

    /// Сбрасывает отметку об обновлении избранной раздачи.
    pub fn clear_favorite_update(&self, topic_id: u64) {
        self.library
            .write()
            .expect("library lock")
            .clear_update(topic_id);
        self.save_library();
    }

    /// Проверяет обновления избранных раздач, перезапрашивая их страницы.
    ///
    /// Ошибки отдельных раздач не прерывают проверку (best-effort).
    pub async fn check_favorites(&self) -> Result<Vec<FavoriteItem>> {
        let ids: Vec<u64> = self
            .library
            .read()
            .expect("library lock")
            .favorites
            .iter()
            .map(|f| f.topic_id)
            .collect();

        for id in ids {
            let Ok(topic) = self
                .with_auth_retry(|client| async move { client.topic(id).await })
                .await
            else {
                continue;
            };
            let signature = topic_signature(&topic);
            let mut lib = self.library.write().expect("library lock");
            if let Some(fav) = lib.favorites.iter_mut().find(|f| f.topic_id == id) {
                fav.last_checked = now_unix();
                if fav.signature != signature {
                    fav.signature = signature;
                    fav.has_update = true;
                }
            }
        }

        self.save_library();
        Ok(self.favorites())
    }

    /// История скачиваний.
    pub fn history(&self) -> Vec<HistoryItem> {
        self.library
            .read()
            .expect("library lock")
            .history
            .iter()
            .cloned()
            .map(Into::into)
            .collect()
    }

    /// Удаляет запись истории.
    pub fn remove_history(&self, topic_id: u64) {
        self.library
            .write()
            .expect("library lock")
            .remove_history(topic_id);
        self.save_library();
    }

    /// Очищает историю скачиваний.
    pub fn clear_history(&self) {
        self.library.write().expect("library lock").clear_history();
        self.save_library();
    }

    fn record_history(&self, topic_id: u64, title: &str, hash: &str) {
        self.library
            .write()
            .expect("library lock")
            .add_history(HistoryRecord {
                topic_id,
                title: title.to_owned(),
                hash: hash.to_owned(),
                added_at: now_unix(),
            });
        self.save_library();
    }

    // ── Статус ──────────────────────────────────────────────────────────

    /// Отмечает состояние внешнего API (устанавливается слоем приложения).
    pub fn set_api_running(&self, running: bool) {
        self.api_running.store(running, Ordering::SeqCst);
    }

    /// Сводный статус подсистем.
    pub async fn status(&self) -> AppStatus {
        let (engine_running, active_downloads) = match self.engine_if_running().await {
            Some(engine) => {
                let active = engine
                    .torrents()
                    .iter()
                    .filter(|t| matches!(t.state, engine::TorrentState::Downloading))
                    .count() as u32;
                (true, active)
            }
            None => (false, 0),
        };

        let session = self.session_status().await.ok();
        AppStatus {
            engine_running,
            api_running: self.api_running.load(Ordering::SeqCst),
            logged_in: session.as_ref().map(|s| s.logged_in).unwrap_or(false),
            username: session.and_then(|s| s.username),
            active_downloads,
        }
    }
}

/// Определяет обобщённую категорию раздачи по названиям разделов (хлебных
/// крошек) и заголовку. Порядок важен: книги проверяются раньше музыки, чтобы
/// «аудиокниги» не относились к музыке.
fn category_for_topic(topic: &TopicPage) -> Option<&'static str> {
    let mut haystack = topic.title.to_lowercase();
    for forum in &topic.forum_path {
        haystack.push(' ');
        haystack.push_str(&forum.name.to_lowercase());
    }
    let has = |needles: &[&str]| needles.iter().any(|n| haystack.contains(n));

    if has(&["кино", "фильм", "сериал", "мультфильм"]) {
        Some("films")
    } else if has(&["книг", "литератур"]) {
        Some("books")
    } else if has(&["музык", "песн", "альбом", "дискограф"]) {
        Some("music")
    } else if has(&["игр", "game", "консол"]) {
        Some("games")
    } else {
        None
    }
}

/// Сигнатура состояния раздачи для детекта обновлений: дата регистрации +
/// размер. При переоформлении раздачи на трекере они меняются.
fn topic_signature(topic: &TopicPage) -> String {
    format!(
        "{}|{}",
        topic.stats.registered.clone().unwrap_or_default(),
        topic.stats.size_bytes.unwrap_or(0)
    )
}

/// Сводит список торрентов в общую статистику передачи.
fn summarize(torrents: &[engine::EngineTorrent]) -> TransferSummary {
    let mut summary = TransferSummary {
        total: torrents.len() as u32,
        ..Default::default()
    };
    for t in torrents {
        summary.dl_speed += t.dl_speed;
        summary.up_speed += t.up_speed;
        if matches!(t.state, engine::TorrentState::Downloading) {
            summary.active += 1;
        }
    }
    summary
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

    fn topic_in(forum: &str) -> rutracker::models::TopicPage {
        rutracker::models::TopicPage {
            id: 1,
            title: "Раздача".into(),
            forum_path: vec![rutracker::models::ForumRef {
                id: 1,
                name: forum.into(),
            }],
            magnet: None,
            has_torrent_file: true,
            stats: Default::default(),
            body: Vec::new(),
        }
    }

    #[test]
    fn category_detection_by_forum() {
        assert_eq!(
            category_for_topic(&topic_in("Зарубежное кино")),
            Some("films")
        );
        assert_eq!(category_for_topic(&topic_in("Игры для PC")), Some("games"));
        assert_eq!(category_for_topic(&topic_in("Поп-музыка")), Some("music"));
        // «Аудиокниги» — книги, а не музыка.
        assert_eq!(category_for_topic(&topic_in("Аудиокниги")), Some("books"));
        assert_eq!(category_for_topic(&topic_in("Прочее ПО")), None);
    }
}
