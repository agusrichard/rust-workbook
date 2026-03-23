use crate::errors::AppError;
use crate::models::todo::{CreateTodo, Todo, UpdateTodo};

pub mod in_memory;

pub trait TodoRepository: Send + Sync {
    fn create(&self, body: CreateTodo) -> Result<Todo, AppError>;
    fn get_all(&self) -> Result<Vec<Todo>, AppError>;
    fn get_todo(&self, todo_id: u64) -> Result<Todo, AppError>;
    fn update(&self, todo_id: u64, body: UpdateTodo) -> Result<Todo, AppError>;
    fn delete(&self, todo_id: u64) -> Result<Todo, AppError>;
}