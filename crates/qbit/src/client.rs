//! HTTP-клиент qBittorrent Web API v2.

use std::time::Duration;

use reqwest::{StatusCode, multipart};
use url::Url;

use crate::{
    Result,
    error::Error,
    models::{
        AddTorrent, Category, TorrentInfo, TorrentSource, TorrentsQuery, TransferInfo,
    },
};

/// Конфигурация клиента.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Базовый адрес Web UI, например `http://127.0.0.1:8080`.
    pub base_url: String,
    pub timeout: Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:8080".to_owned(),
            timeout: Duration::from_secs(15),
        }
    }
}

/// Клиент qBittorrent. Дёшево клонируется.
#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    base: Url,
}

impl Client {
    /// Создаёт клиент. При `LocalHostAuth=false` (наш sidecar) вход не нужен.
    pub fn new(config: ClientConfig) -> Result<Self> {
        let base = normalize_base(&config.base_url)?;
        let http = reqwest::Client::builder()
            .cookie_store(true)
            .timeout(config.timeout)
            .build()?;
        Ok(Self { http, base })
    }

    fn endpoint(&self, path: &str) -> Result<Url> {
        self.base
            .join(path)
            .map_err(|e| Error::Url(format!("{path}: {e}")))
    }

    /// Аутентификация в Web API (нужна только при включённой авторизации).
    pub async fn login(&self, username: &str, password: &str) -> Result<()> {
        let resp = self
            .http
            .post(self.endpoint("api/v2/auth/login")?)
            // Referer обязателен, иначе qBittorrent вернёт 403.
            .header(reqwest::header::REFERER, self.base.as_str())
            .form(&[("username", username), ("password", password)])
            .send()
            .await?;

        if resp.status() == StatusCode::FORBIDDEN {
            return Err(Error::Banned);
        }
        let body = resp.error_for_status()?.text().await?;
        if body.trim().eq_ignore_ascii_case("Ok.") {
            Ok(())
        } else {
            Err(Error::AuthFailed)
        }
    }

    async fn get_text(&self, path: &str) -> Result<String> {
        let resp = self.http.get(self.endpoint(path)?).send().await?;
        Ok(guard_status(resp)?.text().await?)
    }

    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: Url) -> Result<T> {
        let resp = self.http.get(url).send().await?;
        let text = guard_status(resp)?.text().await?;
        serde_json::from_str(&text).map_err(|e| Error::Decode(e.to_string()))
    }

    /// Версия приложения qBittorrent (например, `v5.0.3`).
    pub async fn version(&self) -> Result<String> {
        Ok(self.get_text("api/v2/app/version").await?.trim().to_owned())
    }

    /// Версия Web API (например, `2.11.2`).
    pub async fn web_api_version(&self) -> Result<String> {
        Ok(self
            .get_text("api/v2/app/webapiVersion")
            .await?
            .trim()
            .to_owned())
    }

    /// Доступен ли Web API (быстрая health-проверка).
    pub async fn is_alive(&self) -> bool {
        self.web_api_version().await.is_ok()
    }

    /// Список торрентов по фильтру.
    pub async fn torrents(&self, query: &TorrentsQuery) -> Result<Vec<TorrentInfo>> {
        let mut url = self.endpoint("api/v2/torrents/info")?;
        {
            let mut qp = url.query_pairs_mut();
            qp.append_pair("filter", query.filter.as_str());
            if let Some(category) = &query.category {
                qp.append_pair("category", category);
            }
            if let Some(tag) = &query.tag {
                qp.append_pair("tag", tag);
            }
            if let Some(sort) = &query.sort {
                qp.append_pair("sort", sort);
            }
            if query.reverse {
                qp.append_pair("reverse", "true");
            }
            if let Some(limit) = query.limit {
                qp.append_pair("limit", &limit.to_string());
            }
            if let Some(offset) = query.offset {
                qp.append_pair("offset", &offset.to_string());
            }
            if !query.hashes.is_empty() {
                qp.append_pair("hashes", &query.hashes.join("|"));
            }
        }
        self.get_json(url).await
    }

    /// Глобальная статистика передачи.
    pub async fn transfer_info(&self) -> Result<TransferInfo> {
        let url = self.endpoint("api/v2/transfer/info")?;
        self.get_json(url).await
    }

    /// Категории qBittorrent.
    pub async fn categories(&self) -> Result<Vec<Category>> {
        let url = self.endpoint("api/v2/torrents/categories")?;
        // API отдаёт объект { имя: { name, savePath } }.
        let map: std::collections::HashMap<String, Category> = self.get_json(url).await?;
        let mut categories: Vec<Category> = map
            .into_iter()
            .map(|(name, mut cat)| {
                if cat.name.is_empty() {
                    cat.name = name;
                }
                cat
            })
            .collect();
        categories.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(categories)
    }

    /// Добавляет торрент(ы).
    pub async fn add_torrent(&self, add: &AddTorrent) -> Result<()> {
        if add.sources.is_empty() {
            return Err(Error::OperationFailed("не указан источник торрента".into()));
        }

        let mut form = multipart::Form::new();
        let mut urls = Vec::new();
        for source in &add.sources {
            match source {
                TorrentSource::Url(url) => urls.push(url.clone()),
                TorrentSource::File { filename, bytes } => {
                    let part = multipart::Part::bytes(bytes.clone())
                        .file_name(filename.clone())
                        .mime_str("application/x-bittorrent")?;
                    form = form.part("torrents", part);
                }
            }
        }
        if !urls.is_empty() {
            form = form.text("urls", urls.join("\n"));
        }
        if let Some(save_path) = &add.save_path {
            form = form.text("savepath", save_path.clone());
        }
        if let Some(category) = &add.category {
            form = form.text("category", category.clone());
        }
        if !add.tags.is_empty() {
            form = form.text("tags", add.tags.join(","));
        }
        if add.stopped {
            // qBittorrent принимает и `stopped`, и устаревшее `paused`.
            form = form.text("stopped", "true").text("paused", "true");
        }
        if add.skip_checking {
            form = form.text("skip_checking", "true");
        }

        let resp = self
            .http
            .post(self.endpoint("api/v2/torrents/add")?)
            .header(reqwest::header::REFERER, self.base.as_str())
            .multipart(form)
            .send()
            .await?;

        let resp = guard_status(resp)?;
        let body = resp.text().await?;
        // qBittorrent отвечает "Fails." при неверном/повреждённом торренте.
        if body.trim().eq_ignore_ascii_case("Fails.") {
            return Err(Error::OperationFailed(
                "qBittorrent не принял торрент (неверная ссылка или файл)".into(),
            ));
        }
        Ok(())
    }

    /// Удаляет торренты (опционально с файлами).
    pub async fn delete(&self, hashes: &[String], delete_files: bool) -> Result<()> {
        self.post_form(
            "api/v2/torrents/delete",
            &[
                ("hashes", hashes.join("|")),
                ("deleteFiles", delete_files.to_string()),
            ],
        )
        .await
    }

    /// Останавливает торренты (v5 `stop`, откат на v4 `pause`).
    pub async fn stop(&self, hashes: &[String]) -> Result<()> {
        self.stop_start(hashes, "stop", "pause").await
    }

    /// Запускает торренты (v5 `start`, откат на v4 `resume`).
    pub async fn start(&self, hashes: &[String]) -> Result<()> {
        self.stop_start(hashes, "start", "resume").await
    }

    async fn stop_start(&self, hashes: &[String], v5: &str, v4: &str) -> Result<()> {
        let hashes_value = hashes.join("|");
        // На современном qBittorrent (v5) метод есть; на v4 он вернёт 404/405 —
        // только в этом случае повторяем запрос со старым именем.
        let v5_path = format!("api/v2/torrents/{v5}");
        match self
            .post_form_status(&v5_path, &[("hashes", hashes_value.clone())])
            .await?
        {
            None => Ok(()),
            Some(status) if is_method_absent(status) => {
                self.post_form(&format!("api/v2/torrents/{v4}"), &[("hashes", hashes_value)])
                    .await
            }
            Some(status) => Err(Error::OperationFailed(format!("{v5_path}: код ответа {status}"))),
        }
    }

    /// Пересчитать приоритет (поставить в начало очереди).
    pub async fn set_top_priority(&self, hashes: &[String]) -> Result<()> {
        self.post_form("api/v2/torrents/topPrio", &[("hashes", hashes.join("|"))])
            .await
    }

    /// Устанавливает категорию для торрентов.
    pub async fn set_category(&self, hashes: &[String], category: &str) -> Result<()> {
        self.post_form(
            "api/v2/torrents/setCategory",
            &[
                ("hashes", hashes.join("|")),
                ("category", category.to_owned()),
            ],
        )
        .await
    }

    /// Корректно завершает работу qBittorrent (используется sidecar-менеджером).
    pub async fn shutdown(&self) -> Result<()> {
        let resp = self
            .http
            .post(self.endpoint("api/v2/app/shutdown")?)
            .header(reqwest::header::REFERER, self.base.as_str())
            .send()
            .await?;
        guard_status(resp)?;
        Ok(())
    }

    async fn post_form(&self, path: &str, params: &[(&str, String)]) -> Result<()> {
        let resp = self
            .http
            .post(self.endpoint(path)?)
            .header(reqwest::header::REFERER, self.base.as_str())
            .form(params)
            .send()
            .await?;

        match resp.status() {
            StatusCode::FORBIDDEN => Err(Error::Forbidden),
            status if status.is_success() => Ok(()),
            status => Err(Error::OperationFailed(format!("{path}: код ответа {status}"))),
        }
    }

    /// Как [`Self::post_form`], но при неуспехе возвращает статус, чтобы вызвать
    /// сторона могла отличить «метода нет» (404/405) от прочих ошибок.
    async fn post_form_status(
        &self,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<Option<StatusCode>> {
        let resp = self
            .http
            .post(self.endpoint(path)?)
            .header(reqwest::header::REFERER, self.base.as_str())
            .form(params)
            .send()
            .await?;

        match resp.status() {
            StatusCode::FORBIDDEN => Err(Error::Forbidden),
            status if status.is_success() => Ok(None),
            status => Ok(Some(status)),
        }
    }
}

/// Означает ли статус, что метод отсутствует (старая версия qBittorrent).
fn is_method_absent(status: StatusCode) -> bool {
    status == StatusCode::NOT_FOUND || status == StatusCode::METHOD_NOT_ALLOWED
}

fn guard_status(resp: reqwest::Response) -> Result<reqwest::Response> {
    match resp.status() {
        StatusCode::FORBIDDEN => Err(Error::Forbidden),
        _ => Ok(resp.error_for_status()?),
    }
}

fn normalize_base(input: &str) -> Result<Url> {
    let trimmed = input.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(Error::Url("пустой адрес qBittorrent".into()));
    }
    let with_scheme = if trimmed.contains("://") {
        trimmed.to_owned()
    } else {
        format!("http://{trimmed}")
    };
    Url::parse(&format!("{with_scheme}/")).map_err(|e| Error::Url(format!("{input}: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_base_urls() {
        assert_eq!(
            normalize_base("127.0.0.1:8080").unwrap().as_str(),
            "http://127.0.0.1:8080/"
        );
        assert_eq!(
            normalize_base("http://localhost:9090/").unwrap().as_str(),
            "http://localhost:9090/"
        );
        assert!(normalize_base("").is_err());
    }
}
