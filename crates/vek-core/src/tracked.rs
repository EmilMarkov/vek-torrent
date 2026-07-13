//! Версии списков файлов отслеживаемых раздач.
//!
//! Для каждой раздачи в `<app>/tracked/<topic_id>.json` хранится история
//! версий: список файлов (путь + размер) на каждый момент обнаружения
//! изменения. По этим версиям считается «патч» — набор файлов, изменившихся
//! между версией пользователя и актуальной, — и определяется версия
//! по локальной папке.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Файл раздачи в версии (относительный путь внутри раздачи + размер).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackedFile {
    pub path: String,
    pub size: u64,
}

/// Версия списка файлов раздачи.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileVersionRecord {
    /// Когда зафиксирована (unix).
    pub at: i64,
    pub files: Vec<TrackedFile>,
}

/// История версий файлов одной раздачи.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TrackedVersions {
    pub versions: Vec<FileVersionRecord>,
}

/// Максимум хранимых версий на раздачу (старые вытесняются).
const VERSIONS_CAP: usize = 30;

/// Каталог версий отслеживаемых раздач.
pub fn tracked_dir(app_dir: &Path) -> PathBuf {
    app_dir.join("tracked")
}

fn topic_file(app_dir: &Path, topic_id: u64) -> PathBuf {
    tracked_dir(app_dir).join(format!("{topic_id}.json"))
}

impl TrackedVersions {
    pub fn load(app_dir: &Path, topic_id: u64) -> Self {
        match fs::read_to_string(topic_file(app_dir, topic_id)) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, app_dir: &Path, topic_id: u64) -> Result<()> {
        let path = topic_file(app_dir, topic_id);
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string(self)?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, json)?;
        fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Удаляет историю версий раздачи (при снятии с отслеживания).
    pub fn remove(app_dir: &Path, topic_id: u64) {
        let _ = fs::remove_file(topic_file(app_dir, topic_id));
    }

    /// Добавляет версию, если список файлов отличается от последней.
    /// Возвращает `true`, если версия добавлена.
    pub fn push_if_changed(&mut self, at: i64, files: Vec<TrackedFile>) -> bool {
        if self.versions.last().is_some_and(|last| last.files == files) {
            return false;
        }
        self.versions.push(FileVersionRecord { at, files });
        if self.versions.len() > VERSIONS_CAP {
            let excess = self.versions.len() - VERSIONS_CAP;
            self.versions.drain(0..excess);
        }
        true
    }
}

/// Изменение файла между версиями.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileChangeKind {
    Added,
    Changed,
    Removed,
}

/// Файл в диффе версий.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: String,
    /// Размер в новой версии (для удалённых — в старой).
    pub size: u64,
    pub kind: FileChangeKind,
}

/// Дифф списков файлов: что появилось/изменилось/исчезло в `new` против `old`.
pub fn diff_files(old: &[TrackedFile], new: &[TrackedFile]) -> Vec<FileChange> {
    let old_map: std::collections::HashMap<&str, u64> =
        old.iter().map(|f| (f.path.as_str(), f.size)).collect();
    let new_set: std::collections::HashSet<&str> = new.iter().map(|f| f.path.as_str()).collect();

    let mut changes = Vec::new();
    for file in new {
        match old_map.get(file.path.as_str()) {
            None => changes.push(FileChange {
                path: file.path.clone(),
                size: file.size,
                kind: FileChangeKind::Added,
            }),
            Some(&old_size) if old_size != file.size => changes.push(FileChange {
                path: file.path.clone(),
                size: file.size,
                kind: FileChangeKind::Changed,
            }),
            Some(_) => {}
        }
    }
    for file in old {
        if !new_set.contains(file.path.as_str()) {
            changes.push(FileChange {
                path: file.path.clone(),
                size: file.size,
                kind: FileChangeKind::Removed,
            });
        }
    }
    changes
}

/// Короткое описание файлового диффа для истории изменений.
pub fn describe_file_diff(changes: &[FileChange]) -> Option<String> {
    if changes.is_empty() {
        return None;
    }
    let added = changes
        .iter()
        .filter(|c| c.kind == FileChangeKind::Added)
        .count();
    let changed = changes
        .iter()
        .filter(|c| c.kind == FileChangeKind::Changed)
        .count();
    let removed = changes
        .iter()
        .filter(|c| c.kind == FileChangeKind::Removed)
        .count();
    let mut parts = Vec::new();
    if added > 0 {
        parts.push(format!("новых: {added}"));
    }
    if changed > 0 {
        parts.push(format!("изменено: {changed}"));
    }
    if removed > 0 {
        parts.push(format!("удалено: {removed}"));
    }
    Some(format!("файлы — {}", parts.join(", ")))
}

/// Сканирует локальную папку: относительные пути и размеры файлов.
///
/// Сравнение с версиями идёт по хвосту пути: содержимое раздачи может лежать
/// как в выбранной папке напрямую, так и во вложенной папке раздачи.
pub fn scan_dir(dir: &Path) -> Result<Vec<TrackedFile>> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if meta.is_dir() {
                stack.push(path);
            } else if meta.is_file()
                && let Ok(relative) = path.strip_prefix(dir)
            {
                files.push(TrackedFile {
                    path: relative.to_string_lossy().replace('\\', "/"),
                    size: meta.len(),
                });
            }
        }
        // Защита от гигантских деревьев: сравнение всё равно приблизительное.
        if files.len() > 200_000 {
            break;
        }
    }
    Ok(files)
}

/// Насколько локальные файлы совпадают с версией: количество совпавших
/// (по хвосту пути и размеру) файлов.
pub fn match_score(local: &[TrackedFile], version: &[TrackedFile]) -> usize {
    // Индекс версии по размеру: сравнение путей только среди кандидатов
    // того же размера (раздачи содержат тысячи файлов).
    let mut by_size: std::collections::HashMap<u64, Vec<&str>> = std::collections::HashMap::new();
    for file in version {
        by_size.entry(file.size).or_default().push(&file.path);
    }

    let mut matched = 0;
    for local_file in local {
        let Some(candidates) = by_size.get(&local_file.size) else {
            continue;
        };
        // Файл раздачи может лежать глубже выбранной папки (или наоборот):
        // засчитываем совпадение, если один путь — суффикс другого.
        let found = candidates.iter().any(|v| {
            local_file.path == **v
                || local_file.path.ends_with(&format!("/{v}"))
                || v.ends_with(&format!("/{}", local_file.path))
        });
        if found {
            matched += 1;
        }
    }
    matched
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(path: &str, size: u64) -> TrackedFile {
        TrackedFile {
            path: path.into(),
            size,
        }
    }

    #[test]
    fn diff_detects_added_changed_removed() {
        let old = vec![f("a.bin", 10), f("b.bin", 20), f("c.bin", 30)];
        let new = vec![f("a.bin", 10), f("b.bin", 25), f("d.bin", 40)];
        let diff = diff_files(&old, &new);
        assert_eq!(diff.len(), 3);
        assert!(
            diff.iter()
                .any(|c| c.path == "b.bin" && c.kind == FileChangeKind::Changed)
        );
        assert!(
            diff.iter()
                .any(|c| c.path == "d.bin" && c.kind == FileChangeKind::Added)
        );
        assert!(
            diff.iter()
                .any(|c| c.path == "c.bin" && c.kind == FileChangeKind::Removed)
        );
    }

    #[test]
    fn describe_diff_is_compact() {
        let old = vec![f("a", 1)];
        let new = vec![f("a", 2), f("b", 3)];
        let text = describe_file_diff(&diff_files(&old, &new)).unwrap();
        assert!(text.contains("новых: 1"));
        assert!(text.contains("изменено: 1"));
    }

    #[test]
    fn push_if_changed_dedupes() {
        let mut v = TrackedVersions::default();
        assert!(v.push_if_changed(1, vec![f("a", 1)]));
        assert!(!v.push_if_changed(2, vec![f("a", 1)]));
        assert!(v.push_if_changed(3, vec![f("a", 2)]));
        assert_eq!(v.versions.len(), 2);
    }

    #[test]
    fn match_score_handles_nested_paths() {
        let version = vec![f("Game/data.pak", 100), f("Game/bin/game.exe", 50)];
        // Пользователь выбрал папку самой игры: пути без префикса раздачи.
        let local = vec![
            f("data.pak", 100),
            f("bin/game.exe", 50),
            f("readme.txt", 5),
        ];
        assert_eq!(match_score(&local, &version), 2);
    }

    #[test]
    fn roundtrip_storage() {
        let dir = tempfile::tempdir().unwrap();
        let mut v = TrackedVersions::default();
        v.push_if_changed(1, vec![f("a", 1)]);
        v.save(dir.path(), 42).unwrap();
        let loaded = TrackedVersions::load(dir.path(), 42);
        assert_eq!(loaded.versions.len(), 1);
        TrackedVersions::remove(dir.path(), 42);
        assert_eq!(TrackedVersions::load(dir.path(), 42).versions.len(), 0);
    }
}
