use crate::config::{DeleteNoteNodeResult, NoteDocument, NoteFolder, NotesSnapshot};
use crate::error::{AppError, AppResult};
use redb::ReadableTable;
use std::collections::{HashMap, HashSet};

use super::Storage;
use super::tables::*;
use super::util::*;

const DEFAULT_NOTE_TITLE: &str = "新建笔记";
const DEFAULT_FOLDER_NAME: &str = "新建文件夹";
const MAX_NOTE_NAME_CHARS: usize = 120;

impl Storage {
    pub fn list_note_folders(&self) -> AppResult<Vec<NoteFolder>> {
        let mut folders = self.list_json_by_prefix(NOTE_FOLDERS_TABLE, NOTE_FOLDER_PREFIX)?;
        sort_note_folders(&mut folders);
        Ok(folders)
    }

    pub fn list_notes(&self) -> AppResult<Vec<NoteDocument>> {
        let mut notes = self.list_json_by_prefix(NOTES_TABLE, NOTE_DOCUMENT_PREFIX)?;
        sort_notes(&mut notes);
        Ok(notes)
    }

    pub fn get_note(&self, note_id: &str) -> AppResult<Option<NoteDocument>> {
        self.read_json(NOTES_TABLE, &entity_key(NOTE_DOCUMENT_PREFIX, note_id))
    }

    pub fn create_note_folder(
        &self,
        parent_id: Option<String>,
        name: Option<String>,
    ) -> AppResult<NoteFolder> {
        let txn = self.db.begin_write().map_err(storage_error)?;
        let mut folders = read_note_folders_in_txn(&txn)?;
        let notes = read_notes_in_txn(&txn)?;
        validate_parent_exists(&folders, parent_id.as_deref())?;
        let sibling_names = sibling_names(&folders, &notes, parent_id.as_deref(), None);
        let name = normalize_or_unique_name(name, DEFAULT_FOLDER_NAME, &sibling_names)?;
        let sort_order = next_sort_order_for_parent(&folders, &notes, parent_id.as_deref());
        let now = current_time_ms();
        let folder = NoteFolder {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id,
            name,
            sort_order,
            created_at_ms: now,
            updated_at_ms: now,
        };
        write_note_folder_in_txn(&txn, &folder)?;
        txn.commit().map_err(storage_error)?;
        folders.push(folder.clone());
        Ok(folder)
    }

    pub fn create_note(
        &self,
        parent_id: Option<String>,
        title: Option<String>,
        markdown: Option<String>,
    ) -> AppResult<NoteDocument> {
        let txn = self.db.begin_write().map_err(storage_error)?;
        let folders = read_note_folders_in_txn(&txn)?;
        let mut notes = read_notes_in_txn(&txn)?;
        validate_parent_exists(&folders, parent_id.as_deref())?;
        let sibling_names = sibling_names(&folders, &notes, parent_id.as_deref(), None);
        let title = normalize_or_unique_name(title, DEFAULT_NOTE_TITLE, &sibling_names)?;
        let sort_order = next_sort_order_for_parent(&folders, &notes, parent_id.as_deref());
        let now = current_time_ms();
        let note = NoteDocument {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id,
            title,
            markdown: markdown.unwrap_or_default(),
            sort_order,
            revision: 1,
            created_at_ms: now,
            updated_at_ms: now,
        };
        write_note_in_txn(&txn, &note)?;
        txn.commit().map_err(storage_error)?;
        notes.push(note.clone());
        Ok(note)
    }

    pub fn update_note(
        &self,
        note_id: &str,
        title: String,
        markdown: String,
        expected_revision: u64,
        force: bool,
    ) -> AppResult<NoteDocument> {
        let txn = self.db.begin_write().map_err(storage_error)?;
        let folders = read_note_folders_in_txn(&txn)?;
        let notes = read_notes_in_txn(&txn)?;
        let Some(mut note) = notes.iter().find(|item| item.id == note_id).cloned() else {
            return Err(AppError::Config(format!("Note '{note_id}' does not exist")));
        };
        if !force && note.revision != expected_revision {
            return Err(AppError::Config(format!(
                "Revision conflict: expected {}, found {}",
                expected_revision, note.revision
            )));
        }

        let title = normalize_note_name(&title)?;
        validate_unique_sibling_name(
            &folders,
            &notes,
            note.parent_id.as_deref(),
            &title,
            Some(("note", note_id)),
        )?;

        let changed = note.title != title || note.markdown != markdown;
        if changed {
            note.title = title;
            note.markdown = markdown;
            note.revision = note.revision.saturating_add(1);
            note.updated_at_ms = current_time_ms();
            write_note_in_txn(&txn, &note)?;
        }
        txn.commit().map_err(storage_error)?;
        Ok(note)
    }

    pub fn rename_note_node(&self, node_kind: &str, node_id: &str, name: String) -> AppResult<()> {
        let txn = self.db.begin_write().map_err(storage_error)?;
        let folders = read_note_folders_in_txn(&txn)?;
        let notes = read_notes_in_txn(&txn)?;
        let name = normalize_note_name(&name)?;
        match node_kind {
            "folder" => {
                let Some(mut folder) = folders.iter().find(|item| item.id == node_id).cloned()
                else {
                    return Err(AppError::Config(format!(
                        "Folder '{node_id}' does not exist"
                    )));
                };
                validate_unique_sibling_name(
                    &folders,
                    &notes,
                    folder.parent_id.as_deref(),
                    &name,
                    Some(("folder", node_id)),
                )?;
                folder.name = name;
                folder.updated_at_ms = current_time_ms();
                write_note_folder_in_txn(&txn, &folder)?;
            }
            "note" => {
                let Some(mut note) = notes.iter().find(|item| item.id == node_id).cloned() else {
                    return Err(AppError::Config(format!("Note '{node_id}' does not exist")));
                };
                validate_unique_sibling_name(
                    &folders,
                    &notes,
                    note.parent_id.as_deref(),
                    &name,
                    Some(("note", node_id)),
                )?;
                if note.title != name {
                    note.title = name;
                    note.revision = note.revision.saturating_add(1);
                    note.updated_at_ms = current_time_ms();
                    write_note_in_txn(&txn, &note)?;
                }
            }
            _ => return Err(AppError::Config("Invalid note node kind".to_string())),
        }
        txn.commit().map_err(storage_error)?;
        Ok(())
    }

    pub fn move_note_node(
        &self,
        node_kind: &str,
        node_id: &str,
        parent_id: Option<String>,
        sort_order: i64,
    ) -> AppResult<()> {
        let txn = self.db.begin_write().map_err(storage_error)?;
        let folders = read_note_folders_in_txn(&txn)?;
        let notes = read_notes_in_txn(&txn)?;
        validate_parent_exists(&folders, parent_id.as_deref())?;

        match node_kind {
            "folder" => {
                let Some(mut folder) = folders.iter().find(|item| item.id == node_id).cloned()
                else {
                    return Err(AppError::Config(format!(
                        "Folder '{node_id}' does not exist"
                    )));
                };
                if parent_id.as_deref() == Some(node_id) {
                    return Err(AppError::Config(
                        "A folder cannot be moved into itself".to_string(),
                    ));
                }
                if let Some(parent_id) = parent_id.as_deref() {
                    validate_not_descendant_folder(&folders, node_id, parent_id)?;
                }
                validate_unique_sibling_name(
                    &folders,
                    &notes,
                    parent_id.as_deref(),
                    &folder.name,
                    Some(("folder", node_id)),
                )?;
                folder.parent_id = parent_id;
                folder.sort_order = sort_order;
                folder.updated_at_ms = current_time_ms();
                write_note_folder_in_txn(&txn, &folder)?;
            }
            "note" => {
                let Some(mut note) = notes.iter().find(|item| item.id == node_id).cloned() else {
                    return Err(AppError::Config(format!("Note '{node_id}' does not exist")));
                };
                validate_unique_sibling_name(
                    &folders,
                    &notes,
                    parent_id.as_deref(),
                    &note.title,
                    Some(("note", node_id)),
                )?;
                note.parent_id = parent_id;
                note.sort_order = sort_order;
                note.revision = note.revision.saturating_add(1);
                note.updated_at_ms = current_time_ms();
                write_note_in_txn(&txn, &note)?;
            }
            _ => return Err(AppError::Config("Invalid note node kind".to_string())),
        }
        txn.commit().map_err(storage_error)?;
        Ok(())
    }

    pub fn delete_note_node(
        &self,
        node_kind: &str,
        node_id: &str,
    ) -> AppResult<DeleteNoteNodeResult> {
        let txn = self.db.begin_write().map_err(storage_error)?;
        let folders = read_note_folders_in_txn(&txn)?;
        let notes = read_notes_in_txn(&txn)?;
        let mut folder_ids = HashSet::new();
        let mut note_ids = HashSet::new();

        match node_kind {
            "folder" => {
                if !folders.iter().any(|folder| folder.id == node_id) {
                    return Err(AppError::Config(format!(
                        "Folder '{node_id}' does not exist"
                    )));
                }
                collect_descendant_folder_ids(&folders, node_id, &mut folder_ids);
                folder_ids.insert(node_id.to_string());
                for note in &notes {
                    if note
                        .parent_id
                        .as_ref()
                        .is_some_and(|parent| folder_ids.contains(parent))
                    {
                        note_ids.insert(note.id.clone());
                    }
                }
            }
            "note" => {
                if !notes.iter().any(|note| note.id == node_id) {
                    return Err(AppError::Config(format!("Note '{node_id}' does not exist")));
                }
                note_ids.insert(node_id.to_string());
            }
            _ => return Err(AppError::Config("Invalid note node kind".to_string())),
        }

        {
            let mut folder_table = txn.open_table(NOTE_FOLDERS_TABLE).map_err(storage_error)?;
            for id in &folder_ids {
                folder_table
                    .remove(entity_key(NOTE_FOLDER_PREFIX, id).as_str())
                    .map_err(storage_error)?;
            }
        }
        {
            let mut note_table = txn.open_table(NOTES_TABLE).map_err(storage_error)?;
            for id in &note_ids {
                note_table
                    .remove(entity_key(NOTE_DOCUMENT_PREFIX, id).as_str())
                    .map_err(storage_error)?;
            }
        }
        txn.commit().map_err(storage_error)?;
        let folder_count = folder_ids.len();
        let note_count = note_ids.len();
        let mut ids = folder_ids.into_iter().chain(note_ids).collect::<Vec<_>>();
        ids.sort();
        Ok(DeleteNoteNodeResult {
            folder_count,
            note_count,
            ids,
        })
    }

    pub fn load_notes_snapshot(&self) -> AppResult<NotesSnapshot> {
        Ok(NotesSnapshot {
            folders: self.list_note_folders()?,
            notes: self.list_notes()?,
        })
    }

    pub fn replace_notes_snapshot(&self, snapshot: &NotesSnapshot) -> AppResult<()> {
        validate_notes_snapshot(snapshot)?;
        let txn = self.db.begin_write().map_err(storage_error)?;
        clear_prefix_in_txn(&txn, NOTE_FOLDERS_TABLE, NOTE_FOLDER_PREFIX)?;
        clear_prefix_in_txn(&txn, NOTES_TABLE, NOTE_DOCUMENT_PREFIX)?;
        for folder in &snapshot.folders {
            write_note_folder_in_txn(&txn, folder)?;
        }
        for note in &snapshot.notes {
            write_note_in_txn(&txn, note)?;
        }
        txn.commit().map_err(storage_error)?;
        Ok(())
    }
}

fn read_note_folders_in_txn(txn: &redb::WriteTransaction) -> AppResult<Vec<NoteFolder>> {
    let table = txn.open_table(NOTE_FOLDERS_TABLE).map_err(storage_error)?;
    let mut folders = Vec::new();
    for entry in table.iter().map_err(storage_error)? {
        let (key, value) = entry.map_err(storage_error)?;
        if key.value().starts_with(NOTE_FOLDER_PREFIX) {
            folders.push(deserialize_json::<NoteFolder>(value.value())?);
        }
    }
    sort_note_folders(&mut folders);
    Ok(folders)
}

fn read_notes_in_txn(txn: &redb::WriteTransaction) -> AppResult<Vec<NoteDocument>> {
    let table = txn.open_table(NOTES_TABLE).map_err(storage_error)?;
    let mut notes = Vec::new();
    for entry in table.iter().map_err(storage_error)? {
        let (key, value) = entry.map_err(storage_error)?;
        if key.value().starts_with(NOTE_DOCUMENT_PREFIX) {
            notes.push(deserialize_json::<NoteDocument>(value.value())?);
        }
    }
    sort_notes(&mut notes);
    Ok(notes)
}

fn write_note_folder_in_txn(txn: &redb::WriteTransaction, folder: &NoteFolder) -> AppResult<()> {
    write_json_in_txn(
        txn,
        NOTE_FOLDERS_TABLE,
        &entity_key(NOTE_FOLDER_PREFIX, &folder.id),
        folder,
    )
}

fn write_note_in_txn(txn: &redb::WriteTransaction, note: &NoteDocument) -> AppResult<()> {
    write_json_in_txn(
        txn,
        NOTES_TABLE,
        &entity_key(NOTE_DOCUMENT_PREFIX, &note.id),
        note,
    )
}

fn normalize_note_name(raw: &str) -> AppResult<String> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(AppError::Config("Note name cannot be empty".to_string()));
    }
    if value.chars().count() > MAX_NOTE_NAME_CHARS {
        return Err(AppError::Config(format!(
            "Note name cannot exceed {MAX_NOTE_NAME_CHARS} characters"
        )));
    }
    if value.contains('/') || value.contains('\\') {
        return Err(AppError::Config(
            "Note name cannot contain '/' or '\\'".to_string(),
        ));
    }
    if value.chars().any(char::is_control) {
        return Err(AppError::Config(
            "Note name cannot contain control characters".to_string(),
        ));
    }
    Ok(value.to_string())
}

fn normalize_or_unique_name(
    raw: Option<String>,
    fallback: &str,
    sibling_names: &HashSet<String>,
) -> AppResult<String> {
    if let Some(raw) = raw {
        let name = normalize_note_name(&raw)?;
        if sibling_names.contains(&name.to_lowercase()) {
            return Err(AppError::Config(format!(
                "A note item named '{name}' already exists in this folder"
            )));
        }
        return Ok(name);
    }

    let base = normalize_note_name(fallback)?;
    if !sibling_names.contains(&base.to_lowercase()) {
        return Ok(base);
    }
    for index in 2..10_000 {
        let candidate = format!("{base} {index}");
        if !sibling_names.contains(&candidate.to_lowercase()) {
            return Ok(candidate);
        }
    }
    Err(AppError::Config(
        "Could not generate a unique note name".to_string(),
    ))
}

fn sibling_names(
    folders: &[NoteFolder],
    notes: &[NoteDocument],
    parent_id: Option<&str>,
    exclude: Option<(&str, &str)>,
) -> HashSet<String> {
    let mut names = HashSet::new();
    for folder in folders {
        if folder.parent_id.as_deref() != parent_id
            || exclude == Some(("folder", folder.id.as_str()))
        {
            continue;
        }
        names.insert(folder.name.to_lowercase());
    }
    for note in notes {
        if note.parent_id.as_deref() != parent_id || exclude == Some(("note", note.id.as_str())) {
            continue;
        }
        names.insert(note.title.to_lowercase());
    }
    names
}

fn validate_unique_sibling_name(
    folders: &[NoteFolder],
    notes: &[NoteDocument],
    parent_id: Option<&str>,
    name: &str,
    exclude: Option<(&str, &str)>,
) -> AppResult<()> {
    if sibling_names(folders, notes, parent_id, exclude).contains(&name.to_lowercase()) {
        return Err(AppError::Config(format!(
            "A note item named '{name}' already exists in this folder"
        )));
    }
    Ok(())
}

fn validate_parent_exists(folders: &[NoteFolder], parent_id: Option<&str>) -> AppResult<()> {
    if let Some(parent_id) = parent_id {
        if !folders.iter().any(|folder| folder.id == parent_id) {
            return Err(AppError::Config(format!(
                "Folder '{parent_id}' does not exist"
            )));
        }
    }
    Ok(())
}

fn validate_not_descendant_folder(
    folders: &[NoteFolder],
    source_id: &str,
    target_parent_id: &str,
) -> AppResult<()> {
    let by_id: HashMap<&str, &NoteFolder> = folders
        .iter()
        .map(|folder| (folder.id.as_str(), folder))
        .collect();
    let mut current = Some(target_parent_id);
    let mut visited = HashSet::new();
    while let Some(folder_id) = current {
        if folder_id == source_id {
            return Err(AppError::Config(
                "A folder cannot be moved into its descendant".to_string(),
            ));
        }
        if !visited.insert(folder_id) {
            return Err(AppError::Config(
                "Folder hierarchy contains a cycle".to_string(),
            ));
        }
        current = by_id
            .get(folder_id)
            .and_then(|folder| folder.parent_id.as_deref());
    }
    Ok(())
}

fn collect_descendant_folder_ids(
    folders: &[NoteFolder],
    parent_id: &str,
    collected: &mut HashSet<String>,
) {
    for folder in folders {
        if folder.parent_id.as_deref() == Some(parent_id) && collected.insert(folder.id.clone()) {
            collect_descendant_folder_ids(folders, &folder.id, collected);
        }
    }
}

fn next_sort_order_for_parent(
    folders: &[NoteFolder],
    notes: &[NoteDocument],
    parent_id: Option<&str>,
) -> i64 {
    folders
        .iter()
        .filter(|folder| folder.parent_id.as_deref() == parent_id)
        .map(|folder| folder.sort_order)
        .chain(
            notes
                .iter()
                .filter(|note| note.parent_id.as_deref() == parent_id)
                .map(|note| note.sort_order),
        )
        .max()
        .unwrap_or(-1)
        .saturating_add(1)
}

fn validate_notes_snapshot(snapshot: &NotesSnapshot) -> AppResult<()> {
    let mut folder_ids = HashSet::new();
    let mut note_ids = HashSet::new();
    for folder in &snapshot.folders {
        normalize_note_name(&folder.name)?;
        if !folder_ids.insert(folder.id.as_str()) {
            return Err(AppError::Config(format!(
                "Duplicate note folder id '{}'",
                folder.id
            )));
        }
    }
    for note in &snapshot.notes {
        normalize_note_name(&note.title)?;
        if !note_ids.insert(note.id.as_str()) {
            return Err(AppError::Config(format!("Duplicate note id '{}'", note.id)));
        }
    }
    for folder in &snapshot.folders {
        if let Some(parent_id) = folder.parent_id.as_deref() {
            if !folder_ids.contains(parent_id) {
                return Err(AppError::Config(format!(
                    "Note folder '{}' has missing parent '{}'",
                    folder.id, parent_id
                )));
            }
            validate_not_descendant_folder(&snapshot.folders, &folder.id, parent_id)?;
        }
    }
    for note in &snapshot.notes {
        if let Some(parent_id) = note.parent_id.as_deref() {
            if !folder_ids.contains(parent_id) {
                return Err(AppError::Config(format!(
                    "Note '{}' has missing parent '{}'",
                    note.id, parent_id
                )));
            }
        }
    }
    for folder in &snapshot.folders {
        validate_unique_sibling_name(
            &snapshot.folders,
            &snapshot.notes,
            folder.parent_id.as_deref(),
            &folder.name,
            Some(("folder", &folder.id)),
        )?;
    }
    for note in &snapshot.notes {
        validate_unique_sibling_name(
            &snapshot.folders,
            &snapshot.notes,
            note.parent_id.as_deref(),
            &note.title,
            Some(("note", &note.id)),
        )?;
    }
    Ok(())
}

fn sort_note_folders(folders: &mut [NoteFolder]) {
    folders.sort_by(|left, right| {
        left.parent_id
            .cmp(&right.parent_id)
            .then(left.sort_order.cmp(&right.sort_order))
            .then(left.name.cmp(&right.name))
            .then(left.id.cmp(&right.id))
    });
}

fn sort_notes(notes: &mut [NoteDocument]) {
    notes.sort_by(|left, right| {
        left.parent_id
            .cmp(&right.parent_id)
            .then(left.sort_order.cmp(&right.sort_order))
            .then(left.title.cmp(&right.title))
            .then(left.id.cmp(&right.id))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_storage() -> Storage {
        let dir = std::env::temp_dir().join(format!(
            "nyaterm-notes-test-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&dir).expect("create temp dir");
        Storage::open(&dir).expect("open temp storage")
    }

    #[test]
    fn creates_root_note_and_nested_folder() {
        let storage = temp_storage();
        let folder = storage
            .create_note_folder(None, Some("Projects".to_string()))
            .expect("create folder");
        let note = storage
            .create_note(
                Some(folder.id.clone()),
                Some("Deploy".to_string()),
                Some("# Runbook".to_string()),
            )
            .expect("create note");

        assert_eq!(note.parent_id.as_deref(), Some(folder.id.as_str()));
        assert_eq!(note.revision, 1);
        assert_eq!(storage.list_note_folders().expect("folders").len(), 1);
        assert_eq!(storage.list_notes().expect("notes").len(), 1);
    }

    #[test]
    fn rejects_duplicate_sibling_names_case_insensitive() {
        let storage = temp_storage();
        storage
            .create_note(None, Some("Readme".to_string()), None)
            .expect("create note");

        let error = storage
            .create_note_folder(None, Some("readme".to_string()))
            .expect_err("duplicate should fail");

        assert!(error.to_string().contains("already exists"));
    }

    #[test]
    fn rejects_folder_move_to_self_or_descendant() {
        let storage = temp_storage();
        let root = storage
            .create_note_folder(None, Some("Root".to_string()))
            .expect("root folder");
        let child = storage
            .create_note_folder(Some(root.id.clone()), Some("Child".to_string()))
            .expect("child folder");

        let self_error = storage
            .move_note_node("folder", &root.id, Some(root.id.clone()), 0)
            .expect_err("self move should fail");
        assert!(self_error.to_string().contains("itself"));

        let descendant_error = storage
            .move_note_node("folder", &root.id, Some(child.id.clone()), 0)
            .expect_err("descendant move should fail");
        assert!(descendant_error.to_string().contains("descendant"));
    }

    #[test]
    fn update_note_increments_revision_and_rejects_stale_revision() {
        let storage = temp_storage();
        let note = storage
            .create_note(None, Some("Draft".to_string()), Some("one".to_string()))
            .expect("create note");
        let updated = storage
            .update_note(
                &note.id,
                "Draft".to_string(),
                "two".to_string(),
                note.revision,
                false,
            )
            .expect("update note");
        assert_eq!(updated.revision, note.revision + 1);

        let error = storage
            .update_note(
                &note.id,
                "Draft".to_string(),
                "three".to_string(),
                note.revision,
                false,
            )
            .expect_err("stale update should fail");
        assert!(error.to_string().contains("Revision conflict"));
    }

    #[test]
    fn recursive_delete_removes_folder_descendants_and_notes() {
        let storage = temp_storage();
        let root = storage
            .create_note_folder(None, Some("Root".to_string()))
            .expect("root folder");
        let child = storage
            .create_note_folder(Some(root.id.clone()), Some("Child".to_string()))
            .expect("child folder");
        storage
            .create_note(Some(child.id.clone()), Some("Leaf".to_string()), None)
            .expect("leaf note");

        let result = storage
            .delete_note_node("folder", &root.id)
            .expect("delete folder");

        assert_eq!(result.folder_count, 2);
        assert_eq!(result.note_count, 1);
        assert!(storage.list_note_folders().expect("folders").is_empty());
        assert!(storage.list_notes().expect("notes").is_empty());
    }

    #[test]
    fn snapshot_export_and_replace_roundtrip() {
        let storage = temp_storage();
        let folder = storage
            .create_note_folder(None, Some("Folder".to_string()))
            .expect("folder");
        storage
            .create_note(
                Some(folder.id.clone()),
                Some("Note".to_string()),
                Some("body".to_string()),
            )
            .expect("note");
        let snapshot = storage.load_notes_snapshot().expect("snapshot");

        let replacement = temp_storage();
        replacement
            .replace_notes_snapshot(&snapshot)
            .expect("replace snapshot");
        let roundtrip = replacement.load_notes_snapshot().expect("roundtrip");

        assert_eq!(roundtrip, snapshot);
    }
}
