//! Персистентная «библиотека»: избранные раздачи и история скачиваний.
//!
//! Хранится единым JSON-файлом в каталоге приложения. Для избранного
//! запоминается «сигнатура» раздачи (дата регистрации + размер), по изменению
//! которой определяется обновление раздачи на трекере.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Запись избранной раздачи (персистится).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FavoriteRecord {
    pub topic_id: u64,
    pub title: String,
    /// Когда добавлено в избранное (unix).
    pub added_at: i64,
    /// Когда последний раз проверялось обновление (unix).
    pub last_checked: i64,
    /// Сигнатура состояния раздачи для детекта обновлений.
    #[serde(default)]
    pub signature: String,
    /// Обнаружено обновление с последнего просмотра.
    #[serde(default)]
    pub has_update: bool,
}

/// Запись истории скачивания (персистится).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryRecord {
    pub topic_id: u64,
    pub title: String,
    /// info-hash добавленного торрента.
    #[serde(default)]
    pub hash: String,
    /// Когда добавлено в загрузки (unix).
    pub added_at: i64,
}

/// Содержимое библиотеки.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Library {
    pub favorites: Vec<FavoriteRecord>,
    pub history: Vec<HistoryRecord>,
}

impl Library {
    /// Загружает библиотеку из файла (при отсутствии/повреждении — пустая).
    pub fn load(path: &Path) -> Self {
        match fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Атомарно сохраняет библиотеку.
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

    pub fn is_favorite(&self, topic_id: u64) -> bool {
        self.favorites.iter().any(|f| f.topic_id == topic_id)
    }

    /// Добавляет/обновляет избранное (новые — в начало списка).
    pub fn add_favorite(&mut self, record: FavoriteRecord) {
        self.favorites.retain(|f| f.topic_id != record.topic_id);
        self.favorites.insert(0, record);
    }

    pub fn remove_favorite(&mut self, topic_id: u64) {
        self.favorites.retain(|f| f.topic_id != topic_id);
    }

    /// Сбрасывает отметку обновления у избранного.
    pub fn clear_update(&mut self, topic_id: u64) {
        if let Some(f) = self.favorites.iter_mut().find(|f| f.topic_id == topic_id) {
            f.has_update = false;
        }
    }

    /// Записывает историю (дедуп по теме, новые — в начало, максимум 500).
    pub fn add_history(&mut self, record: HistoryRecord) {
        self.history.retain(|h| h.topic_id != record.topic_id);
        self.history.insert(0, record);
        self.history.truncate(500);
    }

    pub fn remove_history(&mut self, topic_id: u64) {
        self.history.retain(|h| h.topic_id != topic_id);
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
    }
}

/// Путь к файлу библиотеки.
pub fn library_file(app_dir: &Path) -> PathBuf {
    app_dir.join("library.json")
}

/// Текущее unix-время (секунды).
pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fav(id: u64) -> FavoriteRecord {
        FavoriteRecord {
            topic_id: id,
            title: format!("t{id}"),
            added_at: 1,
            last_checked: 1,
            signature: "sig".into(),
            has_update: false,
        }
    }

    #[test]
    fn favorites_dedup_and_order() {
        let mut lib = Library::default();
        lib.add_favorite(fav(1));
        lib.add_favorite(fav(2));
        lib.add_favorite(fav(1)); // повтор — поднимается наверх
        assert_eq!(lib.favorites.len(), 2);
        assert_eq!(lib.favorites[0].topic_id, 1);
        assert!(lib.is_favorite(2));
        lib.remove_favorite(1);
        assert!(!lib.is_favorite(1));
    }

    #[test]
    fn clear_update_resets_flag() {
        let mut lib = Library::default();
        let mut f = fav(5);
        f.has_update = true;
        lib.add_favorite(f);
        lib.clear_update(5);
        assert!(!lib.favorites[0].has_update);
    }

    #[test]
    fn history_dedup_and_cap() {
        let mut lib = Library::default();
        for i in 0..600u64 {
            lib.add_history(HistoryRecord {
                topic_id: i,
                title: "x".into(),
                hash: String::new(),
                added_at: 0,
            });
        }
        assert_eq!(lib.history.len(), 500);
        assert_eq!(lib.history[0].topic_id, 599);
    }

    #[test]
    fn roundtrip_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = library_file(dir.path());
        let mut lib = Library::default();
        lib.add_favorite(fav(1));
        lib.save(&path).unwrap();
        let loaded = Library::load(&path);
        assert!(loaded.is_favorite(1));
    }
}
