use std::path::Path;

use redb::{Database, ReadableTable, TableDefinition};
use uuid::Uuid;

use crate::db::Store;
use crate::error::MocoError;
use crate::models::{Note, Project, Task, TaskStatus};

/// redb table: canonical path (string) → JSON-encoded Project
const PROJECTS: TableDefinition<&str, &str> = TableDefinition::new("projects");

/// redb table: "<project_id_or_global>/<task_uuid>" → JSON-encoded Task
const TASKS: TableDefinition<&str, &str> = TableDefinition::new("tasks");

/// redb table: "<project_id_or_global>/<note_uuid>" → JSON-encoded Note
const NOTES: TableDefinition<&str, &str> = TableDefinition::new("notes");

/// The global scope key used when no project is registered.
const GLOBAL_SCOPE: &str = "global";

fn scope_key(project_id: Option<Uuid>) -> String {
    match project_id {
        Some(id) => id.to_string(),
        None => GLOBAL_SCOPE.to_string(),
    }
}

fn task_key(project_id: Option<Uuid>, task_id: Uuid) -> String {
    format!("{}/{}", scope_key(project_id), task_id)
}

fn note_key(project_id: Option<Uuid>, note_id: Uuid) -> String {
    format!("{}/{}", scope_key(project_id), note_id)
}

/// `redb`-backed implementation of [`Store`].
pub struct RedbStore {
    db: Database,
}

impl RedbStore {
    /// Open (or create) the database at `path`.
    pub fn open(path: &Path) -> Result<Self, MocoError> {
        let db = Database::create(path)?;
        let store = Self { db };
        store.ensure_tables()?;
        Ok(store)
    }

    fn ensure_tables(&self) -> Result<(), MocoError> {
        let tx = self.db.begin_write()?;
        tx.open_table(PROJECTS)?;
        tx.open_table(TASKS)?;
        tx.open_table(NOTES)?;
        tx.commit()?;
        Ok(())
    }

    fn serialize<T: serde::Serialize>(value: &T) -> Result<String, MocoError> {
        Ok(serde_json::to_string(value)?)
    }

    fn deserialize<T: serde::de::DeserializeOwned>(s: &str) -> Result<T, MocoError> {
        Ok(serde_json::from_str(s)?)
    }
}

impl Store for RedbStore {
    // ── Projects ─────────────────────────────────────────────────────────────

    fn create_project(&mut self, name: &str, path: &Path) -> Result<Project, MocoError> {
        let project = Project::new(name, path.to_path_buf());
        let key = path.to_string_lossy().to_string();
        let value = Self::serialize(&project)?;

        let tx = self.db.begin_write()?;
        {
            let mut table = tx.open_table(PROJECTS)?;
            table.insert(key.as_str(), value.as_str())?;
        }
        tx.commit()?;

        Ok(project)
    }

    fn get_project_by_path(&self, path: &Path) -> Result<Option<Project>, MocoError> {
        let key = path.to_string_lossy().to_string();
        let tx = self.db.begin_read()?;
        let table = tx.open_table(PROJECTS)?;

        match table.get(key.as_str())? {
            Some(v) => Ok(Some(Self::deserialize(v.value())?)),
            None => Ok(None),
        }
    }

    fn list_projects(&self) -> Result<Vec<Project>, MocoError> {
        let tx = self.db.begin_read()?;
        let table = tx.open_table(PROJECTS)?;

        let mut projects = Vec::new();
        for entry in table.iter()? {
            let (_, v) = entry?;
            projects.push(Self::deserialize::<Project>(v.value())?);
        }
        // Most recently active first.
        projects.sort_by(|a, b| b.last_active.cmp(&a.last_active));
        Ok(projects)
    }

    fn update_project(&mut self, project: &Project) -> Result<(), MocoError> {
        let key = project.path.to_string_lossy().to_string();
        let value = Self::serialize(project)?;

        let tx = self.db.begin_write()?;
        {
            let mut table = tx.open_table(PROJECTS)?;
            table.insert(key.as_str(), value.as_str())?;
        }
        tx.commit()?;
        Ok(())
    }

    fn relocate_project(&mut self, old_path: &Path, project: &Project) -> Result<(), MocoError> {
        let old_key = old_path.to_string_lossy().to_string();
        let new_key = project.path.to_string_lossy().to_string();
        let value = Self::serialize(project)?;

        let tx = self.db.begin_write()?;
        {
            let mut table = tx.open_table(PROJECTS)?;
            table.remove(old_key.as_str())?;
            table.insert(new_key.as_str(), value.as_str())?;
        }
        tx.commit()?;
        Ok(())
    }

    fn touch_project(&mut self, project_id: Uuid) -> Result<(), MocoError> {
        let tx = self.db.begin_read()?;
        let table = tx.open_table(PROJECTS)?;

        // Scan for the project with the matching ID.
        let mut found: Option<(String, Project)> = None;
        for entry in table.iter()? {
            let (k, v) = entry?;
            let project = Self::deserialize::<Project>(v.value())?;
            if project.id == project_id {
                found = Some((k.value().to_string(), project));
                break;
            }
        }
        drop(table);
        drop(tx);

        if let Some((key, mut project)) = found {
            project.last_active = chrono::Utc::now();
            let value = Self::serialize(&project)?;
            let tx = self.db.begin_write()?;
            {
                let mut table = tx.open_table(PROJECTS)?;
                table.insert(key.as_str(), value.as_str())?;
            }
            tx.commit()?;
        }

        Ok(())
    }

    // ── Tasks ─────────────────────────────────────────────────────────────────

    fn add_task(
        &mut self,
        project_id: Option<Uuid>,
        content: &str,
        parent_id: Option<Uuid>,
    ) -> Result<Task, MocoError> {
        let display_index = self.next_open_display_index(project_id)?;
        let task = Task::new(project_id, content, display_index, parent_id);
        let key = task_key(project_id, task.id);
        let value = Self::serialize(&task)?;

        let tx = self.db.begin_write()?;
        {
            let mut table = tx.open_table(TASKS)?;
            table.insert(key.as_str(), value.as_str())?;
        }
        tx.commit()?;

        if let Some(pid) = project_id {
            self.touch_project(pid)?;
        }

        Ok(task)
    }

    fn get_open_task(
        &self,
        project_id: Option<Uuid>,
        display_index: u32,
    ) -> Result<Option<Task>, MocoError> {
        let tasks = self.list_tasks(project_id)?;
        Ok(tasks
            .into_iter()
            .find(|t| t.status == TaskStatus::Open && t.display_index == display_index))
    }

    fn list_tasks(&self, project_id: Option<Uuid>) -> Result<Vec<Task>, MocoError> {
        let prefix = format!("{}/", scope_key(project_id));
        let tx = self.db.begin_read()?;
        let table = tx.open_table(TASKS)?;

        let mut tasks: Vec<Task> = Vec::new();
        for entry in table.iter()? {
            let (k, v) = entry?;
            if k.value().starts_with(&prefix) {
                tasks.push(Self::deserialize::<Task>(v.value())?);
            }
        }

        tasks.sort_by_key(|t| t.display_index);
        Ok(tasks)
    }

    fn update_task(&mut self, task: &Task) -> Result<(), MocoError> {
        let key = task_key(task.project_id, task.id);
        let value = Self::serialize(task)?;

        let tx = self.db.begin_write()?;
        {
            let mut table = tx.open_table(TASKS)?;
            table.insert(key.as_str(), value.as_str())?;
        }
        tx.commit()?;

        if let Some(pid) = task.project_id {
            self.touch_project(pid)?;
        }

        Ok(())
    }

    fn next_open_display_index(&self, project_id: Option<Uuid>) -> Result<u32, MocoError> {
        let tasks = self.list_tasks(project_id)?;
        let max = tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Open)
            .map(|t| t.display_index)
            .max()
            .unwrap_or(0);
        Ok(max + 1)
    }

    fn next_completed_index(&self, project_id: Option<Uuid>) -> Result<u32, MocoError> {
        let tasks = self.list_tasks(project_id)?;
        let max = tasks
            .iter()
            .filter_map(|t| t.completed_index)
            .max()
            .unwrap_or(0);
        Ok(max + 1)
    }

    fn next_deferred_index(&self, project_id: Option<Uuid>) -> Result<u32, MocoError> {
        let tasks = self.list_tasks(project_id)?;
        let max = tasks
            .iter()
            .filter_map(|t| t.deferred_index)
            .max()
            .unwrap_or(0);
        Ok(max + 1)
    }

    fn reindex_open_tasks(&mut self, project_id: Option<Uuid>) -> Result<(), MocoError> {
        let tasks = self.list_tasks(project_id)?;
        let mut open_tasks: Vec<Task> = tasks
            .into_iter()
            .filter(|t| t.status == TaskStatus::Open)
            .collect();

        // Preserve relative order by sorting on existing display_index.
        open_tasks.sort_by_key(|t| t.display_index);

        for (i, mut task) in open_tasks.into_iter().enumerate() {
            let new_index = (i + 1) as u32;
            if task.display_index != new_index {
                task.display_index = new_index;
                task.updated_at = chrono::Utc::now();
                self.update_task(&task)?;
            }
        }

        Ok(())
    }

    fn list_tasks_by_label(&self, label: &str) -> Result<Vec<(Project, Vec<Task>)>, MocoError> {
        let projects = self.list_projects()?;
        let mut result = Vec::new();

        for project in projects {
            if project.labels.iter().any(|l| l == label) {
                let tasks = self.list_tasks(Some(project.id))?;
                result.push((project, tasks));
            }
        }

        Ok(result)
    }

    fn list_tasks_by_tag(
        &self,
        project_id: Option<Uuid>,
        tag: &str,
    ) -> Result<Vec<Task>, MocoError> {
        let tasks = self.list_tasks(project_id)?;
        Ok(tasks
            .into_iter()
            .filter(|t| t.tags.iter().any(|tg| tg == tag))
            .collect())
    }

    fn delete_project(&mut self, project: &Project) -> Result<(), MocoError> {
        let project_key = project.path.to_string_lossy().to_string();
        let scope_prefix = format!("{}/", project.id);

        let tx = self.db.begin_write()?;
        {
            let mut projects_table = tx.open_table(PROJECTS)?;
            projects_table.remove(project_key.as_str())?;

            // Delete all tasks for this project.
            let mut tasks_table = tx.open_table(TASKS)?;
            let task_keys: Vec<String> = tasks_table
                .iter()?
                .filter_map(|entry| {
                    let (k, _) = entry.ok()?;
                    let key = k.value().to_string();
                    if key.starts_with(&scope_prefix) { Some(key) } else { None }
                })
                .collect();
            for key in task_keys {
                tasks_table.remove(key.as_str())?;
            }

            // Delete all notes for this project.
            let mut notes_table = tx.open_table(NOTES)?;
            let note_keys: Vec<String> = notes_table
                .iter()?
                .filter_map(|entry| {
                    let (k, _) = entry.ok()?;
                    let key = k.value().to_string();
                    if key.starts_with(&scope_prefix) { Some(key) } else { None }
                })
                .collect();
            for key in note_keys {
                notes_table.remove(key.as_str())?;
            }
        }
        tx.commit()?;

        Ok(())
    }

    // ── Notes ─────────────────────────────────────────────────────────────────

    fn add_note(
        &mut self,
        project_id: Option<Uuid>,
        title: &str,
        content: &str,
    ) -> Result<Note, MocoError> {
        let display_index = self.next_note_display_index(project_id)?;
        let note = Note::new(project_id, title, content, display_index);
        let key = note_key(project_id, note.id);
        let value = Self::serialize(&note)?;

        let tx = self.db.begin_write()?;
        {
            let mut table = tx.open_table(NOTES)?;
            table.insert(key.as_str(), value.as_str())?;
        }
        tx.commit()?;

        Ok(note)
    }

    fn get_note(
        &self,
        project_id: Option<Uuid>,
        display_index: u32,
    ) -> Result<Option<Note>, MocoError> {
        let notes = self.list_notes(project_id)?;
        Ok(notes.into_iter().find(|n| n.display_index == display_index))
    }

    fn list_notes(&self, project_id: Option<Uuid>) -> Result<Vec<Note>, MocoError> {
        let prefix = format!("{}/", scope_key(project_id));
        let tx = self.db.begin_read()?;
        let table = tx.open_table(NOTES)?;

        let mut notes: Vec<Note> = Vec::new();
        for entry in table.iter()? {
            let (k, v) = entry?;
            if k.value().starts_with(&prefix) {
                notes.push(Self::deserialize::<Note>(v.value())?);
            }
        }

        notes.sort_by_key(|n| n.display_index);
        Ok(notes)
    }

    fn update_note(&mut self, note: &Note) -> Result<(), MocoError> {
        let key = note_key(note.project_id, note.id);
        let value = Self::serialize(note)?;

        let tx = self.db.begin_write()?;
        {
            let mut table = tx.open_table(NOTES)?;
            table.insert(key.as_str(), value.as_str())?;
        }
        tx.commit()?;

        Ok(())
    }

    fn delete_note(&mut self, note_id: Uuid) -> Result<(), MocoError> {
        // Notes can be in any scope; scan all to find by id.
        let tx = self.db.begin_write()?;
        {
            let mut table = tx.open_table(NOTES)?;
            let key_to_delete: Option<String> = table
                .iter()?
                .filter_map(|entry| {
                    let (k, v) = entry.ok()?;
                    let note: Note = Self::deserialize(v.value()).ok()?;
                    if note.id == note_id {
                        Some(k.value().to_string())
                    } else {
                        None
                    }
                })
                .next();

            if let Some(key) = key_to_delete {
                table.remove(key.as_str())?;
            }
        }
        tx.commit()?;

        Ok(())
    }

    fn next_note_display_index(&self, project_id: Option<Uuid>) -> Result<u32, MocoError> {
        let notes = self.list_notes(project_id)?;
        let max = notes.iter().map(|n| n.display_index).max().unwrap_or(0);
        Ok(max + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn temp_store() -> (TempDir, RedbStore) {
        let dir = TempDir::new().unwrap();
        let store = RedbStore::open(&dir.path().join("test.db")).unwrap();
        (dir, store)
    }

    // ── Project tests ─────────────────────────────────────────────────────────

    #[test]
    fn create_and_retrieve_project() {
        let (_dir, mut store) = temp_store();
        let path = PathBuf::from("/tmp/myproject");
        let p = store.create_project("myproject", &path).unwrap();
        assert_eq!(p.name, "myproject");

        let retrieved = store.get_project_by_path(&path).unwrap().unwrap();
        assert_eq!(retrieved.id, p.id);
        assert_eq!(retrieved.name, "myproject");
    }

    #[test]
    fn get_project_by_path_missing_returns_none() {
        let (_dir, store) = temp_store();
        let result = store
            .get_project_by_path(Path::new("/nonexistent"))
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn list_projects_returns_all() {
        let (_dir, mut store) = temp_store();
        store
            .create_project("a", Path::new("/tmp/a"))
            .unwrap();
        store
            .create_project("b", Path::new("/tmp/b"))
            .unwrap();
        let projects = store.list_projects().unwrap();
        assert_eq!(projects.len(), 2);
    }

    // ── Task tests ────────────────────────────────────────────────────────────

    #[test]
    fn add_task_assigns_sequential_indices() {
        let (_dir, mut store) = temp_store();
        let t1 = store.add_task(None, "first", None).unwrap();
        let t2 = store.add_task(None, "second", None).unwrap();
        assert_eq!(t1.display_index, 1);
        assert_eq!(t2.display_index, 2);
    }

    #[test]
    fn list_tasks_ordered_by_display_index() {
        let (_dir, mut store) = temp_store();
        store.add_task(None, "one", None).unwrap();
        store.add_task(None, "two", None).unwrap();
        store.add_task(None, "three", None).unwrap();
        let tasks = store.list_tasks(None).unwrap();
        let indices: Vec<u32> = tasks.iter().map(|t| t.display_index).collect();
        assert_eq!(indices, vec![1, 2, 3]);
    }

    #[test]
    fn get_open_task_by_display_index() {
        let (_dir, mut store) = temp_store();
        let added = store.add_task(None, "my task", None).unwrap();
        let found = store.get_open_task(None, added.display_index).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, added.id);
    }

    #[test]
    fn update_task_persists_changes() {
        let (_dir, mut store) = temp_store();
        let mut task = store.add_task(None, "initial", None).unwrap();
        task.content = "updated".to_string();
        store.update_task(&task).unwrap();

        let tasks = store.list_tasks(None).unwrap();
        assert_eq!(tasks[0].content, "updated");
    }

    #[test]
    fn reindex_open_tasks_fills_gaps() {
        let (_dir, mut store) = temp_store();
        let _t1 = store.add_task(None, "one", None).unwrap();
        let mut t2 = store.add_task(None, "two", None).unwrap();
        let _t3 = store.add_task(None, "three", None).unwrap();

        // Simulate t2 being completed: mark it complete with a high display_index
        // (leaving a gap at 2)
        t2.status = TaskStatus::Complete;
        t2.completed_index = Some(1);
        t2.display_index = 0; // completed tasks don't use display_index
        store.update_task(&t2).unwrap();

        store.reindex_open_tasks(None).unwrap();

        let open: Vec<Task> = store
            .list_tasks(None)
            .unwrap()
            .into_iter()
            .filter(|t| t.status == TaskStatus::Open)
            .collect();
        let indices: Vec<u32> = open.iter().map(|t| t.display_index).collect();
        assert_eq!(indices, vec![1, 2]);
    }

    #[test]
    fn tasks_are_scoped_to_project() {
        let (_dir, mut store) = temp_store();
        let project_id = Uuid::new_v4();
        store.add_task(Some(project_id), "project task", None).unwrap();
        store.add_task(None, "global task", None).unwrap();

        let project_tasks = store.list_tasks(Some(project_id)).unwrap();
        let global_tasks = store.list_tasks(None).unwrap();

        assert_eq!(project_tasks.len(), 1);
        assert_eq!(global_tasks.len(), 1);
        assert_eq!(project_tasks[0].content, "project task");
        assert_eq!(global_tasks[0].content, "global task");
    }

    #[test]
    fn next_completed_index_increments() {
        let (_dir, mut store) = temp_store();
        assert_eq!(store.next_completed_index(None).unwrap(), 1);

        let mut t = store.add_task(None, "done", None).unwrap();
        t.status = TaskStatus::Complete;
        t.completed_index = Some(1);
        store.update_task(&t).unwrap();

        assert_eq!(store.next_completed_index(None).unwrap(), 2);
    }

    #[test]
    fn next_deferred_index_increments() {
        let (_dir, mut store) = temp_store();
        assert_eq!(store.next_deferred_index(None).unwrap(), 1);

        let mut t = store.add_task(None, "deferred", None).unwrap();
        t.status = TaskStatus::Defer;
        t.deferred_index = Some(1);
        store.update_task(&t).unwrap();

        assert_eq!(store.next_deferred_index(None).unwrap(), 2);
    }

    #[test]
    fn touch_project_updates_last_active_and_list_sorts_by_recency() {
        let (_dir, mut store) = temp_store();
        let p1 = store.create_project("first", &PathBuf::from("/tmp/p1")).unwrap();
        let p2 = store.create_project("second", &PathBuf::from("/tmp/p2")).unwrap();

        // Touch p1 after a tiny sleep so timestamps differ.
        std::thread::sleep(std::time::Duration::from_millis(10));
        store.touch_project(p1.id).unwrap();

        let projects = store.list_projects().unwrap();
        // p1 was touched last, so it should appear first.
        assert_eq!(projects[0].name, "first");
        assert_eq!(projects[1].name, "second");
        assert!(projects[0].last_active > projects[1].last_active);
    }

    #[test]
    fn touch_project_noop_for_unknown_id() {
        let (_dir, mut store) = temp_store();
        // Should not error even though the ID doesn't exist.
        let result = store.touch_project(Uuid::new_v4());
        assert!(result.is_ok());
    }

    #[test]
    fn relocate_project_moves_path_and_preserves_tasks() {
        let (_dir, mut store) = temp_store();
        let old_path = PathBuf::from("/tmp/old_home");
        let new_path = PathBuf::from("/tmp/new_home");

        let project = store.create_project("mover", &old_path).unwrap();
        store.add_task(Some(project.id), "task A", None).unwrap();

        let mut moved = project.clone();
        moved.path = new_path.clone();
        store.relocate_project(&old_path, &moved).unwrap();

        // Old path must be gone.
        assert!(store.get_project_by_path(&old_path).unwrap().is_none());
        // New path must be present.
        let found = store.get_project_by_path(&new_path).unwrap().unwrap();
        assert_eq!(found.name, "mover");
        // Exactly one entry in the project list.
        assert_eq!(store.list_projects().unwrap().len(), 1);
        // Tasks are preserved under the project ID.
        let tasks = store.list_tasks(Some(project.id)).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].content, "task A");
    }

    #[test]
    fn delete_project_removes_project_and_tasks() {
        let (_dir, mut store) = temp_store();
        let path = PathBuf::from("/tmp/todelete");
        let project = store.create_project("todelete", &path).unwrap();
        store.add_task(Some(project.id), "task one", None).unwrap();
        store.add_task(Some(project.id), "task two", None).unwrap();

        store.delete_project(&project).unwrap();

        assert!(store.get_project_by_path(&path).unwrap().is_none());
        assert!(store.list_tasks(Some(project.id)).unwrap().is_empty());
    }

    #[test]
    fn delete_project_does_not_affect_other_projects() {
        let (_dir, mut store) = temp_store();
        let p1 = store.create_project("one", Path::new("/tmp/one")).unwrap();
        let p2 = store.create_project("two", Path::new("/tmp/two")).unwrap();
        store.add_task(Some(p1.id), "p1 task", None).unwrap();
        store.add_task(Some(p2.id), "p2 task", None).unwrap();

        store.delete_project(&p1).unwrap();

        assert!(store.get_project_by_path(Path::new("/tmp/one")).unwrap().is_none());
        assert!(store.get_project_by_path(Path::new("/tmp/two")).unwrap().is_some());
        assert!(store.list_tasks(Some(p1.id)).unwrap().is_empty());
        assert_eq!(store.list_tasks(Some(p2.id)).unwrap().len(), 1);
    }
}
