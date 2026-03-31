use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::cli::project::list::format_last_active;
use crate::models::Project;
use crate::theme::Theme;
use crate::tui::keys;
use crate::tui::scroll_list::ScrollList;

/// Outcome of handling a key event in the project browser.
pub enum ProjectBrowserOutcome {
    /// User confirmed a selection; contains the index into the flattened project slice.
    Selected(usize),
    /// User cancelled.
    Cancelled,
    /// Event consumed; continue rendering.
    Continue,
}

/// Scrollable project list browser with optional category headers.
pub struct ProjectBrowser {
    /// Navigates only over selectable project items (headers are skipped).
    scroll: ScrollList,
    list_state: ListState,
}

impl ProjectBrowser {
    /// Create a browser for `project_count` selectable items (sum across all groups).
    pub fn new(project_count: usize) -> Self {
        let scroll = ScrollList::new(project_count);
        let mut list_state = ListState::default();
        list_state.select(scroll.selected());
        Self { scroll, list_state }
    }

    /// Handle a key event. Returns the outcome to the caller.
    pub fn handle_key(&mut self, key: KeyEvent) -> ProjectBrowserOutcome {
        let code = key.code;
        let mods = key.modifiers;

        if (code, mods) == keys::CANCEL {
            return ProjectBrowserOutcome::Cancelled;
        }
        if (code, mods) == keys::SELECT {
            if let Some(i) = self.scroll.selected() {
                return ProjectBrowserOutcome::Selected(i);
            }
        }
        if (code, mods) == keys::UP {
            self.scroll.move_up();
            self.list_state.select(self.scroll.selected());
        }
        if (code, mods) == keys::DOWN {
            self.scroll.move_down();
            self.list_state.select(self.scroll.selected());
        }

        ProjectBrowserOutcome::Continue
    }

    /// Currently highlighted selectable index (into the flattened project list).
    #[allow(dead_code)]
    pub fn selected(&self) -> Option<usize> {
        self.scroll.selected()
    }

    /// Compute the visual (display) index of selectable item `selectable_idx` within
    /// the grouped list (accounting for header rows).
    fn visual_index_of(groups: &[(String, Vec<(Project, usize, Option<String>)>)], selectable_idx: usize) -> usize {
        let mut visual = 0;
        let mut remaining = selectable_idx;
        for (_, projects) in groups {
            visual += 1; // header row
            if remaining < projects.len() {
                visual += remaining;
                return visual;
            }
            remaining -= projects.len();
            visual += projects.len();
        }
        visual
    }

    /// Render the browser into the given area.
    ///
    /// `groups` is a slice of `(category_name, [(project, open_task_count, compact_git)])` pairs.
    /// Projects without a category should be passed under `"Uncategorized"`.
    pub fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        groups: &[(String, Vec<(Project, usize, Option<String>)>)],
        theme: &Theme,
    ) {
        // Sync the visual list state with the current selectable selection.
        if let Some(sel) = self.scroll.selected() {
            let visual = Self::visual_index_of(groups, sel);
            self.list_state.select(Some(visual));
        } else {
            self.list_state.select(None);
        }

        let highlight_sym_width = 2usize; // "▶ "
        let name_col = 20usize;
        let path_col = 46usize;
        let fixed = highlight_sym_width + name_col + path_col;

        let mut items: Vec<ListItem> = Vec::new();

        for (header, projects) in groups {
            // Non-selectable category header row.
            items.push(ListItem::new(Line::from(vec![
                Span::styled(
                    format!("── {header} ──"),
                    Style::default()
                        .fg(theme.label)
                        .add_modifier(Modifier::BOLD),
                ),
            ])));

            for (p, open_count, compact_git) in projects {
                let path = p.path.display().to_string();
                let path_preview = if path.len() > 45 {
                    format!("…{}", &path[path.len() - 45..])
                } else {
                    path
                };

                // Line 1: bold name + accent path + right-justified date.
                let date = format_last_active(&p.last_active);
                let date_str = format!("[{date}]");
                let remaining = (area.width as usize).saturating_sub(fixed + 6); // +2 for block borders, +4 margin
                let spacer = " ".repeat(remaining.saturating_sub(date_str.len()));
                let line1 = Line::from(vec![
                    Span::styled(
                        format!("{:<20}", p.name),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{:<46}", path_preview),
                        Style::default().fg(theme.accent),
                    ),
                    Span::raw(spacer),
                    Span::raw(date_str),
                ]);

                // Line 2: labels, open task count, and compact git info (if available).
                let mut line2: Vec<Span> = vec![Span::raw("  ")];
                for label in &p.labels {
                    line2.push(Span::styled(
                        format!("[{}] ", label),
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                let count_text = match open_count {
                    0 => "no open tasks".to_string(),
                    1 => "1 open task".to_string(),
                    n => format!("{n} open tasks"),
                };
                line2.push(Span::styled(
                    count_text,
                    Style::default().fg(theme.open),
                ));
                if let Some(git_str) = compact_git {
                    line2.push(Span::raw("   "));
                    line2.push(Span::styled(
                        git_str.clone(),
                        Style::default().fg(theme.label),
                    ));
                }

                items.push(ListItem::new(vec![line1, Line::from(line2)]));
            }
        }

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(
                        " Select Project ",
                        Style::default().add_modifier(Modifier::BOLD),
                    )),
            )
            .highlight_style(
                Style::default()
                    .bg(theme.selection_bg)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, area, &mut self.list_state);

        if area.height > 4 {
            let help_area = Rect {
                x: area.x,
                y: area.y + area.height - 1,
                width: area.width,
                height: 1,
            };
            let help = Paragraph::new(Line::from(vec![
                Span::styled("↑↓", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" navigate  "),
                Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" select  "),
                Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" cancel"),
            ]))
            .style(Style::default().fg(theme.accent));
            frame.render_widget(help, help_area);
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn new_browser_selects_first_item() {
        let b = ProjectBrowser::new(3);
        assert_eq!(b.selected(), Some(0));
    }

    #[test]
    fn new_browser_with_zero_projects_has_no_selection() {
        let b = ProjectBrowser::new(0);
        assert_eq!(b.selected(), None);
    }

    #[test]
    fn down_moves_selection() {
        let mut b = ProjectBrowser::new(3);
        b.handle_key(key(KeyCode::Down));
        assert_eq!(b.selected(), Some(1));
    }

    #[test]
    fn down_wraps_at_end() {
        let mut b = ProjectBrowser::new(3);
        b.handle_key(key(KeyCode::Down));
        b.handle_key(key(KeyCode::Down));
        b.handle_key(key(KeyCode::Down));
        assert_eq!(b.selected(), Some(0));
    }

    #[test]
    fn up_wraps_at_start() {
        let mut b = ProjectBrowser::new(3);
        b.handle_key(key(KeyCode::Up));
        assert_eq!(b.selected(), Some(2));
    }

    #[test]
    fn enter_returns_selected_index() {
        let mut b = ProjectBrowser::new(3);
        b.handle_key(key(KeyCode::Down));
        let outcome = b.handle_key(key(KeyCode::Enter));
        assert!(matches!(outcome, ProjectBrowserOutcome::Selected(1)));
    }

    #[test]
    fn esc_returns_cancelled() {
        let mut b = ProjectBrowser::new(3);
        let outcome = b.handle_key(key(KeyCode::Esc));
        assert!(matches!(outcome, ProjectBrowserOutcome::Cancelled));
    }

    #[test]
    fn visual_index_accounts_for_headers() {
        use std::path::PathBuf;
        // Group A: 2 projects, Group B: 1 project
        // Visual layout: [A header] [A proj0] [A proj1] [B header] [B proj0]
        // Selectable:         -          0         1          -         2
        let dummy_proj = || {
            (
                Project::new("p", PathBuf::from("/tmp/p")),
                0usize,
                None::<String>,
            )
        };
        let groups: Vec<(String, Vec<(Project, usize, Option<String>)>)> = vec![
            ("A".to_string(), vec![dummy_proj(), dummy_proj()]),
            ("B".to_string(), vec![dummy_proj()]),
        ];
        assert_eq!(ProjectBrowser::visual_index_of(&groups, 0), 1); // A proj0
        assert_eq!(ProjectBrowser::visual_index_of(&groups, 1), 2); // A proj1
        assert_eq!(ProjectBrowser::visual_index_of(&groups, 2), 4); // B proj0
    }
}
