use crate::models::todo::{CreateTodo, Todo, UpdateTodo};

pub mod in_memory;

pub trait TodoRepository: Send + Sync {
    fn create(&self, body: CreateTodo) -> Todo;
    fn get_all(&self) -> Vec<Todo>;
    fn get_todo(&self, todo_id: u64) -> Todo;
    fn update(&self, body: UpdateTodo) -> Todo;
    fn delete(&self, todo_id: u64) -> Todo;
}