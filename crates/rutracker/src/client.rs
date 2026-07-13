//! HTTP-клиент rutracker: сессия, логин, поиск, раздачи, скачивание `.torrent`.

use std::{
    fs,
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use reqwest::{header, redirect};
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use url::Url;

use crate::{
    Result,
    encoding::{cp1251_form_urlencode, decode_body},
    error::Error,
    models::{
        CaptchaAnswer, ForumGroup, SearchPage, SearchRequest, SessionInfo, TopicPage, TorrentFile,
    },
    parse,
};

/// Известные зеркала rutracker.
pub const DEFAULT_MIRRORS: &[&str] = &[
    "https://rutracker.org",
    "https://rutracker.net",
    "https://rutracker.nl",
];

const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:140.0) Gecko/20100101 Firefox/140.0";

/// Билдер [`Client`].
#[derive(Debug, Clone)]
pub struct ClientBuilder {
    base_url: String,
    proxy: Option<String>,
    cookie_path: Option<PathBuf>,
    timeout: Duration,
    user_agent: String,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_MIRRORS[0].to_owned(),
            proxy: None,
            cookie_path: None,
            timeout: Duration::from_secs(30),
            user_agent: DEFAULT_USER_AGENT.to_owned(),
        }
    }
}

impl ClientBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Базовый адрес зеркала, например `https://rutracker.org`.
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Прокси (`http://…`, `socks5://…`, `socks5h://…`) либо `None` — напрямую.
    pub fn proxy(mut self, proxy: Option<String>) -> Self {
        self.proxy = proxy.filter(|p| !p.trim().is_empty());
        self
    }

    /// Файл для персистентных куков сессии.
    pub fn cookie_path(mut self, path: Option<PathBuf>) -> Self {
        self.cookie_path = path;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    pub fn build(self) -> Result<Client> {
        let base = normalize_base(&self.base_url)?;

        let store = match &self.cookie_path {
            Some(path) if path.exists() => load_cookie_store(path),
            _ => CookieStore::default(),
        };
        let cookies = Arc::new(CookieStoreMutex::new(store));

        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::ACCEPT_LANGUAGE,
            header::HeaderValue::from_static("ru,en;q=0.8"),
        );

        let mut builder = reqwest::Client::builder()
            .user_agent(&self.user_agent)
            .default_headers(headers)
            .cookie_provider(Arc::clone(&cookies))
            .timeout(self.timeout)
            .redirect(redirect::Policy::limited(10));

        if let Some(proxy) = &self.proxy {
            let proxy = reqwest::Proxy::all(proxy)
                .map_err(|e| Error::Url(format!("некорректный прокси: {e}")))?;
            builder = builder.proxy(proxy);
        }

        Ok(Client {
            http: builder.build()?,
            base,
            cookies,
            cookie_path: self.cookie_path,
        })
    }
}

/// Клиент rutracker. Дёшево клонируется, безопасен для конкурентного использования.
#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    base: Url,
    cookies: Arc<CookieStoreMutex>,
    cookie_path: Option<PathBuf>,
}

impl Client {
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Базовый URL форума (`…/forum/`).
    pub fn base(&self) -> &Url {
        &self.base
    }

    fn url(&self, path: &str) -> Result<Url> {
        self.base
            .join(path)
            .map_err(|e| Error::Url(format!("{path}: {e}")))
    }

    async fn get_html(&self, url: Url) -> Result<String> {
        let resp = self.http.get(url).send().await?.error_for_status()?;
        let content_type = header_str(resp.headers(), header::CONTENT_TYPE);
        let bytes = resp.bytes().await?;
        Ok(decode_body(&bytes, content_type.as_deref()))
    }

    async fn post_form_html(&self, url: Url, body: String) -> Result<String> {
        let resp = self
            .http
            .post(url)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await?
            .error_for_status()?;
        let content_type = header_str(resp.headers(), header::CONTENT_TYPE);
        let bytes = resp.bytes().await?;
        Ok(decode_body(&bytes, content_type.as_deref()))
    }

    /// Вход на трекер.
    ///
    /// При требовании капчи возвращает [`Error::CaptchaRequired`] с данными
    /// вызова; повторите вход, передав [`CaptchaAnswer`].
    pub async fn login(
        &self,
        username: &str,
        password: &str,
        captcha: Option<&CaptchaAnswer>,
    ) -> Result<()> {
        // Начинаем с чистого стора: локально живая, но протухшая на сервере
        // кука не должна маскировать неудачный вход под успех.
        {
            let mut store = self
                .cookies
                .lock()
                .map_err(|_| Error::CookieStore("хранилище куков повреждено".into()))?;
            *store = CookieStore::default();
        }

        let mut pairs: Vec<(String, String)> = vec![
            ("login_username".to_owned(), username.to_owned()),
            ("login_password".to_owned(), password.to_owned()),
            ("login".to_owned(), "Вход".to_owned()),
        ];
        if let Some(answer) = captcha {
            pairs.push(("cap_sid".to_owned(), answer.sid.clone()));
            pairs.push((answer.code_field.clone(), answer.value.clone()));
        }

        let body = cp1251_form_urlencode(&pairs);
        let html = self.post_form_html(self.url("login.php")?, body).await?;

        if self.has_session_cookie() {
            if let Err(e) = self.save_cookies() {
                tracing::warn!("не удалось сохранить куки сессии: {e}");
            }
            return Ok(());
        }

        Err(parse::login::classify_failure(&html, &self.base))
    }

    /// Есть ли в хранилище живой сессионный кук.
    pub fn has_session_cookie(&self) -> bool {
        let Ok(store) = self.cookies.lock() else {
            return false;
        };
        store.iter_unexpired().any(|c| c.name() == "bb_session")
    }

    /// Проверяет состояние сессии по главной странице форума.
    pub async fn session_info(&self) -> Result<SessionInfo> {
        let html = self.get_html(self.url("index.php")?).await?;
        Ok(parse::login::session_info(&html))
    }

    /// Проверяет, что зеркало отвечает и выглядит как rutracker.
    ///
    /// Отличает живое зеркало от заглушки провайдера при блокировке:
    /// заглушка не содержит ни формы логина, ни имени вошедшего пользователя.
    pub async fn probe(&self) -> Result<()> {
        let html = self.get_html(self.url("index.php")?).await?;
        if parse::login::looks_like_rutracker(&html) {
            Ok(())
        } else {
            Err(Error::Parse(
                "страница не похожа на rutracker (заглушка блокировки?)".into(),
            ))
        }
    }

    /// Локальный выход: очистка куков и их файла.
    pub fn logout(&self) -> Result<()> {
        {
            let mut store = self
                .cookies
                .lock()
                .map_err(|_| Error::CookieStore("хранилище куков повреждено".into()))?;
            *store = CookieStore::default();
        }
        self.save_cookies()
    }

    /// Сохраняет куки в файл (если путь задан).
    pub fn save_cookies(&self) -> Result<()> {
        let Some(path) = &self.cookie_path else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let store = self
            .cookies
            .lock()
            .map_err(|_| Error::CookieStore("хранилище куков повреждено".into()))?;
        let mut writer = BufWriter::new(fs::File::create(path)?);
        // Сохраняем и «сессионные» куки без Expires: сессия должна переживать
        // перезапуск приложения.
        cookie_store::serde::json::save_incl_expired_and_nonpersistent(&store, &mut writer)
            .map_err(|e| Error::CookieStore(e.to_string()))
    }

    /// Поиск по трекеру. Требует активной сессии.
    ///
    /// Страницы внутри серверной сессии (`search_id`) читаются GET-запросом;
    /// новая сессия создаётся POST-ом. Если запрошена страница с ненулевым
    /// смещением без токена сессии (или сессия на сервере истекла), POST
    /// создаёт новую сессию, после чего нужная страница дочитывается по GET.
    pub async fn search(&self, req: &SearchRequest) -> Result<SearchPage> {
        if let Some(search_id) = &req.search_id {
            let page = self.search_session_page(search_id, req.offset).await?;
            return Ok(page);
        }

        // Новая серверная сессия поиска (первая страница).
        let mut pairs: Vec<(String, String)> = vec![
            ("nm".to_owned(), req.query.clone()),
            ("o".to_owned(), req.sort.code().to_string()),
            ("s".to_owned(), req.order.code().to_string()),
        ];
        if let Some(author) = req.author.as_deref().filter(|a| !a.trim().is_empty()) {
            pairs.push(("pn".to_owned(), author.trim().to_owned()));
        }
        for forum in &req.forums {
            pairs.push(("f[]".to_owned(), forum.to_string()));
        }
        let body = cp1251_form_urlencode(&pairs);
        let html = self.post_form_html(self.url("tracker.php")?, body).await?;
        if parse::login::is_login_page(&html) {
            return Err(Error::NotAuthenticated);
        }
        let first = parse::search::parse_search_page(&html, 0)?;

        // Нужна не первая страница — дочитываем её в свежесозданной сессии.
        if req.offset > 0
            && let Some(search_id) = &first.search_id
        {
            return self.search_session_page(search_id, req.offset).await;
        }
        Ok(first)
    }

    /// Страница результатов существующей серверной сессии поиска.
    async fn search_session_page(&self, search_id: &str, offset: u32) -> Result<SearchPage> {
        let mut url = self.url("tracker.php")?;
        url.query_pairs_mut()
            .append_pair("search_id", search_id)
            .append_pair("start", &offset.to_string());
        let html = self.get_html(url).await?;
        if parse::login::is_login_page(&html) {
            return Err(Error::NotAuthenticated);
        }
        parse::search::parse_search_page(&html, offset)
    }

    /// Загружает и разбирает страницу раздачи.
    pub async fn topic(&self, id: u64) -> Result<TopicPage> {
        let mut url = self.url("viewtopic.php")?;
        url.query_pairs_mut().append_pair("t", &id.to_string());
        let html = self.get_html(url).await?;
        if parse::login::is_login_page(&html) {
            return Err(Error::NotAuthenticated);
        }
        parse::topic::parse_topic_page(&html, id, &self.base)
    }

    /// Дерево форумов для фильтра поиска. Требует активной сессии.
    pub async fn categories(&self) -> Result<Vec<ForumGroup>> {
        let html = self.get_html(self.url("tracker.php")?).await?;
        if parse::login::is_login_page(&html) {
            return Err(Error::NotAuthenticated);
        }
        parse::categories::parse_forum_select(&html)
    }

    /// Скачивает `.torrent`-файл раздачи. Требует активной сессии.
    pub async fn download_torrent(&self, topic_id: u64) -> Result<TorrentFile> {
        let mut url = self.url("dl.php")?;
        url.query_pairs_mut()
            .append_pair("t", &topic_id.to_string());

        let resp = self.http.get(url).send().await?.error_for_status()?;
        let content_type = header_str(resp.headers(), header::CONTENT_TYPE);
        let filename = header_str(resp.headers(), header::CONTENT_DISPOSITION)
            .as_deref()
            .and_then(parse_disposition_filename);
        let bytes = resp.bytes().await?.to_vec();

        let looks_like_torrent = content_type
            .as_deref()
            .is_some_and(|ct| ct.contains("bittorrent"))
            || bytes.first() == Some(&b'd');

        if !looks_like_torrent {
            let html = decode_body(&bytes, content_type.as_deref());
            if parse::login::is_login_page(&html) {
                return Err(Error::NotAuthenticated);
            }
            return Err(Error::AccessDenied(
                "трекер не отдал .torrent-файл (возможно, исчерпан лимит скачиваний)".into(),
            ));
        }

        Ok(TorrentFile { bytes, filename })
    }

    /// Скачивает изображение (капчу, постер) через ту же сессию и прокси.
    pub async fn fetch_image(&self, url: &str) -> Result<(Vec<u8>, Option<String>)> {
        let absolute = Url::parse(url)
            .or_else(|_| self.base.join(url))
            .map_err(|e| Error::Url(format!("{url}: {e}")))?;
        let resp = self.http.get(absolute).send().await?.error_for_status()?;
        let content_type = header_str(resp.headers(), header::CONTENT_TYPE);
        Ok((resp.bytes().await?.to_vec(), content_type))
    }
}

fn header_str(headers: &header::HeaderMap, name: header::HeaderName) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

fn parse_disposition_filename(disposition: &str) -> Option<String> {
    let start = disposition.find("filename=")? + "filename=".len();
    let rest = disposition[start..].trim();
    let name = rest
        .trim_start_matches('"')
        .split('"')
        .next()
        .unwrap_or(rest)
        .split(';')
        .next()
        .unwrap_or(rest)
        .trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_owned())
    }
}

fn normalize_base(input: &str) -> Result<Url> {
    let trimmed = input.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(Error::Url("пустой адрес зеркала".into()));
    }
    let with_forum = if trimmed.ends_with("/forum") {
        format!("{trimmed}/")
    } else {
        format!("{trimmed}/forum/")
    };
    Url::parse(&with_forum).map_err(|e| Error::Url(format!("{input}: {e}")))
}

fn load_cookie_store(path: &Path) -> CookieStore {
    match fs::File::open(path) {
        Ok(file) => match cookie_store::serde::json::load_all(BufReader::new(file)) {
            Ok(store) => store,
            Err(e) => {
                tracing::warn!("файл куков повреждён, начинаем с чистой сессии: {e}");
                CookieStore::default()
            }
        },
        Err(e) => {
            tracing::warn!("не удалось открыть файл куков: {e}");
            CookieStore::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_base_url() {
        assert_eq!(
            normalize_base("https://rutracker.org").unwrap().as_str(),
            "https://rutracker.org/forum/"
        );
        assert_eq!(
            normalize_base("https://rutracker.org/").unwrap().as_str(),
            "https://rutracker.org/forum/"
        );
        assert_eq!(
            normalize_base("https://rutracker.org/forum/")
                .unwrap()
                .as_str(),
            "https://rutracker.org/forum/"
        );
    }

    #[test]
    fn rejects_bad_base_url() {
        assert!(normalize_base("не-адрес").is_err());
        assert!(normalize_base("").is_err());
    }

    #[test]
    fn parses_content_disposition_filename() {
        assert_eq!(
            parse_disposition_filename(r#"attachment; filename="[rutracker.org].t123.torrent""#),
            Some("[rutracker.org].t123.torrent".to_owned())
        );
        assert_eq!(
            parse_disposition_filename("attachment; filename=plain.torrent; size=1"),
            Some("plain.torrent".to_owned())
        );
        assert_eq!(parse_disposition_filename("attachment"), None);
    }
}
