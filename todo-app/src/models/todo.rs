use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub completed: bool,
    pub created_at: DateTime<Utc>
}

#[derive(Deserialize)]
pub struct CreateTodo {
    pub title: String,
    pub description: Option<String>
}

#[derive(Deserialize)]
pub struct UpdateTodo {
    pub title: Option<String>,
    pub description: Option<String>,
    pub completed: Option<bool>
}