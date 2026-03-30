use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::models::Project;
use crate::theme::Theme;
use crate::tui::keys;
use crate::tui::scroll_list::ScrollList;

/// Outcome of handling a key event in the project browser.
pub enum ProjectBrowserOutcome {
    /// User confirmed a selection; contains the index into the project slice.
    Selected(usize),
    /// User cancelled.
    Cancelled,
    /// Event consumed; continue rendering.
    Continue,
}

/// Scrollable project list browser.
pub struct ProjectBrowser {
    scroll: ScrollList,
    list_state: ListState,
}

impl ProjectBrowser {
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

    /// Currently highlighted index.
    #[allow(dead_code)]
    pub fn selected(&self) -> Option<usize> {
        self.scroll.selected()
    }

    /// Render the browser into the given area.
    ///
    /// `projects` is a slice of `(project, open_task_count)` pairs.
    pub fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        projects: &[(Project, usize)],
        theme: &Theme,
    ) {
        let items: Vec<ListItem> = projects
            .iter()
            .map(|(p, open_count)| {
                let path = p.path.display().to_string();
                let path_preview = if path.len() > 45 {
                    format!("…{}", &path[path.len() - 45..])
                } else {
                    path
                };

                // Line 1: bold name + dimmed path.
                let line1 = Line::from(vec![
                    Span::styled(
                        format!("{:<20}", p.name),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        path_preview,
                        Style::default().fg(theme.accent),
                    ),
                ]);

                // Line 2: labels (if any) then open task count.
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

                ListItem::new(vec![line1, Line::from(line2)])
            })
            .collect();

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
}
