use std::path::Path;
use uuid::Uuid;

use crate::error::MocoError;
use crate::models::{Project, Task};

/// Abstraction over the persistence backend.
///
/// All mutation methods take `&mut self` so implementations can manage
/// transactions or write buffers. Reads take `&self`.
///
/// A different backend (e.g. SQLite) can be introduced by implementing this
/// trait — no other code needs to change.
pub trait Store {
    // ── Projects ────────────────────────────────────────────────────────────

    /// Register a new project at the given canonical path.
    fn create_project(&mut self, name: &str, path: &Path) -> Result<Project, MocoError>;

    /// Look up a project by its exact registered path.
    fn get_project_by_path(&self, path: &Path) -> Result<Option<Project>, MocoError>;

    /// Return all registered projects.
    fn list_projects(&self) -> Result<Vec<Project>, MocoError>;

    // ── Tasks ────────────────────────────────────────────────────────────────

    /// Add a new task to a project (or the global list when `project_id` is `None`).
    fn add_task(
        &mut self,
        project_id: Option<Uuid>,
        content: &str,
        parent_id: Option<Uuid>,
    ) -> Result<Task, MocoError>;

    /// Retrieve an *open* task by its display index.
    fn get_open_task(
        &self,
        project_id: Option<Uuid>,
        display_index: u32,
    ) -> Result<Option<Task>, MocoError>;

    /// Return all tasks for a project (or global list), ordered by display_index.
    fn list_tasks(&self, project_id: Option<Uuid>) -> Result<Vec<Task>, MocoError>;

    /// Persist changes to an existing task (identified by `task.id`).
    fn update_task(&mut self, task: &Task) -> Result<(), MocoError>;

    /// Return the next available display index for open tasks in the given scope.
    fn next_open_display_index(&self, project_id: Option<Uuid>) -> Result<u32, MocoError>;

    /// Return the next available index for completed tasks in the given scope.
    fn next_completed_index(&self, project_id: Option<Uuid>) -> Result<u32, MocoError>;

    /// Return the next available index for deferred tasks in the given scope.
    fn next_deferred_index(&self, project_id: Option<Uuid>) -> Result<u32, MocoError>;

    /// Reassign sequential display_index values (1, 2, …) to all open tasks
    /// in the given scope, ordered by their current display_index.
    fn reindex_open_tasks(&mut self, project_id: Option<Uuid>) -> Result<(), MocoError>;

    /// Delete a project and all its tasks atomically.
    fn delete_project(&mut self, project: &Project) -> Result<(), MocoError>;
}
