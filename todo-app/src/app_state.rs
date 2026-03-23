use crate::repository::todo::TodoRepository;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub todo_repo: Arc<dyn TodoRepository>,
}
