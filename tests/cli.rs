use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
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
        .args(["project", "init", "my-project"])
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
        .args(["project", "init", "my-project"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "init", "my-project"])
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
        .args(["project", "init", "my-project"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "init", "my-project", "--force"])
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
        .args(["project", "init", "proj"])
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
        .args(["project", "init", "proj"])
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
        .args(["project", "init", "proj"])
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
        .stdout(contains("S#1.1"));
}

#[test]
fn add_subtask_to_nonexistent_parent_fails() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
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
        .args(["project", "init", "proj"])
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
        .args(["project", "init", "proj"])
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
        .args(["project", "init", "proj"])
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
        .args(["project", "init", "proj"])
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
        .args(["project", "init", "proj"])
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
        .args(["project", "init", "proj"])
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
        .args(["project", "init", "proj"])
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
        .args(["project", "init", "proj"])
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
        .args(["project", "init", "proj"])
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
        .args(["project", "init", "myproj"])
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
        .args(["project", "init", "proj"])
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
        .args(["project", "init", "proj"])
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
        .args(["project", "init", "proj"])
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
        .args(["project", "init", "proj"])
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
        .args(["project", "init", "myproj"])
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
        .args(["project", "export"])
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
        .args(["project", "init", "proj"])
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
        .args(["project", "export"])
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
        .args(["project", "export"])
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
        .args(["project", "export", "--global"])
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
        .args(["project", "init", "proj"])
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
        .args(["project", "init", "proj"])
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
        .args(["project", "init", "proj"])
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
        .args(["project", "init", "proj"])
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
        .args(["project", "open", "--dry-run"])
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
        .args(["project", "init", "myproj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    // Write a config with a known open_with command.
    let config_path = home.path().join(".moco").join("config.toml");
    std::fs::write(&config_path, "open_with = \"echo\"\n").unwrap();

    moco(&home)
        .args(["project", "open",
            
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
        .args(["project", "init", "myproj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "open",
            
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
        .args(["project", "delete", "--yes"])
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
        .args(["project", "init", "myproject"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "delete",
            
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
        .args(["project", "init", "myproject"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "Task one"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "delete",
            
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
        .args(["project", "init", "proj"])
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
        .args(["project", "delete",
            
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
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    // Provide "n\n" on stdin — should cancel without deleting.
    moco(&home)
        .args(["project", "delete",
            
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
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "delete",
            
            "--yes",
            "--project-path",
            other.path().to_str().unwrap(),
        ])
        .current_dir(workspace.path())
        .assert()
        .failure()
        .stderr(contains("No project registered at"));
}

// ── moco label ───────────────────────────────────────────────────────────────

#[test]
fn label_add_to_project() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "label", "add", "mylab"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("Added label 'mylab'"));
}

#[test]
fn label_list_shows_labels() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "label", "add", "mylab"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "label", "list"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("mylab"));
}

#[test]
fn label_remove_from_project() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "label", "add", "mylab"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "label", "remove", "mylab"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("Removed label 'mylab'"));

    moco(&home)
        .args(["project", "label", "list"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("No labels"));
}

#[test]
fn label_fails_outside_project() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "label", "add", "mylab"])
        .current_dir(workspace.path())
        .assert()
        .failure()
        .stderr(contains("Not in a registered project"));
}

#[test]
fn list_label_filter_shows_project_tasks() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "labeled-proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "label", "add", "mylab"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "Task in labeled project"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["list", "--label", "mylab"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("labeled-proj"))
        .stdout(contains("Task in labeled project"));
}

// ── moco tag ─────────────────────────────────────────────────────────────────

#[test]
fn tag_add_to_task() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "Fix bug"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["tag", "add", "1", "mytag"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("Added tag 'mytag'"));
}

#[test]
fn tag_list_shows_tags() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "Fix bug"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["tag", "add", "1", "mytag"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["tag", "list", "1"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("mytag"));
}

#[test]
fn tag_remove_from_task() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "Fix bug"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["tag", "add", "1", "mytag"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["tag", "remove", "1", "mytag"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("Removed tag 'mytag'"));
}

#[test]
fn list_tag_filter_shows_tagged_tasks() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "Tagged task"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "Untagged task"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["tag", "add", "1", "mytag"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["list", "--tag", "mytag"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("Tagged task"));
}

#[test]
fn add_task_with_tag_flag() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "Fix bug", "--tag", "urgent"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("#1"));

    moco(&home)
        .args(["tag", "list", "1"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("urgent"));
}

// ── moco note ────────────────────────────────────────────────────────────────

#[test]
fn note_add_with_title() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["note", "add", "My Note"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("N#1"))
        .stdout(contains("My Note"));
}

#[test]
fn note_list_shows_notes() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["note", "add", "Meeting notes"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["note", "list"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("N#1"))
        .stdout(contains("Meeting notes"));
}

#[test]
fn note_list_empty_when_no_notes() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["note", "list"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("No notes"));
}

#[test]
fn note_delete_with_yes_flag() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["note", "add", "Temp note"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["note", "delete", "-n", "1", "--yes"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("Deleted note N#1"));

    moco(&home)
        .args(["note", "list"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("No notes"));
}

#[test]
fn note_delete_cancelled_when_user_types_n() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["note", "add", "Keep this note"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["note", "delete", "-n", "1"])
        .write_stdin("n\n")
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("Cancelled"));

    moco(&home)
        .args(["note", "list"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("Keep this note"));
}

#[test]
fn note_edit_replaces_content() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["note", "add", "My Note", "Original content"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["note", "edit", "-n", "1", "New content"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("Updated note N#1"));
}

// ── moco export with notes ───────────────────────────────────────────────────

#[test]
fn export_includes_notes_section() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "myproj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "A task"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["note", "add", "Design decisions", "Some important notes here"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "export"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("Exported to"));

    // Read the exported file and check its contents.
    let export_path = workspace.path().join("myproj.md");
    let content = std::fs::read_to_string(&export_path).expect("exported file should exist");
    assert!(content.contains("## Notes"), "should have Notes section");
    assert!(content.contains("N#1"), "should have note ID");
    assert!(content.contains("Design decisions"), "should have note title");
}

// ── moco project move ─────────────────────────────────────────────────────────

#[test]
fn project_move_updates_registered_path() {
    let home = tmp();
    let old_dir = tmp();
    let new_dir = tmp();

    // Register project at old_dir.
    moco(&home)
        .args(["project", "init", "mover"])
        .current_dir(old_dir.path())
        .assert()
        .success();

    // Move it to new_dir (select via --project-path, new path via --new-path, skip confirm via --yes).
    moco(&home)
        .args([
            "project",
            "move",
            "--project-path",
            old_dir.path().to_str().unwrap(),
            "--new-path",
            new_dir.path().to_str().unwrap(),
            "--yes",
        ])
        .assert()
        .success()
        .stdout(contains("Moved project 'mover'"));

    // Tasks added from new_dir should work.
    moco(&home)
        .args(["add", "After the move"])
        .current_dir(new_dir.path())
        .assert()
        .success();

    moco(&home)
        .args(["list"])
        .current_dir(new_dir.path())
        .assert()
        .success()
        .stdout(contains("After the move"));
}

#[test]
fn project_move_fails_when_target_already_registered() {
    let home = tmp();
    let dir_a = tmp();
    let dir_b = tmp();

    // Register two projects.
    moco(&home)
        .args(["project", "init", "alpha"])
        .current_dir(dir_a.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "init", "beta"])
        .current_dir(dir_b.path())
        .assert()
        .success();

    // Attempt to move alpha to dir_b (already owned by beta).
    moco(&home)
        .args([
            "project",
            "move",
            "--project-path",
            dir_a.path().to_str().unwrap(),
            "--new-path",
            dir_b.path().to_str().unwrap(),
            "--yes",
        ])
        .assert()
        .failure()
        .stderr(contains("already registered"));
}

#[test]
fn project_move_noop_when_same_path() {
    let home = tmp();
    let dir = tmp();

    moco(&home)
        .args(["project", "init", "stable"])
        .current_dir(dir.path())
        .assert()
        .success();

    // "Move" to the same path — should succeed without error.
    moco(&home)
        .args([
            "project",
            "move",
            "--project-path",
            dir.path().to_str().unwrap(),
            "--new-path",
            dir.path().to_str().unwrap(),
            "--yes",
        ])
        .assert()
        .success()
        .stdout(contains("already registered"));
}

#[test]
fn project_move_fails_for_nonexistent_source_project() {
    let home = tmp();
    let registered = tmp();
    let unregistered = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(registered.path())
        .assert()
        .success();

    let new_dir = tmp();
    moco(&home)
        .args([
            "project",
            "move",
            "--project-path",
            unregistered.path().to_str().unwrap(),
            "--new-path",
            new_dir.path().to_str().unwrap(),
            "--yes",
        ])
        .assert()
        .failure()
        .stderr(contains("No project registered at"));
}

// ── moco project list ─────────────────────────────────────────────────────────

#[test]
fn project_list_shows_registered_projects() {
    let home = tmp();
    let dir = tmp();

    moco(&home)
        .args(["project", "init", "my-proj"])
        .current_dir(dir.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "list"])
        .assert()
        .success()
        .stdout(contains("my-proj"))
        .stdout(contains("Projects (1)"));
}

#[test]
fn project_list_shows_task_counts() {
    let home = tmp();
    let dir = tmp();

    moco(&home)
        .args(["project", "init", "tasky"])
        .current_dir(dir.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "A task"])
        .current_dir(dir.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "Another task"])
        .current_dir(dir.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "list"])
        .assert()
        .success()
        .stdout(contains("2 open"));
}

#[test]
fn project_list_shows_labels() {
    let home = tmp();
    let dir = tmp();

    moco(&home)
        .args(["project", "init", "labeled"])
        .current_dir(dir.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "label", "add", "rust"])
        .current_dir(dir.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "list"])
        .assert()
        .success()
        .stdout(contains("rust"));
}

#[test]
fn project_list_empty_when_no_projects() {
    let home = tmp();

    moco(&home)
        .args(["project", "list"])
        .assert()
        .success()
        .stdout(contains("No projects registered"));
}

#[test]
fn project_list_filters_by_label() {
    let home = tmp();
    let dir_a = tmp();
    let dir_b = tmp();

    moco(&home)
        .args(["project", "init", "alpha"])
        .current_dir(dir_a.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "init", "beta"])
        .current_dir(dir_b.path())
        .assert()
        .success();

    // Add label "rust" to alpha only.
    moco(&home)
        .args(["project", "label", "add", "rust"])
        .current_dir(dir_a.path())
        .assert()
        .success();

    // Filtering by "rust" should show alpha but not beta.
    moco(&home)
        .args(["project", "list", "--label", "rust"])
        .assert()
        .success()
        .stdout(contains("alpha"))
        .stdout(contains("Projects (1)"));

    // Filtering by an unknown label should report no matches.
    moco(&home)
        .args(["project", "list", "--label", "python"])
        .assert()
        .success()
        .stdout(contains("No projects with label 'python'"));
}

// ── project category ─────────────────────────────────────────────────────────

#[test]
fn project_category_add() {
    let home = tmp();

    moco(&home)
        .args(["project", "category", "add", "work"])
        .assert()
        .success()
        .stdout(contains("Added category"))
        .stdout(contains("work"));
}

#[test]
fn project_category_add_duplicate_fails() {
    let home = tmp();

    moco(&home)
        .args(["project", "category", "add", "work"])
        .assert()
        .success();

    moco(&home)
        .args(["project", "category", "add", "work"])
        .assert()
        .failure()
        .stderr(contains("already exists"));
}

#[test]
fn project_category_list() {
    let home = tmp();

    moco(&home)
        .args(["project", "category", "add", "work"])
        .assert()
        .success();

    moco(&home)
        .args(["project", "category", "add", "personal"])
        .assert()
        .success();

    moco(&home)
        .args(["project", "category", "list"])
        .assert()
        .success()
        .stdout(contains("work"))
        .stdout(contains("personal"));
}

#[test]
fn project_category_list_empty() {
    let home = tmp();

    moco(&home)
        .args(["project", "category", "list"])
        .assert()
        .success()
        .stdout(contains("No categories registered"));
}

#[test]
fn project_category_remove() {
    let home = tmp();

    moco(&home)
        .args(["project", "category", "add", "work"])
        .assert()
        .success();

    moco(&home)
        .args(["project", "category", "remove", "work"])
        .assert()
        .success()
        .stdout(contains("Removed category 'work'"));

    moco(&home)
        .args(["project", "category", "list"])
        .assert()
        .success()
        .stdout(contains("No categories registered"));
}

#[test]
fn project_category_remove_nonexistent_fails() {
    let home = tmp();

    moco(&home)
        .args(["project", "category", "remove", "ghost"])
        .assert()
        .failure()
        .stderr(contains("not found"));
}

#[test]
fn project_category_remove_with_assigned_projects_requires_force() {
    let home = tmp();
    let dir = tmp();

    moco(&home)
        .args(["project", "init", "myproj"])
        .current_dir(dir.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "category", "add", "work"])
        .assert()
        .success();

    moco(&home)
        .args(["project", "set-category", "myproj", "work"])
        .assert()
        .success();

    // Remove without --force should fail.
    moco(&home)
        .args(["project", "category", "remove", "work"])
        .assert()
        .failure()
        .stderr(contains("--force"));

    // Remove with --force should succeed and un-assign the project.
    moco(&home)
        .args(["project", "category", "remove", "work", "--force"])
        .assert()
        .success()
        .stdout(contains("Removed category 'work'"))
        .stdout(contains("un-assigned"));
}

#[test]
fn project_category_reorder() {
    let home = tmp();

    moco(&home)
        .args(["project", "category", "add", "alpha"])
        .assert()
        .success();

    moco(&home)
        .args(["project", "category", "add", "beta"])
        .assert()
        .success();

    moco(&home)
        .args(["project", "category", "add", "gamma"])
        .assert()
        .success();

    // Move gamma to position 1 (currently position 3).
    moco(&home)
        .args(["project", "category", "reorder", "gamma", "1"])
        .assert()
        .success()
        .stdout(contains("gamma"))
        .stdout(contains("position 1"));

    // List should now show gamma first.
    let out = moco(&home)
        .args(["project", "category", "list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = std::str::from_utf8(&out).unwrap();
    let gamma_pos = text.find("gamma").unwrap();
    let alpha_pos = text.find("alpha").unwrap();
    assert!(
        gamma_pos < alpha_pos,
        "gamma should appear before alpha after reorder"
    );
}

#[test]
fn project_category_reorder_invalid_position_fails() {
    let home = tmp();

    moco(&home)
        .args(["project", "category", "add", "work"])
        .assert()
        .success();

    // Position 0 is invalid.
    moco(&home)
        .args(["project", "category", "reorder", "work", "0"])
        .assert()
        .failure()
        .stderr(contains("out of range"));

    // Position beyond count is also invalid.
    moco(&home)
        .args(["project", "category", "reorder", "work", "99"])
        .assert()
        .failure()
        .stderr(contains("out of range"));
}

// ── project set-category ──────────────────────────────────────────────────────

#[test]
fn project_set_category_assigns_project() {
    let home = tmp();
    let dir = tmp();

    moco(&home)
        .args(["project", "init", "myproj"])
        .current_dir(dir.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "category", "add", "work"])
        .assert()
        .success();

    moco(&home)
        .args(["project", "set-category", "myproj", "work"])
        .assert()
        .success()
        .stdout(contains("myproj"))
        .stdout(contains("work"));
}

#[test]
fn project_set_category_invalid_category_fails() {
    let home = tmp();
    let dir = tmp();

    moco(&home)
        .args(["project", "init", "myproj"])
        .current_dir(dir.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "set-category", "myproj", "ghost"])
        .assert()
        .failure()
        .stderr(contains("does not exist"));
}

#[test]
fn project_set_category_nonexistent_project_fails() {
    let home = tmp();

    moco(&home)
        .args(["project", "category", "add", "work"])
        .assert()
        .success();

    moco(&home)
        .args(["project", "set-category", "ghost", "work"])
        .assert()
        .failure()
        .stderr(contains("No project named 'ghost'"));
}

#[test]
fn project_set_category_unset_removes_assignment() {
    let home = tmp();
    let dir = tmp();

    moco(&home)
        .args(["project", "init", "myproj"])
        .current_dir(dir.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "category", "add", "work"])
        .assert()
        .success();

    moco(&home)
        .args(["project", "set-category", "myproj", "work"])
        .assert()
        .success();

    moco(&home)
        .args(["project", "set-category", "myproj", "--unset"])
        .assert()
        .success()
        .stdout(contains("Removed category 'work'"));
}

// ── project list with categories ──────────────────────────────────────────────

#[test]
fn project_list_shows_category_headers() {
    let home = tmp();
    let dir_a = tmp();
    let dir_b = tmp();

    moco(&home)
        .args(["project", "init", "alpha"])
        .current_dir(dir_a.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "init", "beta"])
        .current_dir(dir_b.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "category", "add", "work"])
        .assert()
        .success();

    moco(&home)
        .args(["project", "set-category", "alpha", "work"])
        .assert()
        .success();

    // List should show "work" as header and "Uncategorized" for beta.
    moco(&home)
        .args(["project", "list"])
        .assert()
        .success()
        .stdout(contains("work"))
        .stdout(contains("Uncategorized"))
        .stdout(contains("alpha"))
        .stdout(contains("beta"));
}

#[test]
fn project_list_uncategorized_appears_last() {
    let home = tmp();
    let dir_a = tmp();
    let dir_b = tmp();

    moco(&home)
        .args(["project", "init", "alpha"])
        .current_dir(dir_a.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "init", "beta"])
        .current_dir(dir_b.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "category", "add", "work"])
        .assert()
        .success();

    moco(&home)
        .args(["project", "set-category", "alpha", "work"])
        .assert()
        .success();

    let out = moco(&home)
        .args(["project", "list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = std::str::from_utf8(&out).unwrap();
    let work_pos = text.find("work").unwrap();
    let uncat_pos = text.find("Uncategorized").unwrap();
    assert!(
        work_pos < uncat_pos,
        "categorised header should appear before Uncategorized"
    );
}

#[test]
fn project_list_filter_by_category() {
    let home = tmp();
    let dir_a = tmp();
    let dir_b = tmp();

    moco(&home)
        .args(["project", "init", "alpha"])
        .current_dir(dir_a.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "init", "beta"])
        .current_dir(dir_b.path())
        .assert()
        .success();

    moco(&home)
        .args(["project", "category", "add", "work"])
        .assert()
        .success();

    moco(&home)
        .args(["project", "set-category", "alpha", "work"])
        .assert()
        .success();

    moco(&home)
        .args(["project", "list", "--category", "work"])
        .assert()
        .success()
        .stdout(contains("alpha"))
        .stdout(predicates::str::contains("beta").not());
}

#[test]
fn project_list_filter_by_nonexistent_category_fails() {
    let home = tmp();

    moco(&home)
        .args(["project", "list", "--category", "ghost"])
        .assert()
        .failure()
        .stderr(contains("not found"));
}

// ── Subtask sub-indexing & completed/deferred task editing ───────────────────

#[test]
fn subtask_gets_sub_index_display() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "Parent"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "--sub", "1", "Child"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("S#1.1"));
}

#[test]
fn second_subtask_gets_incremented_sub_index() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "Parent"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "--sub", "1", "First child"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "--sub", "1", "Second child"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("S#1.2"));
}

#[test]
fn subtask_completion_shows_cs_format() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "Parent"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "--sub", "1", "Child"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["status", "1.1", "complete"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("CS#1.1"));
}

#[test]
fn status_with_subtask_ref_sets_progress() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "Parent"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "--sub", "1", "Child"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["status", "1.1", "50"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("S#1.1"));
}

#[test]
fn edit_completed_task_with_c_ref() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "My task"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["status", "1", "complete"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["edit", "-t", "C1", "Updated content", "--replace"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("C#1"));
}

#[test]
fn status_reopen_completed_task() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "My task"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["status", "1", "complete"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["status", "C1", "open"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("re-opened"));
}

#[test]
fn status_defer_completed_task() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "My task"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["status", "1", "complete"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["status", "C1", "defer"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("deferred"));
}

#[test]
fn subtasks_do_not_affect_top_level_index_sequence() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "First"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("#1"));

    moco(&home)
        .args(["add", "--sub", "1", "Child of first"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("S#1.1"));

    // Second top-level task should be #2, not #3.
    moco(&home)
        .args(["add", "Second"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("#2"));
}

// ── moco remove ──────────────────────────────────────────────────────────────

#[test]
fn remove_open_task_by_index() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "Task to delete"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["remove", "--yes", "1"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("Deleted task"));

    moco(&home)
        .args(["list"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("No tasks"));
}

#[test]
fn remove_task_reindexes_remaining() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
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
        .args(["add", "Third"])
        .current_dir(workspace.path())
        .assert()
        .success();

    // Remove the middle task.
    moco(&home)
        .args(["remove", "--yes", "2"])
        .current_dir(workspace.path())
        .assert()
        .success();

    // "Third" should now be #2.
    moco(&home)
        .args(["status", "2", "complete"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("C#1"));
}

#[test]
fn remove_subtask_by_ref() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "Parent"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "--sub", "1", "Child"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["remove", "--yes", "1.1"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("S#1.1"));
}

#[test]
fn remove_completed_task_by_c_ref() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "Task"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["status", "1", "complete"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["remove", "--yes", "C1"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("C#1"));
}

#[test]
fn remove_nonexistent_task_fails() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["remove", "--yes", "99"])
        .current_dir(workspace.path())
        .assert()
        .failure();
}

#[test]
fn remove_parent_also_removes_subtasks() {
    let home = tmp();
    let workspace = tmp();

    moco(&home)
        .args(["project", "init", "proj"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "Parent"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "--sub", "1", "Child 1"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["add", "--sub", "1", "Child 2"])
        .current_dir(workspace.path())
        .assert()
        .success();

    moco(&home)
        .args(["remove", "--yes", "1"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("2 subtask(s)"));

    moco(&home)
        .args(["list"])
        .current_dir(workspace.path())
        .assert()
        .success()
        .stdout(contains("No tasks"));
}
