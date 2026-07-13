//! Персистентная «библиотека»: избранные раздачи, история скачиваний,
//! пользовательские папки и категории.
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
    /// Снимок состояния раздачи на момент последней проверки — по нему
    /// строится описание того, что именно изменилось.
    #[serde(default)]
    pub snapshot: FavoriteSnapshot,
    /// Человекочитаемое описание изменений последнего обновления.
    #[serde(default)]
    pub changes: Vec<String>,
    /// История обнаруженных обновлений (новые — в начало).
    #[serde(default)]
    pub history: Vec<ChangeEventRecord>,
}

/// Отслеживаемые поля раздачи для детального описания обновлений.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FavoriteSnapshot {
    pub title: String,
    pub size_bytes: Option<u64>,
    /// Дата регистрации торрента (текстом, как на странице).
    pub registered: Option<String>,
    /// info-hash из magnet-ссылки: меняется при перезаливке торрента.
    pub info_hash: Option<String>,
    /// Хэш текста описания (детект изменений описания).
    pub description_hash: Option<String>,
}

/// Событие истории изменений отслеживаемой раздачи.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeEventRecord {
    /// Когда обнаружено (unix).
    pub at: i64,
    /// Человекочитаемые описания изменений.
    pub changes: Vec<String>,
}

/// Максимум событий истории на раздачу.
pub const HISTORY_CAP: usize = 200;

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

/// Пользовательская категория: метка для папок и набор разделов rutracker
/// (используется фильтрами поиска).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryRecord {
    pub id: String,
    pub name: String,
    /// Цвет метки (hex, например `#7c5cff`).
    pub color: String,
    /// Разделы rutracker, объединяемые этой категорией (id форумов).
    #[serde(default)]
    pub forum_ids: Vec<i64>,
}

/// Раздача внутри папки.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderTopicRecord {
    pub topic_id: u64,
    pub title: String,
    /// Когда добавлена в папку (unix).
    pub added_at: i64,
}

/// Пользовательская папка с раздачами.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderRecord {
    pub id: String,
    pub name: String,
    /// Идентификатор категории (см. [`CategoryRecord`]).
    #[serde(default)]
    pub category_id: Option<String>,
    #[serde(default)]
    pub topics: Vec<FolderTopicRecord>,
    /// Когда создана (unix).
    pub created_at: i64,
}

/// Стандартные категории, создаваемые при первом запуске.
pub const DEFAULT_CATEGORIES: &[(&str, &str)] = &[
    ("Фильмы", "#4da3ff"),
    ("Книги", "#f5b544"),
    ("Музыка", "#3ecf8e"),
    ("Игры", "#7c5cff"),
];

/// Содержимое библиотеки.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Library {
    pub favorites: Vec<FavoriteRecord>,
    pub history: Vec<HistoryRecord>,
    pub folders: Vec<FolderRecord>,
    pub categories: Vec<CategoryRecord>,
    /// Стандартные категории уже создавались (защита от повторного посева
    /// после того, как пользователь их удалил).
    pub categories_seeded: bool,
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

    /// Сбрасывает отметку обновления у избранного (и описание изменений).
    pub fn clear_update(&mut self, topic_id: u64) {
        if let Some(f) = self.favorites.iter_mut().find(|f| f.topic_id == topic_id) {
            f.has_update = false;
            f.changes.clear();
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

    // ── Категории ───────────────────────────────────────────────────────

    /// Создаёт стандартные категории при первом обращении (один раз).
    pub fn seed_categories(&mut self, make_id: impl Fn() -> String) -> bool {
        if self.categories_seeded {
            return false;
        }
        for (name, color) in DEFAULT_CATEGORIES {
            self.categories.push(CategoryRecord {
                id: make_id(),
                name: (*name).to_owned(),
                color: (*color).to_owned(),
                forum_ids: Vec::new(),
            });
        }
        self.categories_seeded = true;
        true
    }

    pub fn add_category(&mut self, record: CategoryRecord) {
        self.categories.push(record);
    }

    /// Обновляет категорию; `false`, если категория не найдена.
    pub fn update_category(
        &mut self,
        id: &str,
        name: String,
        color: String,
        forum_ids: Vec<i64>,
    ) -> bool {
        match self.categories.iter_mut().find(|c| c.id == id) {
            Some(category) => {
                category.name = name;
                category.color = color;
                category.forum_ids = forum_ids;
                true
            }
            None => false,
        }
    }

    /// Удаляет категорию и снимает её со всех папок.
    pub fn remove_category(&mut self, id: &str) {
        self.categories.retain(|c| c.id != id);
        for folder in &mut self.folders {
            if folder.category_id.as_deref() == Some(id) {
                folder.category_id = None;
            }
        }
    }

    // ── Папки ───────────────────────────────────────────────────────────

    pub fn add_folder(&mut self, record: FolderRecord) {
        self.folders.push(record);
    }

    /// Обновляет имя/категорию папки; `false`, если папка не найдена.
    pub fn update_folder(&mut self, id: &str, name: String, category_id: Option<String>) -> bool {
        match self.folders.iter_mut().find(|f| f.id == id) {
            Some(folder) => {
                folder.name = name;
                folder.category_id = category_id;
                true
            }
            None => false,
        }
    }

    pub fn remove_folder(&mut self, id: &str) {
        self.folders.retain(|f| f.id != id);
    }

    /// Добавляет раздачу в папку (дедуп по теме); `false`, если папки нет.
    pub fn add_topic_to_folder(&mut self, folder_id: &str, topic: FolderTopicRecord) -> bool {
        match self.folders.iter_mut().find(|f| f.id == folder_id) {
            Some(folder) => {
                folder.topics.retain(|t| t.topic_id != topic.topic_id);
                folder.topics.insert(0, topic);
                true
            }
            None => false,
        }
    }

    pub fn remove_topic_from_folder(&mut self, folder_id: &str, topic_id: u64) {
        if let Some(folder) = self.folders.iter_mut().find(|f| f.id == folder_id) {
            folder.topics.retain(|t| t.topic_id != topic_id);
        }
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
            snapshot: FavoriteSnapshot::default(),
            changes: Vec::new(),
            history: Vec::new(),
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
