use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A named grouping for projects. Categories are globally registered and
/// appear as section headers in `moco project list` and the project browser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: Uuid,
    pub name: String,
    /// Display position: lower values appear first. Reassigned as 1, 2, … on reorder.
    pub order: u32,
    pub created_at: DateTime<Utc>,
}

impl Category {
    pub fn new(name: impl Into<String>, order: u32) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            order,
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_category_has_unique_ids() {
        let c1 = Category::new("work", 1);
        let c2 = Category::new("personal", 2);
        assert_ne!(c1.id, c2.id);
    }

    #[test]
    fn new_category_stores_name_and_order() {
        let c = Category::new("work", 3);
        assert_eq!(c.name, "work");
        assert_eq!(c.order, 3);
    }
}
