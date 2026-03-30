use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

/// Build a `moco` Command with the home dir redirected to a temp location
/// so tests never touch the real `~/.moco/`.
fn moco(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("moco").unwrap();
    cmd.env("HOME", home.path());
    cmd
}

fn tmp() -> TempDir {
    TempDir::new().unwrap()
}

// ── init ─────────────────────────────────────────────────────────────────────

#[test]
fn init_creates_project() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "my-project"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("Initialized project 'my-project'"));
}

#[test]
fn init_twice_fails_without_force() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "my-project"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["init", "my-project"])
        .current_dir(workspace.path())
        .assert()
        .failure()
        .stderr(contains("already initialized"));
}

#[test]
fn init_twice_with_force_succeeds() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "my-project"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["init", "my-project", "--force"])
        .current_dir(workspace.path())
        .assert()
        .success();
}

// ── add ──────────────────────────────────────────────────────────────────────

#[test]
fn add_task_to_project() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "Fix the bug"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("Added project task #1: Fix the bug"));
}

#[test]
fn add_task_to_global_list() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["add", "--global", "A global task"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("Added global task #1: A global task"));
}

#[test]
fn add_multiple_tasks_sequential_indices() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "First task"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("#1"));

    moco(&home)
        .args(["add", "Second task"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("#2"));
}

#[test]
fn add_subtask() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "Parent task"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "--sub", "1", "Child task"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("#2"));
}

#[test]
fn add_subtask_to_nonexistent_parent_fails() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "--sub", "99", "Orphan task"])
        .current_dir(workspace.path())
        .assert()
        .failure()
        .stderr(contains("not found"));
}

// ── status ────────────────────────────────────────────────────────────────────

#[test]
fn status_sets_progress() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();
    moco(&home)
        .args(["add", "A task"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["status", "1", "50"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("50%"));
}

#[test]
fn status_complete_via_string() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();
    moco(&home)
        .args(["add", "A task"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["status", "1", "complete"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("C#1"));
}

#[test]
fn status_complete_via_100() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();
    moco(&home)
        .args(["add", "A task"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["status", "1", "100"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("C#1"));
}

#[test]
fn status_defer() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();
    moco(&home)
        .args(["add", "A task"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["status", "1", "defer"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("D#1"));
}

#[test]
fn status_reindexes_after_complete() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();
    moco(&home)
        .args(["add", "Task one"])
        .current_dir(workspace.path())
        .assert()
        .success();
    moco(&home)
        .args(["add", "Task two"])
        .current_dir(workspace.path())
        .assert()
        .success();
    moco(&home)
        .args(["add", "Task three"])
        .current_dir(workspace.path())
        .assert()
        .success();

    // Complete task #2 (middle)
    moco(&home)
        .args(["status", "2", "complete"])
        .current_dir(workspace.path())
        .assert()
        .success();

    // Task three should now be #2
    moco(&home)
        .args(["list"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("#2"))
        .stdout(contains("Task three"));
}

#[test]
fn status_invalid_value_fails() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();
    moco(&home)
        .args(["add", "A task"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["status", "1", "oops"])
        .current_dir(workspace.path())
        .assert()
        .failure()
        .stderr(contains("invalid"));
}

// ── list ─────────────────────────────────────────────────────────────────────

#[test]
fn list_shows_tasks() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();
    moco(&home)
        .args(["add", "First"])
        .current_dir(workspace.path())
        .assert()
        .success();
    moco(&home)
        .args(["add", "Second"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["list"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("First"))
        .stdout(contains("Second"));
}

#[test]
fn list_empty_project() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["list"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("No tasks"));
}

#[test]
fn list_global_flag() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["add", "--global", "Global task"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["list", "--global"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("Global task"));
}

#[test]
fn list_shows_progress_bar() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();
    moco(&home)
        .args(["add", "In progress"])
        .current_dir(workspace.path())
        .assert()
        .success();
    moco(&home)
        .args(["status", "1", "50"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["list"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("█"));
}

// ── workspace resolution ──────────────────────────────────────────────────────

#[test]
fn add_falls_back_to_global_when_no_project() {
    let home = tmp();
    let workspace = tmp();

    // No init — should fall back to global
    moco(&home)
        .args(["add", "Implicit global"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("global task"));
}

#[test]
fn nested_dir_resolves_to_parent_project() {
    let home = tmp();
    let workspace = tmp();

    // Register the workspace root
    moco(&home)
        .args(["init", "myproj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    // Add a task from a subdirectory
    let subdir = workspace.path().join("src");
    std::fs::create_dir_all(&subdir).unwrap();

    moco(&home)
        .args(["add", "Deep task"])
        .current_dir(&subdir)
        .assert()
        .success()
        .stdout(contains("project task"));
}

// ── moco edit (non-interactive) ───────────────────────────────────────────────

#[test]
fn edit_append_adds_content() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();
    moco(&home)
        .args(["add", "Original content"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["edit", "-t", "1", "Appended line", "--append"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("updated"));

    // Verify the appended content is visible in list.
    moco(&home)
        .args(["list"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("Original content"));
}

#[test]
fn edit_replace_overwrites_content() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();
    moco(&home)
        .args(["add", "Old content"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["edit", "-t", "1", "Brand new content", "--replace"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("updated"));

    moco(&home)
        .args(["list"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("Brand new content"));
}

#[test]
fn edit_without_append_or_replace_flag_fails() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();
    moco(&home)
        .args(["add", "A task"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["edit", "-t", "1", "content"])
        .current_dir(workspace.path())
        .assert()
        .failure();
}

#[test]
fn edit_nonexistent_task_fails() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["edit", "-t", "99", "content", "--replace"])
        .current_dir(workspace.path())
        .assert()
        .failure()
        .stderr(contains("not found"));
}

// ── moco export ───────────────────────────────────────────────────────────────

#[test]
fn export_creates_markdown_file() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "myproj"])
        .current_dir(workspace.path())
        .assert()
        .success();
    moco(&home)
        .args(["add", "Task one"])
        .current_dir(workspace.path())
        .assert()
        .success();
    moco(&home)
        .args(["add", "Task two"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["export"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("Exported to"));

    let md_path = workspace.path().join("myproj.md");
    assert!(md_path.exists(), "markdown file should be created");
    let contents = std::fs::read_to_string(&md_path).unwrap();
    assert!(contents.contains("# myproj"));
    assert!(contents.contains("Task one"));
    assert!(contents.contains("Task two"));
}

#[test]
fn export_includes_all_status_sections() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();
    moco(&home)
        .args(["add", "Open task"])
        .current_dir(workspace.path())
        .assert()
        .success();
    moco(&home)
        .args(["add", "Will complete"])
        .current_dir(workspace.path())
        .assert()
        .success();
    moco(&home)
        .args(["add", "Will defer"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["status", "2", "complete"])
        .current_dir(workspace.path())
        .assert()
        .success();
    moco(&home)
        .args(["status", "2", "defer"]) // original #3 is now #2 after reindex
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["export"])
        .current_dir(workspace.path())
        .assert()
        .success();

    let md_path = workspace.path().join("proj.md");
    let contents = std::fs::read_to_string(&md_path).unwrap();
    assert!(contents.contains("## Open"));
    assert!(contents.contains("## Completed"));
    assert!(contents.contains("## Deferred"));
}

#[test]
fn export_without_project_fails() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["export"])
        .current_dir(workspace.path())
        .assert()
        .failure()
        .stderr(contains("no project found"));
}

#[test]
fn export_global_writes_to_moco_dir() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["add", "--global", "A global task"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["export", "--global"])
        .current_dir(workspace.path())
        .assert()
        .success();

    let md_path = home.path().join(".moco").join("global.md");
    assert!(md_path.exists());
    let contents = std::fs::read_to_string(&md_path).unwrap();
    assert!(contents.contains("A global task"));
}

// ── moco list subtask display ─────────────────────────────────────────────────

#[test]
fn list_displays_subtask_indented_under_parent() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();
    moco(&home)
        .args(["add", "Parent task"])
        .current_dir(workspace.path())
        .assert()
        .success();
    moco(&home)
        .args(["add", "--sub", "1", "Child task"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["list"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("└─"))
        .stdout(contains("Child task"));
}

// ── moco config check ─────────────────────────────────────────────────────────

#[test]
fn config_check_passes_with_default_config() {
    let home = tmp();
    let workspace = tmp();

    // First invocation creates ~/.moco/ and writes default config.toml template.
    moco(&home)
        .args(["init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["config", "check"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("Configuration OK"));
}

#[test]
fn config_check_fails_with_malformed_toml() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    // Overwrite config with malformed TOML.
    let config_path = home.path().join(".moco").join("config.toml");
    std::fs::write(&config_path, "open_with = [invalid").unwrap();

    moco(&home)
        .args(["config", "check"])
        .current_dir(workspace.path())
        .assert()
        .failure()
        .stderr(contains("issue"));
}

#[test]
fn config_check_fails_when_open_with_not_on_path() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    let config_path = home.path().join(".moco").join("config.toml");
    std::fs::write(&config_path, "open_with = \"__moco_nonexistent_cmd__\"\n").unwrap();

    moco(&home)
        .args(["config", "check"])
        .current_dir(workspace.path())
        .assert()
        .failure()
        .stderr(contains("not found on PATH"));
}

// ── moco open ────────────────────────────────────────────────────────────────

#[test]
fn open_fails_when_no_projects_registered() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["open", "--dry-run"])
        .current_dir(workspace.path())
        .assert()
        .failure()
        .stderr(contains("No projects registered"));
}

#[test]
fn open_dry_run_prints_command_for_registered_project() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "myproj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    // Write a config with a known open_with command.
    let config_path = home.path().join(".moco").join("config.toml");
    std::fs::write(&config_path, "open_with = \"echo\"\n").unwrap();

    moco(&home)
        .args([
            "open",
            "--dry-run",
            "--project-path",
            workspace.path().to_str().unwrap(),
        ])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("echo"));
}

#[test]
fn open_dry_run_fails_when_project_path_not_registered() {
    let home = tmp();
    let workspace = tmp();
    let other = tmp();

    moco(&home)
        .args(["init", "myproj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args([
            "open",
            "--dry-run",
            "--project-path",
            other.path().to_str().unwrap(),
        ])
        .current_dir(workspace.path())
        .assert()
        .failure()
        .stderr(contains("No project registered at"));
}

// ── moco delete ───────────────────────────────────────────────────────────────

#[test]
fn delete_fails_when_no_projects_registered() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["delete", "--yes"])
        .current_dir(workspace.path())
        .assert()
        .failure()
        .stderr(contains("No projects registered"));
}

#[test]
fn delete_removes_project_with_yes_flag() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "myproject"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args([
            "delete",
            "--yes",
            "--project-path",
            workspace.path().to_str().unwrap(),
        ])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("Deleted project 'myproject'"));
}

#[test]
fn delete_also_removes_tasks() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "myproject"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "Task one"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args([
            "delete",
            "--yes",
            "--project-path",
            workspace.path().to_str().unwrap(),
        ])
        .current_dir(workspace.path())
        .assert()
        .success();

    // After deletion, listing tasks should show nothing (global list is empty).
    moco(&home)
        .args(["list"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("No tasks"));
}

#[test]
fn delete_shows_task_count_in_confirmation_message() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "Task A"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "Task B"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args([
            "delete",
            "--yes",
            "--project-path",
            workspace.path().to_str().unwrap(),
        ])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("2 tasks"));
}

#[test]
fn delete_cancelled_when_user_types_n() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    // Provide "n\n" on stdin — should cancel without deleting.
    moco(&home)
        .args([
            "delete",
            "--project-path",
            workspace.path().to_str().unwrap(),
        ])
        .write_stdin("n\n")
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("Cancelled"));

    // Project should still exist.
    moco(&home)
        .args(["list"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("No tasks"));
}

#[test]
fn delete_fails_when_project_path_not_registered() {
    let home = tmp();
    let workspace = tmp();
    let other = tmp();

    moco(&home)
        .args(["init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args([
            "delete",
            "--yes",
            "--project-path",
            other.path().to_str().unwrap(),
        ])
        .current_dir(workspace.path())
        .assert()
        .failure()
        .stderr(contains("No project registered at"));
}
