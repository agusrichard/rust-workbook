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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::AppError;
    use crate::models::todo::{CreateTodo, UpdateTodo};
    use crate::repository::todo::TodoRepository;
    use sqlx::PgPool;

    fn create_body(title: &str, description: Option<&str>) -> CreateTodo {
        CreateTodo {
            title: title.to_string(),
            description: description.map(|s| s.to_string()),
        }
    }

    // --- create ---

    #[sqlx::test(migrations = "./migrations")]
    async fn create_sets_fields_correctly(pool: PgPool) {
        let repo = PostgresTodoRepository::new(pool);
        let todo = repo
            .create(create_body("Buy milk", Some("2 litres")))
            .await
            .unwrap();

        assert_eq!(todo.title, "Buy milk");
        assert_eq!(todo.description, "2 litres");
        assert!(!todo.completed);
        assert!(todo.id > 0);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn create_uses_empty_string_when_description_is_none(pool: PgPool) {
        let repo = PostgresTodoRepository::new(pool);
        let todo = repo.create(create_body("No desc", None)).await.unwrap();
        assert_eq!(todo.description, "");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn create_ids_are_unique(pool: PgPool) {
        let repo = PostgresTodoRepository::new(pool);
        let first = repo.create(create_body("First", None)).await.unwrap();
        let second = repo.create(create_body("Second", None)).await.unwrap();
        assert_ne!(first.id, second.id);
    }

    // --- get_all ---

    #[sqlx::test(migrations = "./migrations")]
    async fn get_all_returns_empty_initially(pool: PgPool) {
        let repo = PostgresTodoRepository::new(pool);
        let todos = repo.get_all().await.unwrap();
        assert!(todos.is_empty());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn get_all_returns_all_created_todos(pool: PgPool) {
        let repo = PostgresTodoRepository::new(pool);
        repo.create(create_body("A", None)).await.unwrap();
        repo.create(create_body("B", None)).await.unwrap();
        let todos = repo.get_all().await.unwrap();
        assert_eq!(todos.len(), 2);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn get_all_returns_todos_ordered_by_id(pool: PgPool) {
        let repo = PostgresTodoRepository::new(pool);
        repo.create(create_body("First", None)).await.unwrap();
        repo.create(create_body("Second", None)).await.unwrap();
        let todos = repo.get_all().await.unwrap();
        assert!(todos[0].id < todos[1].id);
    }

    // --- get_todo ---

    #[sqlx::test(migrations = "./migrations")]
    async fn get_todo_returns_correct_todo(pool: PgPool) {
        let repo = PostgresTodoRepository::new(pool);
        let created = repo.create(create_body("Find me", None)).await.unwrap();
        let found = repo.get_todo(created.id).await.unwrap();
        assert_eq!(found.id, created.id);
        assert_eq!(found.title, "Find me");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn get_todo_returns_not_found_for_missing_id(pool: PgPool) {
        let repo = PostgresTodoRepository::new(pool);
        let result = repo.get_todo(99999).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    // --- update ---

    #[sqlx::test(migrations = "./migrations")]
    async fn update_applies_partial_fields(pool: PgPool) {
        let repo = PostgresTodoRepository::new(pool);
        let todo = repo
            .create(create_body("Old title", Some("Old desc")))
            .await
            .unwrap();

        let updated = repo
            .update(
                todo.id,
                UpdateTodo {
                    title: Some("New title".to_string()),
                    description: None,
                    completed: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.title, "New title");
        assert_eq!(updated.description, "Old desc"); // untouched
        assert!(!updated.completed);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn update_marks_todo_completed(pool: PgPool) {
        let repo = PostgresTodoRepository::new(pool);
        let todo = repo.create(create_body("Task", None)).await.unwrap();

        let updated = repo
            .update(
                todo.id,
                UpdateTodo {
                    title: None,
                    description: None,
                    completed: Some(true),
                },
            )
            .await
            .unwrap();

        assert!(updated.completed);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn update_returns_not_found_for_missing_id(pool: PgPool) {
        let repo = PostgresTodoRepository::new(pool);
        let result = repo
            .update(
                99999,
                UpdateTodo {
                    title: Some("X".to_string()),
                    description: None,
                    completed: None,
                },
            )
            .await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    // --- delete ---

    #[sqlx::test(migrations = "./migrations")]
    async fn delete_removes_the_todo(pool: PgPool) {
        let repo = PostgresTodoRepository::new(pool);
        let todo = repo.create(create_body("Delete me", None)).await.unwrap();
        let deleted = repo.delete(todo.id).await.unwrap();

        assert_eq!(deleted.id, todo.id);
        assert!(matches!(
            repo.get_todo(todo.id).await,
            Err(AppError::NotFound(_))
        ));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn delete_returns_not_found_for_missing_id(pool: PgPool) {
        let repo = PostgresTodoRepository::new(pool);
        let result = repo.delete(99999).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }
}