use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::Utc;
use crate::errors::AppError;
use crate::models::todo::{CreateTodo, Todo, UpdateTodo};
use crate::repository::todo::TodoRepository;

pub struct InMemoryTodoRepository {
    db: Arc<Mutex<HashMap<u64, Todo>>>
}

impl InMemoryTodoRepository {
    pub fn new() -> Self {
        Self { db: Arc::new(Mutex::new(HashMap::new())) }
    }
}

impl TodoRepository for InMemoryTodoRepository {
    fn create(&self, body: CreateTodo) -> Result<Todo, AppError> {
        let mut store = self.db.lock().unwrap();
        let id = store.keys().max().copied().unwrap_or(0) + 1;
        let todo = Todo {
            id,
            title: body.title,
            description: body.description.unwrap_or_default(),
            completed: false,
            created_at: Utc::now()
        };
        store.insert(id, todo.clone());
        Ok(todo)
    }
    fn get_all(&self) -> Result<Vec<Todo>, AppError> {
        let store = self.db.lock().unwrap();
        Ok(store.values().cloned().collect())
    }
    fn get_todo(&self, todo_id: u64) -> Result<Todo, AppError> {
        let store = self.db.lock().unwrap();
        store.get(&todo_id).cloned().ok_or_else(|| AppError::NotFound(format!("Todo {} not found", todo_id)))
    }
    fn update(&self, todo_id: u64, body: UpdateTodo) -> Result<Todo, AppError> {
        let mut store = self.db.lock().unwrap();
        store.get_mut(&todo_id).map(|todo| {
            if let Some(title) = body.title { todo.title =
                title; }
            if let Some(description) = body.description {
                todo.description = description; }
            if let Some(completed) = body.completed {
                todo.completed = completed; }
            todo.clone()
        }).ok_or_else(|| {AppError::NotFound(format!("Todo {} not found", todo_id))})
    }
    fn delete(&self, todo_id: u64) -> Result<Todo, AppError> {
        let mut store = self.db.lock().unwrap();
        store.remove(&todo_id).ok_or_else(|| AppError::NotFound(format!("Todo {} not found", todo_id)))
    }
}