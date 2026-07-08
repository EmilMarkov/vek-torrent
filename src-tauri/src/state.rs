//! Состояние приложения Tauri: ядро и жизненный цикл внешнего API.

use std::sync::Arc;

use tokio::sync::Mutex;
use vek_api::{ApiServer, ApiState};
use vek_core::SharedCore;

/// Разделяемое состояние, управляемое Tauri.
pub struct AppState {
    pub core: SharedCore,
    api: Mutex<Option<ApiServer>>,
}

impl AppState {
    pub fn new(core: SharedCore) -> Self {
        Self {
            core,
            api: Mutex::new(None),
        }
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
