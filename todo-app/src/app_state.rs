use std::sync::Arc;
use crate::repository::todo::TodoRepository;

#[derive(Clone)]
pub struct AppState {
    pub todo_repo: Arc<dyn TodoRepository>
}