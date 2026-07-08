//! Sidecar-менеджер qBittorrent: поиск бинарника, генерация изолированного
//! профиля с включённым Web UI, запуск фонового процесса, health-check и
//! корректное завершение.

use std::{
    net::TcpListener,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use qbit::{Client, ClientConfig};
use tokio::{process::Command, time::sleep};

use crate::error::{Error, Result};

/// Кандидаты имени/пути исполняемого файла qBittorrent по платформам.
fn binary_candidates() -> Vec<PathBuf> {
    let mut list: Vec<PathBuf> = Vec::new();

    // Безголовая сборка предпочтительнее — не поднимает GUI.
    for name in ["qbittorrent-nox", "qbittorrent"] {
        if let Ok(path) = which::which(name) {
            list.push(path);
        }
    }

    #[cfg(target_os = "windows")]
    {
        for base in ["C:/Program Files/qBittorrent", "C:/Program Files (x86)/qBittorrent"] {
            list.push(PathBuf::from(format!("{base}/qbittorrent.exe")));
        }
    }

    #[cfg(target_os = "macos")]
    {
        list.push(PathBuf::from(
            "/Applications/qBittorrent.app/Contents/MacOS/qbittorrent",
        ));
    }

    #[cfg(target_os = "linux")]
    {
        for path in [
            "/usr/bin/qbittorrent-nox",
            "/usr/local/bin/qbittorrent-nox",
            "/usr/bin/qbittorrent",
        ] {
            list.push(PathBuf::from(path));
        }
    }

    list
}

/// Находит исполняемый файл qBittorrent: сначала явный путь, затем кандидаты.
pub fn resolve_binary(explicit: &str) -> Result<PathBuf> {
    let explicit = explicit.trim();
    if !explicit.is_empty() {
        let path = PathBuf::from(explicit);
        if path.is_file() {
            return Ok(path);
        }
        // Явно заданное имя ищем в PATH.
        if let Ok(found) = which::which(explicit) {
            return Ok(found);
        }
        return Err(Error::QbitBinaryNotFound);
    }

    binary_candidates()
        .into_iter()
        .find(|p| p.is_file())
        .ok_or(Error::QbitBinaryNotFound)
}

/// Выбирает порт: заданный (если валиден) или свободный из эфемерного диапазона.
pub fn choose_port(preferred: u16) -> Result<u16> {
    if preferred != 0 && port_is_free(preferred) {
        return Ok(preferred);
    }
    // Просим ОС дать свободный порт.
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let port = listener.local_addr()?.port();
    Ok(port)
}

fn port_is_free(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

/// Генерирует `qBittorrent.conf` в профиле: Web UI на localhost без авторизации.
///
/// Возвращает путь к файлу конфигурации.
pub fn write_profile_config(profile_dir: &Path, port: u16, save_path: &str) -> Result<PathBuf> {
    let config_dir = profile_dir.join("qBittorrent").join("config");
    std::fs::create_dir_all(&config_dir)?;

    // Секцию [BitTorrent] пишем только при заданном каталоге сохранения,
    // чтобы не оставлять висячий заголовок.
    let bittorrent_section = if save_path.trim().is_empty() {
        String::new()
    } else {
        format!("\n[BitTorrent]\nSession\\DefaultSavePath={save_path}\n")
    };

    // INI-формат qBittorrent. LocalHostAuth=false отключает пароль для 127.0.0.1;
    // проверки CSRF/Host снимаем ради программного доступа только с localhost.
    let contents = format!(
        "[LegalNotice]\n\
         Accepted=true\n\
         \n\
         [Preferences]\n\
         WebUI\\Enabled=true\n\
         WebUI\\Address=127.0.0.1\n\
         WebUI\\Port={port}\n\
         WebUI\\LocalHostAuth=false\n\
         WebUI\\CSRFProtection=false\n\
         WebUI\\HostHeaderValidation=false\n\
         WebUI\\ClickjackingProtection=false\n\
         {bittorrent_section}"
    );

    let config_file = config_dir.join("qBittorrent.conf");
    std::fs::write(&config_file, contents)?;
    Ok(config_file)
}

/// Параметры запуска sidecar.
#[derive(Debug, Clone)]
pub struct SidecarOptions {
    pub binary_path: String,
    pub profile_dir: PathBuf,
    pub preferred_port: u16,
    pub default_save_path: String,
}

/// Запущенный процесс qBittorrent и данные для подключения.
#[derive(Debug)]
pub struct SidecarProcess {
    child: tokio::process::Child,
    pub port: u16,
    pub binary: PathBuf,
}

impl SidecarProcess {
    /// Клиент qBittorrent, настроенный на этот sidecar.
    pub fn client(&self) -> Result<Client> {
        Client::new(ClientConfig {
            base_url: format!("http://127.0.0.1:{}", self.port),
            timeout: Duration::from_secs(15),
        })
        .map_err(Error::from)
    }

    /// Жив ли ещё дочерний процесс (не завершился ли аварийно).
    pub fn is_running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Корректно завершает qBittorrent: сначала через Web API, затем принудительно.
    pub async fn shutdown(mut self) {
        if let Ok(client) = self.client() {
            let _ = client.shutdown().await;
        }
        // Ждём немного добровольного завершения.
        for _ in 0..20 {
            if let Ok(Some(_)) = self.child.try_wait() {
                return;
            }
            sleep(Duration::from_millis(150)).await;
        }
        let _ = self.child.start_kill();
        let _ = self.child.wait().await;
    }
}

/// Запускает sidecar и дожидается готовности Web API.
pub async fn spawn(options: &SidecarOptions) -> Result<SidecarProcess> {
    let binary = resolve_binary(&options.binary_path)?;
    let port = choose_port(options.preferred_port)?;

    write_profile_config(&options.profile_dir, port, &options.default_save_path)?;

    let mut command = Command::new(&binary);
    command
        .arg(format!("--profile={}", options.profile_dir.display()))
        .arg(format!("--webui-port={port}"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null());

    // GUI-сборка qBittorrent под Linux: не создавать окно, если возможно.
    #[cfg(target_os = "linux")]
    {
        command.env("QT_QPA_PLATFORM", "offscreen");
    }

    #[cfg(windows)]
    {
        // tokio::process::Command предоставляет creation_flags как inherent-метод.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let child = command.spawn().map_err(|e| {
        Error::QbitUnavailable(format!("не удалось запустить {}: {e}", binary.display()))
    })?;

    let mut process = SidecarProcess {
        child,
        port,
        binary,
    };

    wait_until_ready(&mut process).await?;
    Ok(process)
}

/// Ожидает готовности Web API sidecar с таймаутом.
async fn wait_until_ready(process: &mut SidecarProcess) -> Result<()> {
    let client = process.client()?;
    for _ in 0..60 {
        if let Ok(Some(status)) = process.child.try_wait() {
            return Err(Error::QbitUnavailable(format!(
                "процесс qBittorrent завершился при запуске (код {status})"
            )));
        }
        if client.is_alive().await {
            return Ok(());
        }
        sleep(Duration::from_millis(250)).await;
    }
    Err(Error::QbitUnavailable(
        "Web API qBittorrent не поднялся за отведённое время".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_binary_rejects_missing_explicit_path() {
        let err = resolve_binary("/nonexistent/path/to/qbittorrent-xyz");
        assert!(matches!(err, Err(Error::QbitBinaryNotFound)));
    }

    #[test]
    fn choose_port_returns_free_port() {
        let port = choose_port(0).unwrap();
        assert!(port > 0);
        assert!(port_is_free(port));
    }

    #[test]
    fn choose_port_falls_back_when_busy() {
        // Занимаем порт и проверяем, что менеджер выберет другой.
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let busy = listener.local_addr().unwrap().port();
        let chosen = choose_port(busy).unwrap();
        assert_ne!(chosen, busy);
    }

    #[test]
    fn writes_profile_config_with_port_and_auth_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_profile_config(dir.path(), 12345, "/downloads").unwrap();
        let text = std::fs::read_to_string(&path).unwrap();

        assert!(text.contains("WebUI\\Port=12345"));
        assert!(text.contains("WebUI\\LocalHostAuth=false"));
        assert!(text.contains("Accepted=true"));
        assert!(text.contains("Session\\DefaultSavePath=/downloads"));
        assert!(path.ends_with("qBittorrent/config/qBittorrent.conf"));
    }

    #[test]
    fn writes_profile_config_without_save_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_profile_config(dir.path(), 8080, "  ").unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains("DefaultSavePath"));
    }
}
