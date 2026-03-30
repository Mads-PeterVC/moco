use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use tui_textarea::TextArea;

use crate::theme::Theme;
use crate::tui::keys;

/// The focusable fields in the add/edit form.
/// Add new variants here to extend the form without changing control flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormField {
    Title,
    Body,
}

impl FormField {
    fn next(self) -> Self {
        match self {
            FormField::Title => FormField::Body,
            FormField::Body => FormField::Title,
        }
    }

    fn prev(self) -> Self {
        // With two fields, prev == next; extend when more fields are added.
        self.next()
    }
}

/// Outcome of handling a key event in the form.
pub enum FormOutcome {
    /// The user submitted the form.
    Submitted,
    /// The user cancelled.
    Cancelled,
    /// Event was consumed; continue rendering.
    Continue,
}

/// A reusable two-field task form (Title + Body).
pub struct TaskForm<'a> {
    pub focused: FormField,
    title_area: TextArea<'a>,
    body_area: TextArea<'a>,
    theme: Theme,
}

impl<'a> TaskForm<'a> {
    /// Create a blank form using the given theme.
    pub fn new(theme: Theme) -> Self {
        Self::with_values("", "", theme)
    }

    /// Create a form pre-populated with existing values (for editing).
    pub fn with_values(title: &str, body: &str, theme: Theme) -> Self {
        let mut title_area = TextArea::default();
        if !title.is_empty() {
            title_area.insert_str(title);
        }

        let body_lines: Vec<&str> = if body.is_empty() {
            vec![""]
        } else {
            body.lines().collect()
        };
        let body_area = TextArea::new(body_lines.iter().map(|s| s.to_string()).collect());

        let mut form = Self {
            focused: FormField::Title,
            title_area,
            body_area,
            theme,
        };
        form.apply_styles();
        form
    }

    fn apply_styles(&mut self) {
        let active = Style::default().fg(self.theme.accent);
        let inactive = Style::default().fg(self.theme.selection_bg);

        let (title_style, body_style) = match self.focused {
            FormField::Title => (active, inactive),
            FormField::Body => (inactive, active),
        };

        self.title_area.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(title_style)
                .title(Span::styled(" Title ", title_style.add_modifier(Modifier::BOLD))),
        );
        self.body_area.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(body_style)
                .title(Span::styled(
                    " Body (Markdown) ",
                    body_style.add_modifier(Modifier::BOLD),
                )),
        );
    }

    /// Handle a key event. Returns the outcome to the caller.
    pub fn handle_key(&mut self, key: KeyEvent) -> FormOutcome {
        let code = key.code;
        let mods = key.modifiers;

        if (code, mods) == keys::SUBMIT {
            return FormOutcome::Submitted;
        }
        if (code, mods) == keys::CANCEL {
            return FormOutcome::Cancelled;
        }
        if (code, mods) == keys::NEXT_FIELD {
            self.focused = self.focused.next();
            self.apply_styles();
            return FormOutcome::Continue;
        }
        if (code, mods) == keys::PREV_FIELD {
            self.focused = self.focused.prev();
            self.apply_styles();
            return FormOutcome::Continue;
        }

        // Forward remaining events to the focused textarea.
        match self.focused {
            FormField::Title => {
                // Prevent newlines in the title field.
                if code == KeyCode::Enter {
                    return FormOutcome::Continue;
                }
                self.title_area.input(key);
            }
            FormField::Body => {
                self.body_area.input(key);
            }
        }

        FormOutcome::Continue
    }

    /// Render the form into the given area.
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title: single line + borders
                Constraint::Min(5),    // Body: expandable
                Constraint::Length(1), // Help line
            ])
            .split(area);

        frame.render_widget(&self.title_area, chunks[0]);
        frame.render_widget(&self.body_area, chunks[1]);

        let help = Paragraph::new(Line::from(vec![
            Span::styled("Ctrl+S", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" submit  "),
            Span::styled("Tab", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" next field  "),
            Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" cancel"),
        ]))
        .style(Style::default().fg(self.theme.selection_bg));
        frame.render_widget(help, chunks[2]);
    }

    /// Extract (title, body) strings from the current form contents.
    pub fn values(&self) -> (String, String) {
        let title = self.title_area.lines().join("");
        let body = self.body_area.lines().join("\n");
        (title, body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use crate::theme::{Theme, ThemeConfig};

    fn default_theme() -> Theme {
        Theme::resolve(&ThemeConfig::default())
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    #[test]
    fn new_form_focuses_title() {
        let form = TaskForm::new(default_theme());
        assert_eq!(form.focused, FormField::Title);
    }

    #[test]
    fn tab_moves_focus_to_body() {
        let mut form = TaskForm::new(default_theme());
        form.handle_key(key(KeyCode::Tab));
        assert_eq!(form.focused, FormField::Body);
    }

    #[test]
    fn tab_wraps_back_to_title() {
        let mut form = TaskForm::new(default_theme());
        form.handle_key(key(KeyCode::Tab));
        form.handle_key(key(KeyCode::Tab));
        assert_eq!(form.focused, FormField::Title);
    }

    #[test]
    fn ctrl_s_returns_submitted() {
        let mut form = TaskForm::new(default_theme());
        let outcome = form.handle_key(ctrl('s'));
        assert!(matches!(outcome, FormOutcome::Submitted));
    }

    #[test]
    fn esc_returns_cancelled() {
        let mut form = TaskForm::new(default_theme());
        let outcome = form.handle_key(key(KeyCode::Esc));
        assert!(matches!(outcome, FormOutcome::Cancelled));
    }

    #[test]
    fn enter_in_title_does_not_insert_newline() {
        let mut form = TaskForm::new(default_theme());
        form.handle_key(key(KeyCode::Char('h')));
        form.handle_key(key(KeyCode::Char('i')));
        form.handle_key(key(KeyCode::Enter)); // should be ignored
        let (title, _) = form.values();
        assert!(!title.contains('\n'));
    }

    #[test]
    fn with_values_prepopulates_fields() {
        let form = TaskForm::with_values("My Title", "Line 1\nLine 2", default_theme());
        let (title, body) = form.values();
        assert_eq!(title, "My Title");
        assert!(body.contains("Line 1"));
        assert!(body.contains("Line 2"));
    }

    #[test]
    fn values_empty_on_new_form() {
        let form = TaskForm::new(default_theme());
        let (title, body) = form.values();
        assert!(title.is_empty());
        assert!(body.trim().is_empty());
    }
}
