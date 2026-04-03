use chrono::Utc;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use crate::errors::AppError;
use crate::models::todo::{CreateTodo, Todo, UpdateTodo};
use crate::repository::todo::TodoRepository;


pub struct InMemoryTodoRepository {
    db: Arc<Mutex<HashMap<u64, Todo>>>,
    next_id: Arc<AtomicU64>,
}

impl InMemoryTodoRepository {
    pub fn new() -> Self {
        Self {
            db: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, HashMap<u64, Todo>>, AppError> {
        self.db
            .lock()
            .map_err(|_| AppError::InternalError("Storage lock poisoned".to_string()))
    }
}

#[async_trait]
impl TodoRepository for InMemoryTodoRepository {
    async fn create(&self, body: CreateTodo) -> Result<Todo, AppError> {
        let mut store = self.lock()?;
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let todo = Todo {
            id,
            title: body.title,
            description: body.description.unwrap_or_default(),
            completed: false,
            created_at: Utc::now(),
        };
        store.insert(id, todo.clone());
        Ok(todo)
    }

    async fn get_all(&self) -> Result<Vec<Todo>, AppError> {
        let store = self.lock()?;
        Ok(store.values().cloned().collect())
    }

    async fn get_todo(&self, todo_id: u64) -> Result<Todo, AppError> {
        let store = self.lock()?;
        store
            .get(&todo_id)
            .cloned()
            .ok_or_else(|| AppError::not_found("Todo", todo_id))
    }

    async fn update(&self, todo_id: u64, body: UpdateTodo) -> Result<Todo, AppError> {
        let mut store = self.lock()?;
        store
            .get_mut(&todo_id)
            .map(|todo| {
                if let Some(title) = body.title {
                    todo.title = title;
                }
                if let Some(description) = body.description {
                    todo.description = description;
                }
                if let Some(completed) = body.completed {
                    todo.completed = completed;
                }
                todo.clone()
            })
            .ok_or_else(|| AppError::not_found("Todo", todo_id))
    }

    async fn delete(&self, todo_id: u64) -> Result<Todo, AppError> {
        let mut store = self.lock()?;
        store
            .remove(&todo_id)
            .ok_or_else(|| AppError::not_found("Todo", todo_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::todo::{CreateTodo, UpdateTodo};

    fn make_repo() -> InMemoryTodoRepository {
        InMemoryTodoRepository::new()
    }

    fn create_body(title: &str, description: Option<&str>) -> CreateTodo {
        CreateTodo {
            title: title.to_string(),
            description: description.map(|s| s.to_string()),
        }
    }

    // --- create ---

    #[tokio::test]
    async fn create_sets_fields_correctly() {
        let repo = make_repo();
        let todo = repo
            .create(create_body("Buy milk", Some("2 litres")))
            .await
            .unwrap();

        assert_eq!(todo.title, "Buy milk");
        assert_eq!(todo.description, "2 litres");
        assert!(!todo.completed);
    }

    #[tokio::test]
    async fn create_uses_empty_string_when_description_is_none() {
        let repo = make_repo();
        let todo = repo.create(create_body("No desc", None)).await.unwrap();
        assert_eq!(todo.description, "");
    }

    #[tokio::test]
    async fn create_auto_increments_ids() {
        let repo = make_repo();
        let first = repo.create(create_body("First", None)).await.unwrap();
        let second = repo.create(create_body("Second", None)).await.unwrap();
        assert!(second.id > first.id);
    }

    // --- get_all ---

    #[tokio::test]
    async fn get_all_returns_empty_initially() {
        let repo = make_repo();
        let todos = repo.get_all().await.unwrap();
        assert!(todos.is_empty());
    }

    #[tokio::test]
    async fn get_all_returns_all_created_todos() {
        let repo = make_repo();
        repo.create(create_body("A", None)).await.unwrap();
        repo.create(create_body("B", None)).await.unwrap();
        let todos = repo.get_all().await.unwrap();
        assert_eq!(todos.len(), 2);
    }

    // --- get_todo ---

    #[tokio::test]
    async fn get_todo_returns_correct_todo() {
        let repo = make_repo();
        let created = repo.create(create_body("Find me", None)).await.unwrap();
        let found = repo.get_todo(created.id).await.unwrap();
        assert_eq!(found.id, created.id);
        assert_eq!(found.title, "Find me");
    }

    #[tokio::test]
    async fn get_todo_returns_not_found_for_missing_id() {
        let repo = make_repo();
        let result = repo.get_todo(999).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    // --- update ---

    #[tokio::test]
    async fn update_applies_partial_fields() {
        let repo = make_repo();
        let todo = repo.create(create_body("Old title", Some("Old desc"))).await.unwrap();

        let updated = repo
            .update(
                todo.id,
                UpdateTodo {
                    title: Some("New title".to_string()),
                    description: None,
                    completed: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.title, "New title");
        // description untouched
        assert_eq!(updated.description, "Old desc");
        assert!(!updated.completed);
    }

    #[tokio::test]
    async fn update_marks_todo_completed() {
        let repo = make_repo();
        let todo = repo.create(create_body("Task", None)).await.unwrap();

        let updated = repo
            .update(
                todo.id,
                UpdateTodo {
                    title: None,
                    description: None,
                    completed: Some(true),
                },
            )
            .await
            .unwrap();

        assert!(updated.completed);
    }

    #[tokio::test]
    async fn update_returns_not_found_for_missing_id() {
        let repo = make_repo();
        let result = repo
            .update(
                42,
                UpdateTodo {
                    title: Some("X".to_string()),
                    description: None,
                    completed: None,
                },
            )
            .await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    // --- delete ---

    #[tokio::test]
    async fn delete_removes_the_todo() {
        let repo = make_repo();
        let todo = repo.create(create_body("Delete me", None)).await.unwrap();
        let deleted = repo.delete(todo.id).await.unwrap();

        assert_eq!(deleted.id, todo.id);
        // Confirm it's gone
        assert!(matches!(repo.get_todo(todo.id).await, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn delete_returns_not_found_for_missing_id() {
        let repo = make_repo();
        let result = repo.delete(999).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }
}
