/// Pure navigation state for a scrollable list — no rendering logic.
///
/// Both [`TaskBrowser`](super::browser::TaskBrowser) and
/// [`ProjectBrowser`](super::project_browser::ProjectBrowser) use this to avoid
/// duplicating wrap-around index management.
pub struct ScrollList {
    selected: Option<usize>,
    count: usize,
}

impl ScrollList {
    pub fn new(count: usize) -> Self {
        Self {
            selected: if count > 0 { Some(0) } else { None },
            count,
        }
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Move selection up by one, wrapping from the first item to the last.
    pub fn move_up(&mut self) {
        if self.count == 0 {
            return;
        }
        let i = self.selected.unwrap_or(0);
        self.selected = Some(if i == 0 { self.count - 1 } else { i - 1 });
    }

    /// Move selection down by one, wrapping from the last item to the first.
    pub fn move_down(&mut self) {
        if self.count == 0 {
            return;
        }
        let i = self.selected.unwrap_or(0);
        self.selected = Some((i + 1) % self.count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_with_items_selects_first() {
        let sl = ScrollList::new(3);
        assert_eq!(sl.selected(), Some(0));
    }

    #[test]
    fn new_with_zero_has_no_selection() {
        let sl = ScrollList::new(0);
        assert_eq!(sl.selected(), None);
    }

    #[test]
    fn move_down_advances() {
        let mut sl = ScrollList::new(3);
        sl.move_down();
        assert_eq!(sl.selected(), Some(1));
    }

    #[test]
    fn move_down_wraps() {
        let mut sl = ScrollList::new(3);
        sl.move_down();
        sl.move_down();
        sl.move_down();
        assert_eq!(sl.selected(), Some(0));
    }

    #[test]
    fn move_up_wraps_to_last() {
        let mut sl = ScrollList::new(3);
        sl.move_up();
        assert_eq!(sl.selected(), Some(2));
    }

    #[test]
    fn move_on_empty_is_no_op() {
        let mut sl = ScrollList::new(0);
        sl.move_up();
        sl.move_down();
        assert_eq!(sl.selected(), None);
    }
}
