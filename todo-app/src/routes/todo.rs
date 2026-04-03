use crate::app_state::AppState;
use crate::errors::AppError;
use crate::models::todo::{CreateTodo, UpdateTodo};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{
    Json, Router,
    routing::{delete, get, post, put},
};

async fn create_todo(
    State(state): State<AppState>,
    Json(body): Json<CreateTodo>,
) -> Result<impl IntoResponse, AppError> {
    let todo = state.todo_repo.create(body).await?;
    Ok((StatusCode::CREATED, Json(todo)))
}

async fn get_todos(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let todos = state.todo_repo.get_all().await?;
    Ok(Json(todos))
}

async fn get_todo(
    State(state): State<AppState>,
    Path(todo_id): Path<u64>,
) -> Result<impl IntoResponse, AppError> {
    let todo = state.todo_repo.get_todo(todo_id).await?;
    Ok(Json(todo))
}

async fn update_todo(
    State(state): State<AppState>,
    Path(todo_id): Path<u64>,
    Json(body): Json<UpdateTodo>,
) -> Result<impl IntoResponse, AppError> {
    let todo = state.todo_repo.update(todo_id, body).await?;
    Ok(Json(todo))
}

async fn delete_todo(
    State(state): State<AppState>,
    Path(todo_id): Path<u64>,
) -> Result<impl IntoResponse, AppError> {
    state.todo_repo.delete(todo_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_todos))
        .route("/", post(create_todo))
        .route("/{todo_id}", get(get_todo))
        .route("/{todo_id}", put(update_todo))
        .route("/{todo_id}", delete(delete_todo))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::AppState;
    use crate::errors::AppError;
    use crate::models::todo::Todo;
    use crate::repository::todo::MockTodoRepository;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use chrono::Utc;
    use std::sync::Arc;
    use tower::ServiceExt;

    fn sample_todo() -> Todo {
        Todo {
            id: 1,
            title: "Test todo".to_string(),
            description: "Test description".to_string(),
            completed: false,
            created_at: Utc::now(),
        }
    }

    fn app_with(mock: MockTodoRepository) -> Router {
        router().with_state(AppState {
            todo_repo: Arc::new(mock),
        })
    }

    fn json_request(method: &str, uri: &str, body: serde_json::Value) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap()
    }

    // --- GET / ---

    #[tokio::test]
    async fn get_todos_returns_200_with_empty_list() {
        let mut mock = MockTodoRepository::new();
        mock.expect_get_all().once().returning(|| Ok(vec![]));

        let res = app_with(mock)
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn get_todos_returns_all_todos() {
        let mut mock = MockTodoRepository::new();
        mock.expect_get_all()
            .once()
            .returning(|| Ok(vec![sample_todo()]));

        let res = app_with(mock)
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }

    // --- GET /{id} ---

    #[tokio::test]
    async fn get_todo_returns_200_when_found() {
        let mut mock = MockTodoRepository::new();
        mock.expect_get_todo()
            .withf(|id| *id == 1)
            .once()
            .returning(|_| Ok(sample_todo()));

        let res = app_with(mock)
            .oneshot(Request::builder().uri("/1").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn get_todo_returns_404_when_not_found() {
        let mut mock = MockTodoRepository::new();
        mock.expect_get_todo()
            .once()
            .returning(|id| Err(AppError::not_found("Todo", id)));

        let res = app_with(mock)
            .oneshot(Request::builder().uri("/99").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    // --- POST / ---

    #[tokio::test]
    async fn create_todo_returns_201_with_created_todo() {
        let mut mock = MockTodoRepository::new();
        mock.expect_create()
            .once()
            .returning(|_| Ok(sample_todo()));

        let res = app_with(mock)
            .oneshot(json_request(
                "POST",
                "/",
                serde_json::json!({"title": "Test todo", "description": "Test description"}),
            ))
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn create_todo_accepts_missing_description() {
        let mut mock = MockTodoRepository::new();
        mock.expect_create()
            .once()
            .returning(|_| Ok(sample_todo()));

        let res = app_with(mock)
            .oneshot(json_request("POST", "/", serde_json::json!({"title": "No desc"})))
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::CREATED);
    }

    // --- PUT /{id} ---

    #[tokio::test]
    async fn update_todo_returns_200_when_found() {
        let mut mock = MockTodoRepository::new();
        mock.expect_update()
            .withf(|id, _| *id == 1)
            .once()
            .returning(|_, _| Ok(sample_todo()));

        let res = app_with(mock)
            .oneshot(json_request(
                "PUT",
                "/1",
                serde_json::json!({"title": "Updated", "completed": true}),
            ))
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn update_todo_returns_404_when_not_found() {
        let mut mock = MockTodoRepository::new();
        mock.expect_update()
            .once()
            .returning(|id, _| Err(AppError::not_found("Todo", id)));

        let res = app_with(mock)
            .oneshot(json_request("PUT", "/99", serde_json::json!({"title": "X"})))
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    // --- DELETE /{id} ---

    #[tokio::test]
    async fn delete_todo_returns_204_when_deleted() {
        let mut mock = MockTodoRepository::new();
        mock.expect_delete()
            .withf(|id| *id == 1)
            .once()
            .returning(|_| Ok(sample_todo()));

        let res = app_with(mock)
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn delete_todo_returns_404_when_not_found() {
        let mut mock = MockTodoRepository::new();
        mock.expect_delete()
            .once()
            .returning(|id| Err(AppError::not_found("Todo", id)));

        let res = app_with(mock)
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/99")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }
}
