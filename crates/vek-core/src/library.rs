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
    /// Идентификаторы сторонних торрентов в папке (см. [`ExternalTorrentRecord`]).
    #[serde(default)]
    pub external_ids: Vec<String>,
    /// Когда создана (unix).
    pub created_at: i64,
}

/// Сторонний `.torrent`-файл, импортированный пользователем (не с rutracker).
/// Байты хранятся отдельным файлом в каталоге приложения.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalTorrentRecord {
    pub id: String,
    pub name: String,
    /// info-hash (hex) — для дедупликации импорта.
    pub info_hash: String,
    pub size: u64,
    pub added_at: i64,
}

/// Стандартные категории, создаваемые при первом запуске.
pub const DEFAULT_CATEGORIES: &[(&str, &str)] = &[
    ("Фильмы", "#4da3ff"),
    ("Сериалы", "#ff8a5c"),
    ("Книги", "#f5b544"),
    ("Музыка", "#3ecf8e"),
    ("Игры", "#7c5cff"),
];

/// Стандартные категории первой версии (для миграции старых установок,
/// где отмечен лишь флаг `categories_seeded`).
const LEGACY_SEEDED: &[&str] = &["Фильмы", "Книги", "Музыка", "Игры"];

/// Содержимое библиотеки.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Library {
    pub favorites: Vec<FavoriteRecord>,
    pub history: Vec<HistoryRecord>,
    pub folders: Vec<FolderRecord>,
    pub categories: Vec<CategoryRecord>,
    #[serde(default)]
    pub external_torrents: Vec<ExternalTorrentRecord>,
    /// Легаси-флаг: стандартные категории первой версии уже создавались.
    pub categories_seeded: bool,
    /// Имена стандартных категорий, которые уже создавались — чтобы новые
    /// стандартные (напр. «Сериалы») появлялись при апгрейде, а удалённые
    /// пользователем не пересоздавались.
    #[serde(default)]
    pub seeded_standard: Vec<String>,
}

impl Library {
    /// Загружает библиотеку из файла (при отсутствии/повреждении — пустая).
    pub fn load(path: &Path) -> Self {
        let mut library: Self = match fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => Self::default(),
        };
        library.drop_registered_changes();
        library
    }

    /// Чистит записи об изменении даты регистрации: у свежих раздач она
    /// приходит относительным временем («N часов»), парсилась как меняющееся
    /// число и порождала ложные обновления. Событие больше не создаётся —
    /// убираем и уже накопленные.
    fn drop_registered_changes(&mut self) {
        let is_registered = |c: &String| c.starts_with("дата регистрации");
        for favorite in &mut self.favorites {
            let had_changes = !favorite.changes.is_empty();
            favorite.changes.retain(|c| !is_registered(c));

            favorite.history.retain_mut(|event| {
                let had = !event.changes.is_empty();
                event.changes.retain(|c| !is_registered(c));
                // Уходит только событие, опустевшее из-за чистки. Изначально
                // пустые («обновление без деталей» у записей без снимка) —
                // легитимны и остаются.
                !had || !event.changes.is_empty()
            });

            // Дата регистрации затирала описание при каждой проверке, поэтому
            // опустевший `changes` при стоящей отметке ещё не значит, что
            // обновления не было: восстанавливаем описание из свежайшего
            // уцелевшего события. Отметку снимаем, только если весь след был шумом.
            if had_changes && favorite.changes.is_empty() && favorite.has_update {
                match favorite.history.first() {
                    Some(event) => favorite.changes = event.changes.clone(),
                    None => favorite.has_update = false,
                }
            }
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

    /// Создаёт недостающие стандартные категории. Возвращает `true`, если
    /// что-то добавлено. Уже созданные и удалённые пользователем не трогает
    /// (учёт по именам в `seeded_standard`).
    pub fn seed_categories(&mut self, make_id: impl Fn() -> String) -> bool {
        // Миграция со старого флага: 4 базовые считаем уже посеянными.
        if self.categories_seeded && self.seeded_standard.is_empty() {
            self.seeded_standard = LEGACY_SEEDED.iter().map(|s| (*s).to_owned()).collect();
        }

        let mut added = false;
        for (name, color) in DEFAULT_CATEGORIES {
            if self.seeded_standard.iter().any(|s| s == name) {
                continue;
            }
            self.categories.push(CategoryRecord {
                id: make_id(),
                name: (*name).to_owned(),
                color: (*color).to_owned(),
                forum_ids: Vec::new(),
            });
            self.seeded_standard.push((*name).to_owned());
            added = true;
        }
        self.categories_seeded = true;
        added
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

    /// Добавляет сторонний торрент в папку (дедуп); `false`, если папки нет.
    pub fn add_external_to_folder(&mut self, folder_id: &str, external_id: &str) -> bool {
        match self.folders.iter_mut().find(|f| f.id == folder_id) {
            Some(folder) => {
                if !folder.external_ids.iter().any(|e| e == external_id) {
                    folder.external_ids.insert(0, external_id.to_owned());
                }
                true
            }
            None => false,
        }
    }

    pub fn remove_external_from_folder(&mut self, folder_id: &str, external_id: &str) {
        if let Some(folder) = self.folders.iter_mut().find(|f| f.id == folder_id) {
            folder.external_ids.retain(|e| e != external_id);
        }
    }

    // ── Сторонние торренты ──────────────────────────────────────────────

    pub fn add_external_torrent(&mut self, record: ExternalTorrentRecord) {
        // Дедуп по info-hash: повторный импорт того же файла обновляет запись.
        self.external_torrents
            .retain(|e| e.info_hash != record.info_hash);
        self.external_torrents.insert(0, record);
    }

    /// Удаляет сторонний торрент и его вхождения во все папки.
    pub fn remove_external_torrent(&mut self, id: &str) {
        self.external_torrents.retain(|e| e.id != id);
        for folder in &mut self.folders {
            folder.external_ids.retain(|e| e != id);
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
    fn drops_registered_changes_from_history() {
        let mut lib = Library::default();

        // (1) описание затёрто датой регистрации, но в истории есть настоящее
        // изменение: отметку сохраняем и восстанавливаем описание из него
        let mut f = fav(1);
        f.has_update = true;
        f.changes = vec!["дата регистрации: 4 → 5".into()];
        f.history = vec![
            // событие только про дату регистрации — уходит целиком
            ChangeEventRecord {
                at: 30,
                changes: vec!["дата регистрации: 4 → 5".into()],
            },
            // содержательное: дата регистрации вычищается, само событие остаётся
            ChangeEventRecord {
                at: 20,
                changes: vec![
                    "дата регистрации: 5 → 6".into(),
                    "размер: 1 ГБ → 2 ГБ".into(),
                ],
            },
            // «обновление без деталей» (запись без снимка) — легитимно, остаётся
            ChangeEventRecord {
                at: 10,
                changes: Vec::new(),
            },
        ];
        lib.favorites.push(f);

        // (2) запись со старой версии: отметка есть, changes пуст изначально
        let mut migrated = fav(2);
        migrated.has_update = true;
        lib.favorites.push(migrated);

        // (3) шумовая раздача: весь след — только дата регистрации
        let mut noise = fav(3);
        noise.has_update = true;
        noise.changes = vec!["дата регистрации: 4 → 5".into()];
        noise.history = vec![ChangeEventRecord {
            at: 40,
            changes: vec!["дата регистрации: 4 → 5".into()],
        }];
        lib.favorites.push(noise);

        lib.drop_registered_changes();

        let f = &lib.favorites[0];
        assert!(f.has_update, "настоящее обновление не теряем");
        assert_eq!(
            f.changes,
            vec!["размер: 1 ГБ → 2 ГБ".to_owned()],
            "описание восстановлено из свежайшего уцелевшего события"
        );
        assert_eq!(f.history.len(), 2, "чисто-регистрационное событие удалено");
        assert_eq!(f.history[0].at, 20);
        assert_eq!(f.history[0].changes, vec!["размер: 1 ГБ → 2 ГБ".to_owned()]);
        assert_eq!(f.history[1].at, 10, "«обновление без деталей» сохранено");
        assert!(f.history[1].changes.is_empty());

        assert!(
            lib.favorites[1].has_update,
            "у записи без changes отметку обновления не снимаем"
        );

        let noise = &lib.favorites[2];
        assert!(!noise.has_update, "шумовая раздача — отметка снимается");
        assert!(noise.changes.is_empty());
        assert!(noise.history.is_empty());
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
