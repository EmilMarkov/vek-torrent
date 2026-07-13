//! [`AppCore`] — центральный оркестратор: конфигурация, клиент rutracker,
//! встроенный торрент-движок, доменные операции для UI и внешнего API.

use std::{
    path::PathBuf,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use engine::{AddParams, Engine, Source};
use rutracker::models::{
    CaptchaAnswer, ForumGroup, SearchPage, SearchRequest, SessionInfo, TopicPage,
};

use crate::{
    config::{self, AppConfig},
    error::{Error, Result},
    library::{
        self, ChangeEventRecord, FavoriteRecord, FavoriteSnapshot, HistoryRecord, Library, now_unix,
    },
    models::{
        AddOptions, AppStatus, CategoryItem, ChangeEventItem, DownloadItem, FavoriteItem,
        FileChangeItem, FileVersionInfo, FolderItem, HistoryItem, MirrorStatus, PatchInfo,
        TorrentFilesPreview, TransferSummary, VersionMatch,
    },
    tracked::{self, TrackedFile, TrackedVersions},
};

/// Таймаут проверки доступности зеркала.
const PROBE_TIMEOUT: Duration = Duration::from_secs(8);

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
    /// Сериализует автоматическое переключение зеркала между запросами.
    failover_lock: tokio::sync::Mutex<()>,
    /// Сериализует проверки обновлений отслеживаемого: параллельные проходы
    /// (фон + ручная кнопка + внешний API) дублировали бы события истории.
    favorites_check_lock: tokio::sync::Mutex<()>,
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
            failover_lock: tokio::sync::Mutex::new(()),
            favorites_check_lock: tokio::sync::Mutex::new(()),
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

    /// Выполняет операцию с двумя уровнями восстановления: при
    /// `NotAuthenticated` — вход и повтор; при недоступности зеркала —
    /// автоматическое переключение на живое зеркало и повтор.
    async fn with_auth_retry<F, Fut, T>(&self, make: F) -> Result<T>
    where
        F: Fn(rutracker::Client) -> Fut,
        Fut: std::future::Future<Output = std::result::Result<T, rutracker::Error>>,
    {
        let mirror_at_start = normalize_mirror(&self.config().rutracker.mirror);
        match self.auth_call(&make).await {
            Err(err) if self.mirror_failover_applies(&err) => {
                if self.try_failover(&mirror_at_start).await {
                    self.auth_call(&make).await
                } else {
                    Err(err)
                }
            }
            other => other,
        }
    }

    /// Одна попытка операции с авто-логином при протухшей сессии.
    async fn auth_call<F, Fut, T>(&self, make: &F) -> Result<T>
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

    /// Нужно ли пробовать сменить зеркало после этой ошибки.
    fn mirror_failover_applies(&self, err: &Error) -> bool {
        if !self.config().rutracker.auto_mirror {
            return false;
        }
        matches!(err, Error::Rutracker(e) if e.is_mirror_unreachable())
    }

    // ── Зеркала и обход блокировок ──────────────────────────────────────

    /// Проверяет доступность известных зеркал (и текущего) через прокси
    /// из конфигурации. Порядок: текущее зеркало, затем остальные.
    pub async fn check_mirrors(&self) -> Vec<MirrorStatus> {
        let config = self.config();
        let current = normalize_mirror(&config.rutracker.mirror);
        let proxy = proxy_of(&config);

        let mut candidates: Vec<String> = vec![current.clone()];
        for mirror in rutracker::DEFAULT_MIRRORS {
            let normalized = normalize_mirror(mirror);
            if !candidates.contains(&normalized) {
                candidates.push(normalized);
            }
        }

        let mut set = tokio::task::JoinSet::new();
        for (index, mirror) in candidates.iter().enumerate() {
            let mirror = mirror.clone();
            let proxy = proxy.clone();
            set.spawn(async move {
                let started = Instant::now();
                let result = probe_mirror(&mirror, proxy).await;
                (index, mirror, result, started.elapsed())
            });
        }

        let mut statuses: Vec<(usize, MirrorStatus)> = Vec::with_capacity(candidates.len());
        while let Some(joined) = set.join_next().await {
            let Ok((index, mirror, result, elapsed)) = joined else {
                continue;
            };
            let current_flag = mirror == current;
            statuses.push((
                index,
                match result {
                    Ok(()) => MirrorStatus {
                        url: mirror,
                        ok: true,
                        latency_ms: Some(elapsed.as_millis() as u64),
                        error: None,
                        current: current_flag,
                    },
                    Err(e) => MirrorStatus {
                        url: mirror,
                        ok: false,
                        latency_ms: None,
                        error: Some(e.to_string()),
                        current: current_flag,
                    },
                },
            ));
        }
        statuses.sort_by_key(|(index, _)| *index);
        statuses.into_iter().map(|(_, status)| status).collect()
    }

    /// Пытается уйти с отказавшего зеркала `failed_mirror` на живое.
    /// Возвращает `true`, если стоит повторить операцию (зеркало сменилось).
    async fn try_failover(&self, failed_mirror: &str) -> bool {
        let _guard = self.failover_lock.lock().await;

        let config = self.config();
        let current = normalize_mirror(&config.rutracker.mirror);
        let proxy = proxy_of(&config);

        // Пока ждали замок, зеркало уже сменили (другой запрос или
        // пользователь в настройках) — не перетираем чужой выбор, просто
        // повторяем операцию с новым клиентом.
        if current != failed_mirror {
            return true;
        }

        // Текущее зеркало нарочно НЕ пробуем: при частичной блокировке
        // (index.php отвечает, а нужный эндпоинт — нет) это заблокировало бы
        // переключение навсегда.
        for mirror in rutracker::DEFAULT_MIRRORS {
            let candidate = normalize_mirror(mirror);
            if candidate == current {
                continue;
            }
            if probe_mirror(&candidate, proxy.clone()).await.is_err() {
                continue;
            }
            let mut new_config = self.config();
            new_config.rutracker.mirror = candidate.clone();
            match self.update_config(new_config) {
                Ok(()) => {
                    tracing::info!("зеркало rutracker недоступно, переключено на {candidate}");
                    return true;
                }
                Err(e) => {
                    tracing::warn!("не удалось переключить зеркало на {candidate}: {e}");
                }
            }
        }
        false
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

    /// Скачивает `.torrent`-файл раздачи и сохраняет его по указанному пути.
    /// Возвращает оригинальное имя файла с трекера (если он его сообщил).
    pub async fn save_torrent_file(&self, topic_id: u64, path: String) -> Result<Option<String>> {
        let file = self
            .with_auth_retry(|client| async move { client.download_torrent(topic_id).await })
            .await?;
        std::fs::write(&path, &file.bytes)?;
        Ok(file.filename)
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

    /// Добавляет раздачу в отслеживаемое (запоминает снимок для детекта
    /// обновлений; при включённом отслеживании файлов — и версию файлов).
    pub async fn add_favorite(&self, topic_id: u64) -> Result<()> {
        let config = self.config();
        let topic = self
            .with_auth_retry(|client| async move { client.topic(topic_id).await })
            .await?;
        let now = now_unix();
        let snapshot = topic_snapshot(&topic, config.favorites.track_description);
        let record = FavoriteRecord {
            topic_id,
            title: topic.title.clone(),
            added_at: now,
            last_checked: now,
            signature: snapshot_signature(&snapshot),
            has_update: false,
            snapshot,
            changes: Vec::new(),
            history: Vec::new(),
        };
        self.library
            .write()
            .expect("library lock")
            .add_favorite(record);
        self.save_library();

        // Базовая версия файлов — точка отсчёта для патчей (best-effort).
        if config.favorites.track_files
            && topic.has_torrent_file
            && let Err(e) = self.record_file_version(topic_id).await
        {
            tracing::warn!("не удалось сохранить версию файлов раздачи {topic_id}: {e}");
        }
        Ok(())
    }

    /// Скачивает актуальный `.torrent` и фиксирует версию списка файлов.
    /// Возвращает описание файловых изменений (если они есть).
    async fn record_file_version(&self, topic_id: u64) -> Result<Option<String>> {
        let file = self
            .with_auth_retry(|client| async move { client.download_torrent(topic_id).await })
            .await?;

        // Пока качали .torrent, раздачу могли снять с отслеживания —
        // не воскрешаем только что удалённую историю версий.
        if !self.is_favorite(topic_id) {
            return Ok(None);
        }

        let files: Vec<TrackedFile> = engine::torrent_files(&file.bytes)?
            .into_iter()
            .map(|f| TrackedFile {
                path: f.path,
                size: f.size,
            })
            .collect();

        let mut versions = TrackedVersions::load(&self.app_dir, topic_id);
        let previous = versions.versions.last().map(|v| v.files.clone());
        if !versions.push_if_changed(now_unix(), files.clone()) {
            return Ok(None);
        }
        versions.save(&self.app_dir, topic_id)?;

        Ok(previous
            .and_then(|prev| tracked::describe_file_diff(&tracked::diff_files(&prev, &files))))
    }

    /// Убирает раздачу из отслеживаемого (история изменений и версии файлов
    /// удаляются — фронтенд предупреждает об этом пользователя).
    pub fn remove_favorite(&self, topic_id: u64) {
        self.library
            .write()
            .expect("library lock")
            .remove_favorite(topic_id);
        TrackedVersions::remove(&self.app_dir, topic_id);
        self.save_library();
    }

    /// История изменений отслеживаемой раздачи.
    pub fn favorite_history(&self, topic_id: u64) -> Vec<ChangeEventItem> {
        self.library
            .read()
            .expect("library lock")
            .favorites
            .iter()
            .find(|f| f.topic_id == topic_id)
            .map(|f| f.history.iter().cloned().map(Into::into).collect())
            .unwrap_or_default()
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
        // Параллельные проходы (фон + кнопка + API) читали бы одинаковое
        // «старое» состояние и дублировали события истории.
        let _guard = self.favorites_check_lock.lock().await;

        let ids: Vec<u64> = self
            .library
            .read()
            .expect("library lock")
            .favorites
            .iter()
            .map(|f| f.topic_id)
            .collect();

        let config = self.config();
        for id in ids {
            let Ok(topic) = self
                .with_auth_retry(|client| async move { client.topic(id).await })
                .await
            else {
                continue;
            };
            let mut snapshot = topic_snapshot(&topic, config.favorites.track_description);

            // Готовим сравнение по копии записи; сетевые операции (скачивание
            // .torrent для версии файлов) — строго вне замка библиотеки.
            let Some(old) = self
                .library
                .read()
                .expect("library lock")
                .favorites
                .iter()
                .find(|f| f.topic_id == id)
                .cloned()
            else {
                continue;
            };

            // Временная пропажа данных со страницы (гость/сбой/заглушка) — не
            // обновление: сохраняем последние известные значения.
            if snapshot.info_hash.is_none() {
                snapshot.info_hash = old.snapshot.info_hash.clone();
            }
            if snapshot.description_hash.is_none() {
                snapshot.description_hash = old.snapshot.description_hash.clone();
            }
            if snapshot.size_bytes.is_none() {
                snapshot.size_bytes = old.snapshot.size_bytes;
            }
            if snapshot.registered.is_none() {
                snapshot.registered = old.snapshot.registered.clone();
            }
            // Сигнатура — по снимку с унаследованными значениями, чтобы
            // пропавшие поля не выглядели изменением.
            let signature = snapshot_signature(&snapshot);

            // Записи со старых версий не имеют снимка — заполняем его без
            // пометки обновления, дальше сравниваем содержательный дифф.
            let had_snapshot = old.snapshot != FavoriteSnapshot::default();
            let mut changes = if had_snapshot {
                describe_changes(&old.snapshot, &snapshot)
            } else {
                Vec::new()
            };
            // При наличии снимка «обновлением» считается только содержательный
            // дифф — сигнатура покрывает лишь миграцию старых записей.
            let updated = if had_snapshot {
                !changes.is_empty()
            } else {
                old.signature != signature
            };

            // Файлы сверяются только при обнаруженном обновлении: скачивание
            // .torrent на каждую проверку тратило бы дневной лимит rutracker.
            if updated && config.favorites.track_files && topic.has_torrent_file {
                match self.record_file_version(id).await {
                    Ok(Some(diff)) => changes.push(diff),
                    Ok(None) => {}
                    Err(e) => tracing::warn!("версия файлов раздачи {id}: {e}"),
                }
            }

            let mut lib = self.library.write().expect("library lock");
            if let Some(fav) = lib.favorites.iter_mut().find(|f| f.topic_id == id) {
                fav.last_checked = now_unix();
                fav.signature = signature;
                fav.snapshot = snapshot;
                if updated {
                    fav.changes = changes.clone();
                    fav.has_update = true;
                    fav.title = topic.title.clone();
                    fav.history.insert(
                        0,
                        ChangeEventRecord {
                            at: now_unix(),
                            changes,
                        },
                    );
                    fav.history.truncate(library::HISTORY_CAP);
                }
            }
        }

        self.save_library();
        Ok(self.favorites())
    }

    // ── Патчи отслеживаемых раздач ──────────────────────────────────────
    //
    // Версии адресуются стабильным ключом `at` (unix-время фиксации), а не
    // позицией в списке: вытеснение старых версий по капу сдвигает индексы,
    // и позиционная адресация могла бы молча подменить базу патча.

    /// Сохранённые версии списков файлов раздачи (старые → новые).
    pub fn tracked_versions(&self, topic_id: u64) -> Vec<FileVersionInfo> {
        TrackedVersions::load(&self.app_dir, topic_id)
            .versions
            .iter()
            .enumerate()
            .map(|(index, v)| FileVersionInfo {
                index,
                at: v.at,
                file_count: v.files.len(),
                total_size: v.files.iter().map(|f| f.size).sum(),
            })
            .collect()
    }

    /// Патч: изменения файлов между версией пользователя (`base_at`) и
    /// последней сохранённой версией. Без сетевых запросов: последняя версия
    /// обновляется при каждой проверке обновлений.
    pub fn compute_patch(&self, topic_id: u64, base_at: i64) -> Result<PatchInfo> {
        let versions = TrackedVersions::load(&self.app_dir, topic_id);
        let base = versions
            .versions
            .iter()
            .find(|v| v.at == base_at)
            .ok_or_else(|| Error::Config("версия не найдена (список обновился?)".into()))?;
        let latest = versions
            .versions
            .last()
            .ok_or_else(|| Error::Config("нет сохранённых версий".into()))?;

        let changes = tracked::diff_files(&base.files, &latest.files);
        let download_size = changes
            .iter()
            .filter(|c| c.kind != tracked::FileChangeKind::Removed)
            .map(|c| c.size)
            .sum();

        Ok(PatchInfo {
            files: changes
                .into_iter()
                .map(|c| FileChangeItem {
                    path: c.path,
                    size: c.size,
                    kind: match c.kind {
                        tracked::FileChangeKind::Added => "added".to_owned(),
                        tracked::FileChangeKind::Changed => "changed".to_owned(),
                        tracked::FileChangeKind::Removed => "removed".to_owned(),
                    },
                })
                .collect(),
            download_size,
            base_at: base.at,
        })
    }

    /// Определяет версию раздачи по локальной папке пользователя: сравнивает
    /// файлы папки (относительный путь + размер) с каждой сохранённой версией.
    pub fn detect_version(&self, topic_id: u64, dir: String) -> Result<Vec<VersionMatch>> {
        let local = tracked::scan_dir(std::path::Path::new(&dir))?;
        if local.is_empty() {
            return Err(Error::Config("в выбранной папке нет файлов".into()));
        }
        let versions = TrackedVersions::load(&self.app_dir, topic_id);
        if versions.versions.is_empty() {
            return Err(Error::Config(
                "у раздачи нет сохранённых версий файлов".into(),
            ));
        }

        let mut matches: Vec<VersionMatch> = versions
            .versions
            .iter()
            .enumerate()
            .map(|(version, v)| VersionMatch {
                version,
                at: v.at,
                matched: tracked::match_score(&v.files, &local),
                total: v.files.len(),
            })
            .collect();
        // Лучшие совпадения — первыми (при равенстве очков — новее версия).
        matches.sort_by(|a, b| b.matched.cmp(&a.matched).then(b.version.cmp(&a.version)));
        Ok(matches)
    }

    /// Скачивает патч: добавляет актуальную раздачу в движок, выбрав только
    /// файлы, изменившиеся относительно базовой версии пользователя.
    /// Единственное скачивание `.torrent` — индексы `only_files` обязаны
    /// соответствовать именно тому торренту, который уходит в движок.
    pub async fn download_patch(
        &self,
        topic_id: u64,
        base_at: i64,
        options: AddOptions,
    ) -> Result<String> {
        let versions = TrackedVersions::load(&self.app_dir, topic_id);
        let base = versions
            .versions
            .iter()
            .find(|v| v.at == base_at)
            .ok_or_else(|| Error::Config("версия не найдена (список обновился?)".into()))?;

        let file = self
            .with_auth_retry(|client| async move { client.download_torrent(topic_id).await })
            .await?;
        let current = engine::torrent_files(&file.bytes)?;
        let current_tracked: Vec<TrackedFile> = current
            .iter()
            .map(|f| TrackedFile {
                path: f.path.clone(),
                size: f.size,
            })
            .collect();

        // Индексы файлов актуального торрента, которых не было (или которые
        // изменились) относительно базовой версии.
        let diff = tracked::diff_files(&base.files, &current_tracked);
        let changed: std::collections::HashSet<&str> = diff
            .iter()
            .filter(|c| c.kind != tracked::FileChangeKind::Removed)
            .map(|c| c.path.as_str())
            .collect();
        let only_files: Vec<usize> = current
            .iter()
            .filter(|f| changed.contains(f.path.as_str()))
            .map(|f| f.index)
            .collect();
        if only_files.is_empty() {
            return Err(Error::Config(
                "изменённых файлов нет — патч не требуется".into(),
            ));
        }

        let config = self.config();
        let engine_handle = self.ensure_engine().await?;
        let mut params = self.add_params(&config, &options);
        params.only_files = Some(only_files);

        // Название берём из записи отслеживаемого — лишний запрос страницы
        // раздачи не нужен.
        let title = self
            .library
            .read()
            .expect("library lock")
            .favorites
            .iter()
            .find(|f| f.topic_id == topic_id)
            .map(|f| f.title.clone())
            .unwrap_or_else(|| format!("Раздача {topic_id}"));

        let hash = engine_handle
            .add(Source::TorrentBytes(file.bytes), params)
            .await
            .map_err(Error::from)?;
        self.record_history(topic_id, &title, &hash);
        Ok(hash)
    }

    // ── Папки и категории ───────────────────────────────────────────────

    /// Пользовательские категории (при первом обращении создаются стандартные).
    pub fn user_categories(&self) -> Vec<CategoryItem> {
        let seeded = self
            .library
            .write()
            .expect("library lock")
            .seed_categories(|| uuid::Uuid::new_v4().simple().to_string());
        if seeded {
            self.save_library();
        }
        self.library
            .read()
            .expect("library lock")
            .categories
            .iter()
            .cloned()
            .map(Into::into)
            .collect()
    }

    /// Создаёт категорию.
    pub fn add_user_category(
        &self,
        name: String,
        color: String,
        forum_ids: Vec<i64>,
    ) -> Result<CategoryItem> {
        let name = valid_name(name)?;
        let record = library::CategoryRecord {
            id: uuid::Uuid::new_v4().simple().to_string(),
            name,
            color,
            forum_ids,
        };
        self.library
            .write()
            .expect("library lock")
            .add_category(record.clone());
        self.save_library();
        Ok(record.into())
    }

    /// Обновляет категорию: имя, цвет, набор разделов rutracker.
    pub fn update_user_category(
        &self,
        id: String,
        name: String,
        color: String,
        forum_ids: Vec<i64>,
    ) -> Result<()> {
        let name = valid_name(name)?;
        let found = self
            .library
            .write()
            .expect("library lock")
            .update_category(&id, name, color, forum_ids);
        if !found {
            return Err(Error::Config("категория не найдена".into()));
        }
        self.save_library();
        Ok(())
    }

    /// Удаляет категорию (папки с ней остаются без категории).
    pub fn remove_user_category(&self, id: String) {
        self.library
            .write()
            .expect("library lock")
            .remove_category(&id);
        self.save_library();
    }

    /// Папки с раздачами (категории развёрнуты).
    pub fn folders(&self) -> Vec<FolderItem> {
        let lib = self.library.read().expect("library lock");
        lib.folders
            .iter()
            .map(|folder| FolderItem {
                id: folder.id.clone(),
                name: folder.name.clone(),
                category: folder
                    .category_id
                    .as_deref()
                    .and_then(|id| lib.categories.iter().find(|c| c.id == id))
                    .cloned()
                    .map(Into::into),
                topics: folder.topics.iter().cloned().map(Into::into).collect(),
                created_at: folder.created_at,
            })
            .collect()
    }

    /// Создаёт папку.
    pub fn add_folder(&self, name: String, category_id: Option<String>) -> Result<()> {
        let name = valid_name(name)?;
        let record = library::FolderRecord {
            id: uuid::Uuid::new_v4().simple().to_string(),
            name,
            category_id: category_id.filter(|c| !c.is_empty()),
            topics: Vec::new(),
            created_at: now_unix(),
        };
        self.library
            .write()
            .expect("library lock")
            .add_folder(record);
        self.save_library();
        Ok(())
    }

    /// Переименовывает папку / меняет её категорию.
    pub fn update_folder(
        &self,
        id: String,
        name: String,
        category_id: Option<String>,
    ) -> Result<()> {
        let name = valid_name(name)?;
        let found = self.library.write().expect("library lock").update_folder(
            &id,
            name,
            category_id.filter(|c| !c.is_empty()),
        );
        if !found {
            return Err(Error::Config("папка не найдена".into()));
        }
        self.save_library();
        Ok(())
    }

    /// Удаляет папку.
    pub fn remove_folder(&self, id: String) {
        self.library
            .write()
            .expect("library lock")
            .remove_folder(&id);
        self.save_library();
    }

    /// Добавляет раздачу в папку.
    pub fn add_topic_to_folder(
        &self,
        folder_id: String,
        topic_id: u64,
        title: String,
    ) -> Result<()> {
        let topic = library::FolderTopicRecord {
            topic_id,
            title,
            added_at: now_unix(),
        };
        let found = self
            .library
            .write()
            .expect("library lock")
            .add_topic_to_folder(&folder_id, topic);
        if !found {
            return Err(Error::Config("папка не найдена".into()));
        }
        self.save_library();
        Ok(())
    }

    /// Убирает раздачу из папки.
    pub fn remove_topic_from_folder(&self, folder_id: String, topic_id: u64) {
        self.library
            .write()
            .expect("library lock")
            .remove_topic_from_folder(&folder_id, topic_id);
        self.save_library();
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
/// размер (по снимку с уже унаследованными значениями). Используется для
/// миграции записей со старых версий приложения, у которых нет снимка.
fn snapshot_signature(snapshot: &FavoriteSnapshot) -> String {
    format!(
        "{}|{}",
        snapshot.registered.clone().unwrap_or_default(),
        snapshot.size_bytes.unwrap_or(0)
    )
}

/// Снимок отслеживаемых полей раздачи.
fn topic_snapshot(topic: &TopicPage, track_description: bool) -> FavoriteSnapshot {
    FavoriteSnapshot {
        title: topic.title.clone(),
        size_bytes: topic.stats.size_bytes,
        registered: topic.stats.registered.clone(),
        info_hash: topic.magnet.as_deref().and_then(magnet_info_hash),
        description_hash: track_description.then(|| {
            sha1_smol::Sha1::from(topic.body_html.as_bytes())
                .digest()
                .to_string()
        }),
    }
}

/// info-hash из magnet-ссылки (`xt=urn:btih:HASH`).
fn magnet_info_hash(magnet: &str) -> Option<String> {
    let start = magnet.find("urn:btih:")? + "urn:btih:".len();
    let hash: String = magnet[start..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect();
    (!hash.is_empty()).then(|| hash.to_ascii_lowercase())
}

/// Человекочитаемое описание разницы между снимками раздачи.
fn describe_changes(old: &FavoriteSnapshot, new: &FavoriteSnapshot) -> Vec<String> {
    let mut changes = Vec::new();
    // «Перезалит» — только при смене одного известного хэша на другой:
    // появление/пропажа magnet сама по себе обновлением не считается.
    if let (Some(before), Some(after)) = (&old.info_hash, &new.info_hash)
        && before != after
    {
        changes.push("торрент перезалит".to_owned());
    }
    if old.size_bytes != new.size_bytes
        && let (Some(before), Some(after)) = (old.size_bytes, new.size_bytes)
    {
        changes.push(format!(
            "размер: {} → {}",
            format_size(before),
            format_size(after)
        ));
    }
    if old.registered != new.registered
        && let (Some(before), Some(after)) = (&old.registered, &new.registered)
    {
        changes.push(format!("дата регистрации: {before} → {after}"));
    }
    if old.title != new.title && !old.title.is_empty() {
        changes.push(format!(
            "название: «{}» → «{}»",
            truncate_title(&old.title),
            truncate_title(&new.title)
        ));
    }
    if let (Some(before), Some(after)) = (&old.description_hash, &new.description_hash)
        && before != after
    {
        changes.push("изменилось описание раздачи".to_owned());
    }
    changes
}

/// Укорачивает название для описания изменений.
fn truncate_title(title: &str) -> String {
    const MAX: usize = 60;
    if title.chars().count() <= MAX {
        title.to_owned()
    } else {
        let cut: String = title.chars().take(MAX).collect();
        format!("{cut}…")
    }
}

/// Размер в человекочитаемом виде (для описаний изменений).
fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["Б", "КБ", "МБ", "ГБ", "ТБ"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} Б")
    } else {
        format!("{value:.2} {}", UNITS[unit])
    }
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
    rutracker::Client::builder()
        .base_url(config.rutracker.mirror.clone())
        .proxy(proxy_of(config))
        .cookie_path(Some(config::cookies_file(app_dir)))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(Error::from_rutracker)
}

/// Прокси из конфигурации (пустая строка — напрямую).
fn proxy_of(config: &AppConfig) -> Option<String> {
    let proxy = config.rutracker.proxy.trim();
    (!proxy.is_empty()).then(|| proxy.to_owned())
}

/// Приводит адрес зеркала к сравнимому виду: без хвостового `/` и суффикса
/// `/forum` (клиент добавляет его сам — см. `normalize_base` в rutracker).
fn normalize_mirror(mirror: &str) -> String {
    let trimmed = mirror.trim().trim_end_matches('/');
    trimmed.strip_suffix("/forum").unwrap_or(trimmed).to_owned()
}

/// Проверяет доступность зеркала одноразовым клиентом (без файла куков,
/// чтобы не трогать сохранённую сессию) через тот же прокси.
async fn probe_mirror(mirror: &str, proxy: Option<String>) -> rutracker::Result<()> {
    rutracker::Client::builder()
        .base_url(mirror)
        .proxy(proxy)
        .timeout(PROBE_TIMEOUT)
        .build()?
        .probe()
        .await
}

/// Валидирует пользовательское имя (папки/категории): непустое, разумной длины.
fn valid_name(name: String) -> Result<String> {
    let name = name.trim().to_owned();
    if name.is_empty() {
        return Err(Error::Config("имя не может быть пустым".into()));
    }
    if name.chars().count() > 100 {
        return Err(Error::Config("имя слишком длинное".into()));
    }
    Ok(name)
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
            author: None,
            magnet: None,
            has_torrent_file: true,
            stats: Default::default(),
            body_html: String::new(),
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
