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
