//! Состояние приложения Tauri: ядро и жизненный цикл внешнего API.

use std::sync::{Arc, Mutex as StdMutex};

use tokio::sync::Mutex;
use vek_api::{ApiServer, ApiState};
use vek_core::SharedCore;

/// Разделяемое состояние, управляемое Tauri.
pub struct AppState {
    pub core: SharedCore,
    api: Mutex<Option<ApiServer>>,
    /// Тема из внутренней ссылки при холодном старте (забирается фронтендом).
    pending_deeplink: StdMutex<Option<u64>>,
}

impl AppState {
    pub fn new(core: SharedCore) -> Self {
        Self {
            core,
            api: Mutex::new(None),
            pending_deeplink: StdMutex::new(None),
        }
    }

    /// Запоминает тему из ссылки, открывшей приложение.
    pub fn set_pending_deeplink(&self, topic_id: u64) {
        *self.pending_deeplink.lock().expect("deeplink lock") = Some(topic_id);
    }

    /// Забирает отложенную тему (единожды).
    pub fn take_pending_deeplink(&self) -> Option<u64> {
        self.pending_deeplink.lock().expect("deeplink lock").take()
    }

    /// Останавливает текущий API-сервер (если запущен) и, если он включён в
    /// конфигурации, запускает заново на актуальных host/port/token.
    pub async fn restart_api(&self) -> Result<(), String> {
        if let Some(server) = self.api.lock().await.take() {
            server.stop().await;
        }
        self.core.set_api_running(false);

        let config = self.core.config();
        if !config.api.enabled {
            return Ok(());
        }

        let token: Arc<str> = Arc::from(config.api.token.as_str());
        let api_state = ApiState {
            core: Arc::clone(&self.core),
            token,
        };

        let server = vek_api::serve(api_state, &config.api.host, config.api.port)
            .await
            .map_err(|e| format!("не удалось запустить API: {e}"))?;

        tracing::info!("внешний API запущен на {}", server.addr);
        self.core.set_api_running(true);
        *self.api.lock().await = Some(server);
        Ok(())
    }

    /// Останавливает API-сервер при завершении приложения.
    pub async fn stop_api(&self) {
        if let Some(server) = self.api.lock().await.take() {
            server.stop().await;
        }
        self.core.set_api_running(false);
    }
}
