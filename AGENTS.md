# moco

# Project Description

This project is intended to be a CLI todo/task manager. 

The tool should use a simple, preferably single-file database to store all tasks. 
Task specification should support Markdown formatting and projects should be exportable to 
`.md`-files. When invoking any of the task commands the tool should look for the "nearest" workspace 
that has been registered, e.g. if invoked from `$HOME/example/my_project/` and that is a registered project 
then the task is added to that project. If no project exists, then the tool also keeps a global task list. 

Upon the first invocation the tool should create the database at `$HOME/.moco/` which is also 
where any configuration etc should be saved. 

## Intended usage examples

Examples of usage are shown below. This may involve over time. 

### Project initialization

```sh
moco init <project-name> # Intialize a project for the current directory/workspace.
```

Initializes a project with the current working directory registered as the root. This should lead to 
a new database "section" for items related to that directory. 

If a matching database entry already exists, then the tool should warn and not do anything unless 
an extra flag is provided.

### Adding a task

```sh
moco add "Fix CI errors."  # Add a task to the current project.
```

If this is only non-completed task for a project it gets ID `#1`, a second task would get `#2`. New tasks are assigned the status `open`. 

### Adding a task with TUI

```sh
moco add # Opens a TUI to write a task 
```

Some tasks might require more editing than what is ergonomic to write as a single-command, the tool should have a TUI that allows 
for writing and editing tasks.

### Adding a subtask.

Tasks can have subtasks, to add a subtask to task `#1`

```sh
moco add --sub 1 <Subtask description>
```

Or with the TUI 

```sh
moco add --sub 1 # Directly open TUI for a new subtask to task 1.
```
or 
```sh
moco add # TUI should let one simple navigate to create a task or subtask.
```

### Edit tasks

Tasks should editabled through both commands and TUI 

```sh
moco edit # Opens a TUI to edit task(s).
```

```sh
moco edit -t 1 <New content> --append # Append new content for task #1.
moco edit -t 1 <Replacement content> --replace # Replace/overwrite content.
```

### Changed task status.

```sh
moco status <task_id> <task_status>
```

Fundamentally tasks are either `open`, `complete`, or `defer`. Tasks that are open can have progress as a percentage between 0 and 99. 

```sh
moco status 1 50
```
This would set the progress of task `#1` to 50% leaving it as `open`. 

The next two are equivalent
```sh
moco status 1 100
moco status 1 complete
```
Once a task is completed it moves from being indexed by just a number to being `C#1` if it was the first completed task. 
The current task indexing of the project is then adjusted taking into account the completed task. 

For tasks that were considered but for one reason or another work was stopped the intention is to mark 
them deferred

```sh
moco status 1 defer
```
The task retains its progress but is moved to a deferred category with a seperate index `D#1` for the first defered task and 
so on. Again this should trigger reindexing of the tasks for the project. 

### List tasks

```sh
moco list # List all tasks in the current project. 
```

This should show a list of task, their status and a visual indication of their progress. 

```sh
moco list --global # or -g: To list global tasks
```

### Open a project

```sh 
moco open # TUI menu to select project
```
Picked project is opened in the configured editor/program.

## Configuration

`moco` will keep a configuration file in `$HOME/.moco/config.toml` that keeps all configuration options. This includes the command used to open projects. 

## Programming Language

The tool should be written in modern Rust. While efficiency is important it is not the 
end all be all, all of these things are important: 

- Readability: The code should be easy to understand. This means keeping strict separation of concerns, 
not overengineering and preferring a simpler implementation even if a more complex one offers marginally better
 performance.
 - Abstractions: It is important to make use of Rust's abstractions when they contribute towards the other criteria. 
 - Testability: Everything needs to be tested, no feature is completed without having proper test coverage. This includes both 
 unit and integration tests. The expectation is 99%+ test coverage at all times. 
 - Expendability: The code should always be considered a work in progress and should be written such that new features can be 
 added without too having to invent new entry points or rewire the code base.
 - Avoiding dependency lock-in: For certain aspects, mostly the database, the code should avoid being written in ways that lock the codebase into a specific dependency - for example for database it is preferably if the codebase from the beginning uses an abstraction layer to interface with databases such that the backend/dependency can be easily swapped.
 - Performance: The tool is expected to be lightweight and feel snappy, but finding microsecond improvements is not important. 

 ## Agent Behavior 

- Agents should ask as many questions as needed to construct a clear plan, it is better to ask one too many than one too little. 
- Agents need to adhere to the criteria stated above. 
- Agent can, if they find reason to, suggest features or changes. This could for example be other commands than those listed in the usage examples. 
- Agents should ensure consistency, both in code and in user experience - this means for example notifying if a notable discrepancy in CLI strcuture is found or suggested in a prompt. 


