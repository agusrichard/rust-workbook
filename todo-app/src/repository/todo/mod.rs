use async_trait::async_trait;
use crate::errors::AppError;
use crate::models::todo::{CreateTodo, Todo, UpdateTodo};

pub mod in_memory;
pub mod postgres;

#[async_trait]
pub trait TodoRepository: Send + Sync {
    async fn create(&self, body: CreateTodo) -> Result<Todo, AppError>;
    async fn get_all(&self) -> Result<Vec<Todo>, AppError>;
    async fn get_todo(&self, todo_id: u64) -> Result<Todo, AppError>;
    async fn update(&self, todo_id: u64, body: UpdateTodo) -> Result<Todo, AppError>;
    async fn delete(&self, todo_id: u64) -> Result<Todo, AppError>;
}
