use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use crate::errors::AppError;
use crate::models::todo::{CreateTodo, Todo, UpdateTodo};
use crate::repository::todo::TodoRepository;

struct TodoRow {
    id: i64,
    title: String,
    description: String,
    completed: bool,
    created_at: DateTime<Utc>
}

impl From<TodoRow> for Todo {
    fn from(row: TodoRow) -> Self {
        Todo {
            id: row.id as u64,
            title: row.title,
            description: row.description,
            completed: row.completed,
            created_at: row.created_at,
        }
    }
}

pub struct PostgresTodoRepository {
    pool: PgPool
}

impl PostgresTodoRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TodoRepository for PostgresTodoRepository {
    async fn create(&self, body: CreateTodo) -> Result<Todo, AppError> {
        let row = sqlx::query_as!(
            TodoRow,
            "INSERT INTO todos (title, description) VALUES ($1, $2) RETURNING *",
            body.title,
            body.description.unwrap_or_default()
        )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        Ok(row.into())
    }

    async fn get_all(&self) -> Result<Vec<Todo>, AppError> {
        let rows = sqlx::query_as!(TodoRow, "SELECT * FROM todos ORDER BY id ASC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn get_todo(&self, todo_id: u64) -> Result<Todo, AppError> {
        let row = sqlx::query_as!(
            TodoRow,
            "SELECT * FROM todos WHERE id = $1",
            todo_id as i64
        )
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        row.map(Into::into)
            .ok_or_else(|| AppError::not_found("Todo", todo_id))
    }

    async fn update(&self, todo_id: u64, body: UpdateTodo) -> Result<Todo, AppError> {
        let row = sqlx::query_as!(
            TodoRow,
            "UPDATE todos
             SET
               title       = COALESCE($1, title),
               description = COALESCE($2, description),
               completed   = COALESCE($3, completed)
             WHERE id = $4
             RETURNING *",
            body.title,
            body.description,
            body.completed,
            todo_id as i64
        )
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        row.map(Into::into)
            .ok_or_else(|| AppError::not_found("Todo", todo_id))
    }

    async fn delete(&self, todo_id: u64) -> Result<Todo, AppError> {
        let row = sqlx::query_as!(
            TodoRow,
            "DELETE FROM todos WHERE id = $1 RETURNING *",
            todo_id as i64
        )
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        row.map(Into::into)
            .ok_or_else(|| AppError::not_found("Todo", todo_id))
    }}