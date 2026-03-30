use crossterm::event::{KeyCode, KeyModifiers};

/// Submit the current form / confirm an action.
pub const SUBMIT: (KeyCode, KeyModifiers) = (KeyCode::Char('s'), KeyModifiers::CONTROL);

/// Cancel / close without saving.
pub const CANCEL: (KeyCode, KeyModifiers) = (KeyCode::Esc, KeyModifiers::NONE);

/// Move to the next field or list item.
pub const NEXT_FIELD: (KeyCode, KeyModifiers) = (KeyCode::Tab, KeyModifiers::NONE);

/// Move to the previous field.
pub const PREV_FIELD: (KeyCode, KeyModifiers) = (KeyCode::BackTab, KeyModifiers::SHIFT);

/// Confirm selection in a browser / list.
pub const SELECT: (KeyCode, KeyModifiers) = (KeyCode::Enter, KeyModifiers::NONE);

/// Move the list cursor up.
pub const UP: (KeyCode, KeyModifiers) = (KeyCode::Up, KeyModifiers::NONE);

/// Move the list cursor down.
pub const DOWN: (KeyCode, KeyModifiers) = (KeyCode::Down, KeyModifiers::NONE);
