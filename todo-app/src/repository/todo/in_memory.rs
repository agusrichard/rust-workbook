use std::collections::HashMap;
use std::sync::{Arc, Mutex};
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
    fn create(&self, body: CreateTodo) -> Todo {
        todo!()
    }
    fn get_all(&self) -> Vec<Todo> {
        todo!()
    }
    fn get_todo(&self, todo_id: u64) -> Todo {
        todo!()
    }
    fn update(&self, body: UpdateTodo) -> Todo {
        todo!()
    }
    fn delete(&self, todo_id: u64) -> Todo {
        todo!()
    }
}