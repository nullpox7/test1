use std::sync::{Arc, Mutex};

/// A single task. Rendered by `templates/tasks/row.html`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Task {
    pub id: u64,
    pub title: String,
    pub done: bool,
}

/// In-memory store. Swap for a database behind the same methods;
/// handlers and templates do not need to change.
#[derive(Clone, Default)]
pub struct AppState {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Default)]
struct Inner {
    next_id: u64,
    tasks: Vec<Task>,
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_sample_data() -> Self {
        let state = Self::new();
        state.create("Read the htmx docs");
        state.create("Write an Askama template");
        let id = state.create("Ship it");
        state.toggle(id);
        state
    }

    /// Insert a task and return its id. Title is trimmed; callers validate emptiness.
    pub fn create(&self, title: &str) -> u64 {
        let mut inner = self.inner.lock().expect("state lock");
        inner.next_id += 1;
        let id = inner.next_id;
        inner.tasks.push(Task {
            id,
            title: title.trim().to_string(),
            done: false,
        });
        id
    }

    pub fn get(&self, id: u64) -> Option<Task> {
        self.inner
            .lock()
            .expect("state lock")
            .tasks
            .iter()
            .find(|t| t.id == id)
            .cloned()
    }

    /// Flip `done`. Returns the updated task, or None if it does not exist.
    pub fn toggle(&self, id: u64) -> Option<Task> {
        let mut inner = self.inner.lock().expect("state lock");
        let task = inner.tasks.iter_mut().find(|t| t.id == id)?;
        task.done = !task.done;
        Some(task.clone())
    }

    /// Remove a task. Returns true if something was removed.
    pub fn delete(&self, id: u64) -> bool {
        let mut inner = self.inner.lock().expect("state lock");
        let before = inner.tasks.len();
        inner.tasks.retain(|t| t.id != id);
        inner.tasks.len() != before
    }

    /// Tasks whose title contains `query` (case-insensitive). Empty query returns all.
    pub fn search(&self, query: &str) -> Vec<Task> {
        let q = query.trim().to_lowercase();
        self.inner
            .lock()
            .expect("state lock")
            .tasks
            .iter()
            .filter(|t| q.is_empty() || t.title.to_lowercase().contains(&q))
            .cloned()
            .collect()
    }

    /// Number of tasks not yet done.
    pub fn remaining(&self) -> usize {
        self.inner
            .lock()
            .expect("state lock")
            .tasks
            .iter()
            .filter(|t| !t.done)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_toggle_delete_roundtrip() {
        let s = AppState::new();
        let id = s.create("  hello  ");
        assert_eq!(s.get(id).unwrap().title, "hello");
        assert_eq!(s.remaining(), 1);
        assert!(s.toggle(id).unwrap().done);
        assert_eq!(s.remaining(), 0);
        assert!(s.delete(id));
        assert!(!s.delete(id));
        assert!(s.get(id).is_none());
    }

    #[test]
    fn search_is_case_insensitive_and_empty_matches_all() {
        let s = AppState::new();
        s.create("Buy Milk");
        s.create("Walk dog");
        assert_eq!(s.search("milk").len(), 1);
        assert_eq!(s.search("").len(), 2);
        assert_eq!(s.search("zzz").len(), 0);
    }
}
