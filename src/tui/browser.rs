use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::models::{Task, TaskStatus};
use crate::theme::Theme;
use crate::tui::keys;
use crate::tui::scroll_list::ScrollList;

/// Outcome of handling a key event in the browser.
pub enum BrowserOutcome {
    /// User confirmed a selection; contains the index into the task slice.
    Selected(usize),
    /// User cancelled.
    Cancelled,
    /// Event consumed; continue rendering.
    Continue,
}

/// Scrollable task list browser.
pub struct TaskBrowser {
    scroll: ScrollList,
    list_state: ListState,
}

impl TaskBrowser {
    pub fn new(task_count: usize) -> Self {
        let scroll = ScrollList::new(task_count);
        let mut list_state = ListState::default();
        list_state.select(scroll.selected());
        Self { scroll, list_state }
    }

    /// Handle a key event. Returns the outcome to the caller.
    pub fn handle_key(&mut self, key: KeyEvent) -> BrowserOutcome {
        let code = key.code;
        let mods = key.modifiers;

        if (code, mods) == keys::CANCEL {
            return BrowserOutcome::Cancelled;
        }
        if (code, mods) == keys::SELECT {
            if let Some(i) = self.scroll.selected() {
                return BrowserOutcome::Selected(i);
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

        BrowserOutcome::Continue
    }

    /// Currently highlighted index.
    #[allow(dead_code)]
    pub fn selected(&self) -> Option<usize> {
        self.scroll.selected()
    }

    /// Render the browser into the given area.
    pub fn render(&mut self, frame: &mut Frame, area: Rect, tasks: &[Task], theme: &Theme) {
        let items: Vec<ListItem> = tasks
            .iter()
            .map(|t| {
                let id = t.display_id();
                let bar = progress_bar(t.progress, 10);
                let preview = t.content.lines().next().unwrap_or("").trim();
                let preview = if preview.len() > 40 {
                    format!("{}…", &preview[..40])
                } else {
                    preview.to_string()
                };
                let status_color = match t.status {
                    TaskStatus::Open => theme.open,
                    TaskStatus::Complete => theme.complete,
                    TaskStatus::Defer => theme.defer,
                };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:>5}  ", id),
                        Style::default().fg(status_color).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(format!("{}  ", bar)),
                    Span::raw(preview),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(
                        " Select Task ",
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

        // Help line below the list — only if there's room.
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
            .style(Style::default().fg(theme.selection_bg));
            frame.render_widget(help, help_area);
        }
    }
}

pub(crate) fn progress_bar(progress: u8, width: usize) -> String {
    let filled = ((progress as usize) * width) / 100;
    let empty = width - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
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
        let b = TaskBrowser::new(3);
        assert_eq!(b.selected(), Some(0));
    }

    #[test]
    fn new_browser_with_zero_tasks_has_no_selection() {
        let b = TaskBrowser::new(0);
        assert_eq!(b.selected(), None);
    }

    #[test]
    fn down_moves_selection() {
        let mut b = TaskBrowser::new(3);
        b.handle_key(key(KeyCode::Down));
        assert_eq!(b.selected(), Some(1));
    }

    #[test]
    fn down_wraps_at_end() {
        let mut b = TaskBrowser::new(3);
        b.handle_key(key(KeyCode::Down));
        b.handle_key(key(KeyCode::Down));
        b.handle_key(key(KeyCode::Down));
        assert_eq!(b.selected(), Some(0));
    }

    #[test]
    fn up_wraps_at_start() {
        let mut b = TaskBrowser::new(3);
        b.handle_key(key(KeyCode::Up));
        assert_eq!(b.selected(), Some(2));
    }

    #[test]
    fn enter_returns_selected_index() {
        let mut b = TaskBrowser::new(3);
        b.handle_key(key(KeyCode::Down));
        let outcome = b.handle_key(key(KeyCode::Enter));
        assert!(matches!(outcome, BrowserOutcome::Selected(1)));
    }

    #[test]
    fn esc_returns_cancelled() {
        let mut b = TaskBrowser::new(3);
        let outcome = b.handle_key(key(KeyCode::Esc));
        assert!(matches!(outcome, BrowserOutcome::Cancelled));
    }
}

