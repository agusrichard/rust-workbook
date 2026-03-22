use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::models::todo::{Todo};

pub type TodoDb = Arc<Mutex<HashMap<u64, Todo>>>;

pub fn new_todo_db() -> TodoDb {
    Arc::new(Mutex::new(HashMap::new()))
}
