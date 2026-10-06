use std::time::Duration;

use async_graphql::{Request, Response};
use serde_json::{Value, json};
use sqlx::sqlite::SqlitePoolOptions;
use tokio_stream::StreamExt;

use crate::auth::{CurrentUser, JwtSecret};
use crate::{AppSchema, build_schema};

// ---- Helpers ----

// A fresh in-memory database per test, with the migrations (and seed data) applied.
// Each connection to sqlite::memory: is its own database, so the pool is limited to one.
// Two users exist from the start: "tester" (id 1) and "other" (id 2).
async fn test_schema() -> AppSchema {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();
    sqlx::query("INSERT INTO users (username, password_hash) VALUES ('tester', 'x'), ('other', 'x')")
        .execute(&pool)
        .await
        .unwrap();
    build_schema(pool, JwtSecret("test-secret".into()))
}

// A request that looks logged in: attach CurrentUser, as the HTTP handler does.
fn as_user(query: &str) -> Request {
    Request::new(query).data(CurrentUser { id: 1 })
}

// The same, as the second user.
fn as_other_user(query: &str) -> Request {
    Request::new(query).data(CurrentUser { id: 2 })
}

// The response data as JSON. Fails the test if the response has errors.
fn data(response: Response) -> Value {
    assert!(response.errors.is_empty(), "unexpected errors: {:?}", response.errors);
    response.data.into_json().unwrap()
}

// The first error message. Fails the test if the response has no errors.
fn error(response: Response) -> String {
    response.errors.first().expect("expected an error").message.clone()
}

// ---- Queries ----

#[tokio::test]
async fn query_returns_book_with_author() {
    let schema = test_schema().await;
    let response = schema.execute("{ book(id: 1) { id title year author { id name } } }").await;

    assert_eq!(
        data(response),
        json!({ "book": {
            "id": "1",
            "title": "The Hobbit",
            "year": 1937,
            "author": { "id": "1", "name": "J.R.R. Tolkien" }
        } })
    );
}

#[tokio::test]
async fn query_returns_author_with_books() {
    let schema = test_schema().await;
    let response = schema.execute("{ author(id: 1) { name books { title } } }").await;

    assert_eq!(
        data(response),
        json!({ "author": {
            "name": "J.R.R. Tolkien",
            "books": [
                { "title": "The Hobbit" },
                { "title": "The Fellowship of the Ring" },
                { "title": "The Silmarillion" }
            ]
        } })
    );
}

#[tokio::test]
async fn author_without_books_has_empty_list() {
    let schema = test_schema().await;
    let response = schema.execute("{ author(id: 100) { name books { title } } }").await;

    assert_eq!(
        data(response),
        json!({ "author": { "name": "Seamus Heaney", "books": [] } })
    );
}

#[tokio::test]
async fn unknown_id_returns_null() {
    let schema = test_schema().await;
    let response = schema.execute("{ book(id: 999) { title } author(id: 999) { name } }").await;

    assert_eq!(data(response), json!({ "book": null, "author": null }));
}

#[tokio::test]
async fn author_id_is_not_exposed_on_book() {
    let schema = test_schema().await;
    let response = schema.execute("{ book(id: 1) { authorId } }").await;

    assert!(error(response).contains("Unknown field"));
}

// ---- Pagination ----

#[tokio::test]
async fn pagination_uses_defaults() {
    let schema = test_schema().await;
    let response = schema.execute("{ books { items { id } totalCount hasNextPage } }").await;
    let result = data(response);

    assert_eq!(result["books"]["items"].as_array().unwrap().len(), 10);
    assert_eq!(result["books"]["items"][0], json!({ "id": "1" }));
    assert_eq!(result["books"]["totalCount"], 100);
    assert_eq!(result["books"]["hasNextPage"], true);
}

#[tokio::test]
async fn pagination_returns_middle_page() {
    let schema = test_schema().await;
    let response = schema
        .execute("{ books(limit: 2, offset: 2) { items { id title } totalCount hasNextPage } }")
        .await;

    assert_eq!(
        data(response),
        json!({ "books": {
            "items": [
                { "id": "3", "title": "Don Quixote" },
                { "id": "4", "title": "Robinson Crusoe" }
            ],
            "totalCount": 100,
            "hasNextPage": true
        } })
    );
}

#[tokio::test]
async fn pagination_returns_last_page() {
    let schema = test_schema().await;
    let response = schema
        .execute("{ books(limit: 2, offset: 98) { items { id } totalCount hasNextPage } }")
        .await;

    assert_eq!(
        data(response),
        json!({ "books": {
            "items": [{ "id": "99" }, { "id": "100" }],
            "totalCount": 100,
            "hasNextPage": false
        } })
    );
}

#[tokio::test]
async fn pagination_past_the_end_is_empty() {
    let schema = test_schema().await;
    let response = schema
        .execute("{ books(offset: 500) { items { id } totalCount hasNextPage } }")
        .await;

    assert_eq!(
        data(response),
        json!({ "books": { "items": [], "totalCount": 100, "hasNextPage": false } })
    );
}

#[tokio::test]
async fn pagination_works_for_authors() {
    let schema = test_schema().await;
    let response = schema
        .execute("{ authors(limit: 2, offset: 0) { items { id name } totalCount hasNextPage } }")
        .await;

    assert_eq!(
        data(response),
        json!({ "authors": {
            "items": [
                { "id": "1", "name": "J.R.R. Tolkien" },
                { "id": "2", "name": "Dan Brown" }
            ],
            "totalCount": 100,
            "hasNextPage": true
        } })
    );
}

#[tokio::test]
async fn pagination_rejects_invalid_arguments() {
    let schema = test_schema().await;

    for query in [
        "{ books(limit: 0) { totalCount } }",
        "{ books(limit: 51) { totalCount } }",
        "{ books(limit: -5) { totalCount } }",
        "{ authors(limit: 0) { totalCount } }",
    ] {
        assert_eq!(
            error(schema.execute(query).await),
            "Limit should be between 1 and 50",
            "{query}"
        );
    }

    for query in [
        "{ books(offset: -1) { totalCount } }",
        "{ authors(offset: -1) { totalCount } }",
    ] {
        assert_eq!(error(schema.execute(query).await), "Offset cannot be negative", "{query}");
    }
}

// ---- Mutations ----

#[tokio::test]
async fn create_author_trims_name() {
    let schema = test_schema().await;
    let response = schema
        .execute(as_user(
            r#"mutation { createAuthor(input: { name: "  New Author  " }) { id name books { id } } }"#,
        ))
        .await;

    assert_eq!(
        data(response),
        json!({ "createAuthor": { "id": "101", "name": "New Author", "books": [] } })
    );
}

#[tokio::test]
async fn create_book_returns_book_with_author() {
    let schema = test_schema().await;
    let response = schema
        .execute(as_user(
            r#"mutation {
                createBook(input: { title: " Test Book ", year: 2020, authorId: 2 }) {
                    id title year author { name }
                }
            }"#,
        ))
        .await;

    assert_eq!(
        data(response),
        json!({ "createBook": {
            "id": "101",
            "title": "Test Book",
            "year": 2020,
            "author": { "name": "Dan Brown" }
        } })
    );
}

#[tokio::test]
async fn create_book_without_year_has_null_year() {
    let schema = test_schema().await;
    let response = schema
        .execute(as_user(
            r#"mutation { createBook(input: { title: "No Year", authorId: 1 }) { title year } }"#,
        ))
        .await;

    assert_eq!(
        data(response),
        json!({ "createBook": { "title": "No Year", "year": null } })
    );
}

#[tokio::test]
async fn update_book_changes_only_given_fields() {
    let schema = test_schema().await;
    let response = schema
        .execute(as_user(
            "mutation { updateBook(id: 1, input: { year: 1938 }) { title year author { name } } }",
        ))
        .await;

    assert_eq!(
        data(response),
        json!({ "updateBook": {
            "title": "The Hobbit",
            "year": 1938,
            "author": { "name": "J.R.R. Tolkien" }
        } })
    );
}

#[tokio::test]
async fn update_book_can_move_it_to_another_author() {
    let schema = test_schema().await;
    let response = schema
        .execute(as_user(
            "mutation { updateBook(id: 1, input: { authorId: 2 }) { title author { name } } }",
        ))
        .await;

    assert_eq!(
        data(response),
        json!({ "updateBook": { "title": "The Hobbit", "author": { "name": "Dan Brown" } } })
    );
}

#[tokio::test]
async fn update_author_changes_name() {
    let schema = test_schema().await;
    let response = schema
        .execute(as_user(
            r#"mutation { updateAuthor(id: 1, input: { name: "Tolkien" }) { id name } }"#,
        ))
        .await;

    assert_eq!(
        data(response),
        json!({ "updateAuthor": { "id": "1", "name": "Tolkien" } })
    );
}

#[tokio::test]
async fn update_author_with_empty_input_keeps_name() {
    let schema = test_schema().await;
    let response = schema
        .execute(as_user("mutation { updateAuthor(id: 1, input: {}) { name } }"))
        .await;

    assert_eq!(
        data(response),
        json!({ "updateAuthor": { "name": "J.R.R. Tolkien" } })
    );
}

#[tokio::test]
async fn delete_book_removes_it() {
    let schema = test_schema().await;

    let deleted = schema.execute(as_user("mutation { deleteBook(id: 1) }")).await;
    assert_eq!(data(deleted), json!({ "deleteBook": true }));

    let after = schema.execute("{ book(id: 1) { title } books { totalCount } }").await;
    assert_eq!(
        data(after),
        json!({ "book": null, "books": { "totalCount": 99 } })
    );
}

#[tokio::test]
async fn delete_author_without_books_succeeds() {
    let schema = test_schema().await;

    let deleted = schema.execute(as_user("mutation { deleteAuthor(id: 100) }")).await;
    assert_eq!(data(deleted), json!({ "deleteAuthor": true }));

    let after = schema.execute("{ author(id: 100) { name } }").await;
    assert_eq!(data(after), json!({ "author": null }));
}

// ---- Validation ----

#[tokio::test]
async fn delete_author_with_books_is_rejected() {
    let schema = test_schema().await;

    let response = schema.execute(as_user("mutation { deleteAuthor(id: 1) }")).await;
    assert_eq!(error(response), "Author 1 has books and cannot be deleted");

    let after = schema.execute("{ author(id: 1) { name } }").await;
    assert_eq!(data(after), json!({ "author": { "name": "J.R.R. Tolkien" } }));
}

#[tokio::test]
async fn unknown_id_on_write_is_an_error() {
    let schema = test_schema().await;

    let cases = [
        (r#"mutation { updateAuthor(id: 999, input: { name: "x" }) { id } }"#, "Author 999 does not exist"),
        ("mutation { deleteAuthor(id: 999) }", "Author 999 does not exist"),
        (r#"mutation { updateBook(id: 999, input: { title: "x" }) { id } }"#, "Book 999 does not exist"),
        ("mutation { deleteBook(id: 999) }", "Book 999 does not exist"),
    ];
    for (query, expected) in cases {
        assert_eq!(error(schema.execute(as_user(query)).await), expected, "{query}");
    }
}

#[tokio::test]
async fn blank_text_is_rejected() {
    let schema = test_schema().await;

    let cases = [
        (r#"mutation { createAuthor(input: { name: "   " }) { id } }"#, "Name cannot be blank"),
        (r#"mutation { updateAuthor(id: 1, input: { name: "" }) { id } }"#, "Name cannot be blank"),
        (r#"mutation { createBook(input: { title: " ", authorId: 1 }) { id } }"#, "Title cannot be blank"),
        (r#"mutation { updateBook(id: 1, input: { title: "" }) { id } }"#, "Title cannot be blank"),
    ];
    for (query, expected) in cases {
        assert_eq!(error(schema.execute(as_user(query)).await), expected, "{query}");
    }

    // Nothing was created or changed.
    let after = schema
        .execute("{ authors { totalCount } books { totalCount } book(id: 1) { title } author(id: 1) { name } }")
        .await;
    assert_eq!(
        data(after),
        json!({
            "authors": { "totalCount": 100 },
            "books": { "totalCount": 100 },
            "book": { "title": "The Hobbit" },
            "author": { "name": "J.R.R. Tolkien" }
        })
    );
}

#[tokio::test]
async fn book_needs_an_existing_author() {
    let schema = test_schema().await;

    let cases = [
        r#"mutation { createBook(input: { title: "x", authorId: 999 }) { id } }"#,
        "mutation { updateBook(id: 1, input: { authorId: 999 }) { id } }",
    ];
    for query in cases {
        assert_eq!(
            error(schema.execute(as_user(query)).await),
            "Author 999 does not exist",
            "{query}"
        );
    }
}

#[tokio::test]
async fn invalid_id_is_an_error() {
    let schema = test_schema().await;
    let response = schema.execute(r#"{ book(id: "abc") { title } }"#).await;

    assert!(!response.errors.is_empty());
}

// ---- Authentication ----

#[tokio::test]
async fn mutations_need_login() {
    let schema = test_schema().await;

    let mutations = [
        r#"mutation { createAuthor(input: { name: "x" }) { id } }"#,
        r#"mutation { updateAuthor(id: 1, input: { name: "x" }) { id } }"#,
        "mutation { deleteAuthor(id: 100) }",
        r#"mutation { createBook(input: { title: "x", authorId: 1 }) { id } }"#,
        r#"mutation { updateBook(id: 1, input: { title: "x" }) { id } }"#,
        "mutation { deleteBook(id: 1) }",
    ];
    for query in mutations {
        assert_eq!(error(schema.execute(query).await), "Not authenticated", "{query}");
    }

    // Nothing changed.
    let after = schema
        .execute("{ authors { totalCount } books { totalCount } book(id: 1) { title } }")
        .await;
    assert_eq!(
        data(after),
        json!({
            "authors": { "totalCount": 100 },
            "books": { "totalCount": 100 },
            "book": { "title": "The Hobbit" }
        })
    );
}

#[tokio::test]
async fn same_mutation_works_when_logged_in() {
    let schema = test_schema().await;
    let query = r#"mutation { createBook(input: { title: "Test Book", authorId: 1 }) { title } }"#;

    assert_eq!(error(schema.execute(query).await), "Not authenticated");
    assert_eq!(
        data(schema.execute(as_user(query)).await),
        json!({ "createBook": { "title": "Test Book" } })
    );
}

#[tokio::test]
async fn queries_do_not_need_login() {
    let schema = test_schema().await;
    let response = schema.execute("{ books(limit: 1) { items { title } } }").await;

    assert_eq!(
        data(response),
        json!({ "books": { "items": [{ "title": "The Hobbit" }] } })
    );
}

#[tokio::test]
async fn register_returns_token_and_user() {
    let schema = test_schema().await;
    let response = schema
        .execute(
            r#"mutation {
                register(input: { username: "  reader  ", password: "password123" }) {
                    token user { id username }
                }
            }"#,
        )
        .await;
    let result = data(response);

    assert_eq!(
        result["register"]["user"],
        json!({ "id": "3", "username": "reader" })
    );
    assert!(!result["register"]["token"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn register_rejects_bad_input() {
    let schema = test_schema().await;

    let cases = [
        (r#"mutation { register(input: { username: "  ", password: "password123" }) { token } }"#, "Username cannot be blank"),
        (r#"mutation { register(input: { username: "reader", password: "short" }) { token } }"#, "Password must be at least 8 characters"),
    ];
    for (query, expected) in cases {
        assert_eq!(error(schema.execute(query).await), expected, "{query}");
    }
}

#[tokio::test]
async fn register_rejects_duplicate_username() {
    let schema = test_schema().await;
    let query = r#"mutation { register(input: { username: "reader", password: "password123" }) { token } }"#;

    data(schema.execute(query).await);
    assert_eq!(error(schema.execute(query).await), "Username is already taken");
}

#[tokio::test]
async fn login_returns_token_for_correct_password() {
    let schema = test_schema().await;
    data(schema
        .execute(r#"mutation { register(input: { username: "reader", password: "password123" }) { token } }"#)
        .await);

    let response = schema
        .execute(r#"mutation { login(input: { username: "reader", password: "password123" }) { token user { username } } }"#)
        .await;
    let result = data(response);

    assert_eq!(result["login"]["user"], json!({ "username": "reader" }));
    assert!(!result["login"]["token"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn login_gives_same_error_for_wrong_password_and_unknown_user() {
    let schema = test_schema().await;
    data(schema
        .execute(r#"mutation { register(input: { username: "reader", password: "password123" }) { token } }"#)
        .await);

    let wrong_password = schema
        .execute(r#"mutation { login(input: { username: "reader", password: "wrong-password" }) { token } }"#)
        .await;
    let unknown_user = schema
        .execute(r#"mutation { login(input: { username: "nobody", password: "password123" }) { token } }"#)
        .await;

    assert_eq!(error(wrong_password), "Invalid username or password");
    assert_eq!(error(unknown_user), "Invalid username or password");
}

#[tokio::test]
async fn me_is_null_when_anonymous() {
    let schema = test_schema().await;
    let response = schema.execute("{ me { username } }").await;

    assert_eq!(data(response), json!({ "me": null }));
}

#[tokio::test]
async fn me_returns_logged_in_user() {
    let schema = test_schema().await;
    let response = schema.execute(as_user("{ me { id username } }")).await;

    assert_eq!(
        data(response),
        json!({ "me": { "id": "1", "username": "tester" } })
    );
}

#[tokio::test]
async fn password_hash_is_not_exposed() {
    let schema = test_schema().await;
    let response = schema
        .execute(r#"mutation { register(input: { username: "reader", password: "password123" }) { user { passwordHash } } }"#)
        .await;

    assert!(error(response).contains("Unknown field"));
}

// ---- Subscriptions ----

#[tokio::test]
async fn subscription_receives_new_book() {
    let schema = test_schema().await;
    let mut stream = schema.execute_stream(as_user("subscription { bookAdded { title author { name } } }"));

    // Poll once so the subscription is registered before the mutation runs.
    // Nothing has been sent yet, so this is expected to time out.
    let nothing_yet = tokio::time::timeout(Duration::from_millis(50), stream.next()).await;
    assert!(nothing_yet.is_err());

    data(schema
        .execute(as_user(
            r#"mutation { createBook(input: { title: "Test Book", authorId: 1 }) { id } }"#,
        ))
        .await);

    let event = stream.next().await.unwrap();
    assert_eq!(
        data(event),
        json!({ "bookAdded": { "title": "Test Book", "author": { "name": "J.R.R. Tolkien" } } })
    );
}

#[tokio::test]
async fn subscription_ignores_failed_creates() {
    let schema = test_schema().await;
    let mut stream = schema.execute_stream(as_user("subscription { bookAdded { title } }"));

    let nothing_yet = tokio::time::timeout(Duration::from_millis(50), stream.next()).await;
    assert!(nothing_yet.is_err());

    // Three creates that fail: unknown author, blank title, not logged in.
    error(schema
        .execute(as_user(r#"mutation { createBook(input: { title: "x", authorId: 999 }) { id } }"#))
        .await);
    error(schema
        .execute(as_user(r#"mutation { createBook(input: { title: " ", authorId: 1 }) { id } }"#))
        .await);
    error(schema
        .execute(r#"mutation { createBook(input: { title: "Anonymous", authorId: 1 }) { id } }"#)
        .await);

    // Then one that succeeds. It must be the first thing the subscriber sees.
    data(schema
        .execute(as_user(r#"mutation { createBook(input: { title: "Real Book", authorId: 1 }) { id } }"#))
        .await);

    let event = stream.next().await.unwrap();
    assert_eq!(data(event), json!({ "bookAdded": { "title": "Real Book" } }));
}

#[tokio::test]
async fn subscription_does_not_replay_old_books() {
    let schema = test_schema().await;

    // Created before anyone subscribes.
    data(schema
        .execute(as_user(r#"mutation { createBook(input: { title: "Earlier Book", authorId: 1 }) { id } }"#))
        .await);

    let mut stream = schema.execute_stream(as_user("subscription { bookAdded { title } }"));
    let nothing = tokio::time::timeout(Duration::from_millis(50), stream.next()).await;

    assert!(nothing.is_err());
}

// ---- Filtering ----

#[tokio::test]
async fn books_can_be_filtered_by_author() {
    let schema = test_schema().await;
    let response = schema
        .execute("{ books(authorId: 1) { items { title } totalCount hasNextPage } }")
        .await;

    assert_eq!(
        data(response),
        json!({ "books": {
            "items": [
                { "title": "The Hobbit" },
                { "title": "The Fellowship of the Ring" },
                { "title": "The Silmarillion" }
            ],
            "totalCount": 3,
            "hasNextPage": false
        } })
    );
}

#[tokio::test]
async fn books_can_be_filtered_by_title_ignoring_case() {
    let schema = test_schema().await;
    let response = schema
        .execute(r#"{ books(titleContains: "dune") { items { title } totalCount } }"#)
        .await;

    assert_eq!(
        data(response),
        json!({ "books": {
            "items": [{ "title": "Dune" }, { "title": "Dune Messiah" }],
            "totalCount": 2
        } })
    );
}

#[tokio::test]
async fn book_filters_combine() {
    let schema = test_schema().await;
    let response = schema
        .execute(r#"{ books(authorId: 2, titleContains: "code") { items { title } totalCount } }"#)
        .await;

    assert_eq!(
        data(response),
        json!({ "books": { "items": [{ "title": "The Da Vinci Code" }], "totalCount": 1 } })
    );
}

#[tokio::test]
async fn book_filters_page_within_the_matches() {
    let schema = test_schema().await;
    let response = schema
        .execute("{ books(authorId: 1, limit: 2, offset: 1) { items { title } totalCount hasNextPage } }")
        .await;

    assert_eq!(
        data(response),
        json!({ "books": {
            "items": [
                { "title": "The Fellowship of the Ring" },
                { "title": "The Silmarillion" }
            ],
            "totalCount": 3,
            "hasNextPage": false
        } })
    );
}

#[tokio::test]
async fn book_filter_edge_cases() {
    let schema = test_schema().await;

    // A blank search filters nothing.
    let blank = schema.execute(r#"{ books(titleContains: "  ") { totalCount } }"#).await;
    assert_eq!(data(blank), json!({ "books": { "totalCount": 100 } }));

    // % and _ are plain characters, not wildcards.
    let wildcard = schema.execute(r#"{ books(titleContains: "%") { totalCount } }"#).await;
    assert_eq!(data(wildcard), json!({ "books": { "totalCount": 0 } }));

    // An author that does not exist simply has no books.
    let unknown = schema.execute("{ books(authorId: 999) { items { id } totalCount } }").await;
    assert_eq!(data(unknown), json!({ "books": { "items": [], "totalCount": 0 } }));
}

// ---- Cursor pagination ----

#[tokio::test]
async fn connection_returns_first_page() {
    let schema = test_schema().await;
    let response = schema
        .execute(
            "{ booksConnection(first: 2) {
                edges { node { id title } }
                pageInfo { hasPreviousPage hasNextPage }
                totalCount
            } }",
        )
        .await;

    assert_eq!(
        data(response),
        json!({ "booksConnection": {
            "edges": [
                { "node": { "id": "1", "title": "The Hobbit" } },
                { "node": { "id": "2", "title": "Angels & Demons" } }
            ],
            "pageInfo": { "hasPreviousPage": false, "hasNextPage": true },
            "totalCount": 100
        } })
    );
}

#[tokio::test]
async fn connection_continues_after_a_cursor() {
    let schema = test_schema().await;
    let first = data(schema.execute("{ booksConnection(first: 2) { pageInfo { endCursor } } }").await);
    let cursor = first["booksConnection"]["pageInfo"]["endCursor"].as_str().unwrap();

    let response = schema
        .execute(format!(
            r#"{{ booksConnection(first: 2, after: "{cursor}") {{
                edges {{ node {{ id }} }}
                pageInfo {{ hasPreviousPage hasNextPage }}
            }} }}"#
        ))
        .await;

    assert_eq!(
        data(response),
        json!({ "booksConnection": {
            "edges": [{ "node": { "id": "3" } }, { "node": { "id": "4" } }],
            "pageInfo": { "hasPreviousPage": true, "hasNextPage": true }
        } })
    );
}

#[tokio::test]
async fn connection_uses_default_page_size() {
    let schema = test_schema().await;
    let result = data(schema.execute("{ booksConnection { edges { node { id } } } }").await);

    assert_eq!(result["booksConnection"]["edges"].as_array().unwrap().len(), 10);
}

#[tokio::test]
async fn connection_pages_backward() {
    let schema = test_schema().await;

    // The last two books.
    let last = data(schema
        .execute(
            "{ booksConnection(last: 2) {
                edges { node { id } }
                pageInfo { startCursor hasPreviousPage hasNextPage }
            } }",
        )
        .await);
    assert_eq!(
        last["booksConnection"]["edges"],
        json!([{ "node": { "id": "99" } }, { "node": { "id": "100" } }])
    );
    assert_eq!(last["booksConnection"]["pageInfo"]["hasPreviousPage"], true);
    assert_eq!(last["booksConnection"]["pageInfo"]["hasNextPage"], false);

    // The two before those.
    let cursor = last["booksConnection"]["pageInfo"]["startCursor"].as_str().unwrap();
    let previous = data(schema
        .execute(format!(
            r#"{{ booksConnection(last: 2, before: "{cursor}") {{
                edges {{ node {{ id }} }}
                pageInfo {{ hasPreviousPage hasNextPage }}
            }} }}"#
        ))
        .await);
    assert_eq!(
        previous,
        json!({ "booksConnection": {
            "edges": [{ "node": { "id": "97" } }, { "node": { "id": "98" } }],
            "pageInfo": { "hasPreviousPage": true, "hasNextPage": true }
        } })
    );
}

#[tokio::test]
async fn connection_reaches_the_end() {
    let schema = test_schema().await;
    let response = schema
        .execute(
            "{ booksConnection(first: 50, authorId: 1) {
                edges { node { title } }
                pageInfo { hasPreviousPage hasNextPage }
                totalCount
            } }",
        )
        .await;

    assert_eq!(
        data(response),
        json!({ "booksConnection": {
            "edges": [
                { "node": { "title": "The Hobbit" } },
                { "node": { "title": "The Fellowship of the Ring" } },
                { "node": { "title": "The Silmarillion" } }
            ],
            "pageInfo": { "hasPreviousPage": false, "hasNextPage": false },
            "totalCount": 3
        } })
    );
}

// The weakness of offset pagination, and what a cursor fixes: deleting a row
// from page 1 does not make page 2 skip a row.
#[tokio::test]
async fn cursor_page_is_stable_when_earlier_rows_are_deleted() {
    let schema = test_schema().await;
    let first = data(schema.execute("{ booksConnection(first: 2) { pageInfo { endCursor } } }").await);
    let cursor = first["booksConnection"]["pageInfo"]["endCursor"].as_str().unwrap();

    data(schema.execute(as_user("mutation { deleteBook(id: 1) }")).await);

    // Offset: "skip 2" now skips books 2 and 3, so book 3 is never seen.
    let offset = data(schema.execute("{ books(limit: 2, offset: 2) { items { id } } }").await);
    assert_eq!(offset["books"]["items"], json!([{ "id": "4" }, { "id": "5" }]));

    // Cursor: "after book 2" still starts at book 3.
    let after = data(schema
        .execute(format!(
            r#"{{ booksConnection(first: 2, after: "{cursor}") {{ edges {{ node {{ id }} }} }} }}"#
        ))
        .await);
    assert_eq!(
        after["booksConnection"]["edges"],
        json!([{ "node": { "id": "3" } }, { "node": { "id": "4" } }])
    );
}

#[tokio::test]
async fn connection_rejects_invalid_arguments() {
    let schema = test_schema().await;

    for query in [
        "{ booksConnection(first: 0) { totalCount } }",
        "{ booksConnection(first: 51) { totalCount } }",
        "{ booksConnection(last: 51) { totalCount } }",
        "{ authorsConnection(first: 0) { totalCount } }",
    ] {
        assert_eq!(
            error(schema.execute(query).await),
            "Page size should be between 1 and 50",
            "{query}"
        );
    }

    assert_eq!(
        error(schema.execute("{ booksConnection(first: 2, last: 2) { totalCount } }").await),
        "Use either first or last, not both"
    );

    // A negative size, and a cursor that is not one of ours.
    for query in [
        "{ booksConnection(first: -1) { totalCount } }",
        r#"{ booksConnection(after: "not a cursor") { totalCount } }"#,
    ] {
        assert!(!schema.execute(query).await.errors.is_empty(), "{query}");
    }
}

#[tokio::test]
async fn connection_works_for_authors() {
    let schema = test_schema().await;
    let response = schema
        .execute(
            "{ authorsConnection(first: 2) {
                edges { node { id name } }
                pageInfo { hasNextPage }
                totalCount
            } }",
        )
        .await;

    assert_eq!(
        data(response),
        json!({ "authorsConnection": {
            "edges": [
                { "node": { "id": "1", "name": "J.R.R. Tolkien" } },
                { "node": { "id": "2", "name": "Dan Brown" } }
            ],
            "pageInfo": { "hasNextPage": true },
            "totalCount": 100
        } })
    );
}

#[tokio::test]
async fn author_books_can_be_paged() {
    let schema = test_schema().await;
    let response = schema
        .execute(
            "{ author(id: 1) { booksConnection(first: 2) {
                edges { node { title } }
                pageInfo { hasNextPage }
                totalCount
            } } }",
        )
        .await;

    assert_eq!(
        data(response),
        json!({ "author": { "booksConnection": {
            "edges": [
                { "node": { "title": "The Hobbit" } },
                { "node": { "title": "The Fellowship of the Ring" } }
            ],
            "pageInfo": { "hasNextPage": true },
            "totalCount": 3
        } } })
    );
}

// ---- Batched relations (DataLoader) ----

#[tokio::test]
async fn every_book_on_a_page_gets_its_own_author() {
    let schema = test_schema().await;
    let result = data(schema
        .execute("{ books(limit: 5) { items { title author { name } } } }")
        .await);

    assert_eq!(
        result["books"]["items"],
        json!([
            { "title": "The Hobbit", "author": { "name": "J.R.R. Tolkien" } },
            { "title": "Angels & Demons", "author": { "name": "Dan Brown" } },
            { "title": "Don Quixote", "author": { "name": "Miguel de Cervantes" } },
            { "title": "Robinson Crusoe", "author": { "name": "Daniel Defoe" } },
            { "title": "Gulliver's Travels", "author": { "name": "Jonathan Swift" } }
        ])
    );
}

#[tokio::test]
async fn every_author_on_a_page_gets_their_own_books() {
    let schema = test_schema().await;
    let result = data(schema
        .execute("{ authors(limit: 3) { items { name books { title } } } }")
        .await);

    assert_eq!(
        result["authors"]["items"],
        json!([
            { "name": "J.R.R. Tolkien", "books": [
                { "title": "The Hobbit" },
                { "title": "The Fellowship of the Ring" },
                { "title": "The Silmarillion" }
            ] },
            { "name": "Dan Brown", "books": [
                { "title": "Angels & Demons" },
                { "title": "The Da Vinci Code" },
                { "title": "Inferno" }
            ] },
            { "name": "Jane Austen", "books": [
                { "title": "Pride and Prejudice" },
                { "title": "Emma" }
            ] }
        ])
    );
}

#[tokio::test]
async fn relations_see_changes_made_in_the_same_session() {
    let schema = test_schema().await;

    // Loaders must not cache across requests: move a book, then read it back.
    data(schema.execute("{ book(id: 1) { author { name } } }").await);
    data(schema.execute(as_user("mutation { updateBook(id: 1, input: { authorId: 2 }) { id } }")).await);

    let after = data(schema
        .execute("{ book(id: 1) { author { name } } author(id: 1) { books { title } } }")
        .await);
    assert_eq!(
        after,
        json!({
            "book": { "author": { "name": "Dan Brown" } },
            "author": { "books": [
                { "title": "The Fellowship of the Ring" },
                { "title": "The Silmarillion" }
            ] }
        })
    );
}

// ---- Ownership ----

#[tokio::test]
async fn new_book_is_owned_by_its_creator() {
    let schema = test_schema().await;
    let response = schema
        .execute(as_user(
            r#"mutation { createBook(input: { title: "Mine", authorId: 1 }) { title createdBy { id username } } }"#,
        ))
        .await;

    assert_eq!(
        data(response),
        json!({ "createBook": { "title": "Mine", "createdBy": { "id": "1", "username": "tester" } } })
    );
}

#[tokio::test]
async fn seed_books_have_no_owner() {
    let schema = test_schema().await;
    let response = schema.execute("{ book(id: 1) { createdBy { username } } }").await;

    assert_eq!(data(response), json!({ "book": { "createdBy": null } }));
}

#[tokio::test]
async fn only_the_owner_can_change_a_book() {
    let schema = test_schema().await;
    data(schema
        .execute(as_user(r#"mutation { createBook(input: { title: "Mine", authorId: 1 }) { id } }"#))
        .await);

    // The other user is refused, and nothing changes.
    let update = schema
        .execute(as_other_user(r#"mutation { updateBook(id: 101, input: { title: "Stolen" }) { id } }"#))
        .await;
    assert_eq!(error(update), "Book 101 belongs to another user");

    let delete = schema.execute(as_other_user("mutation { deleteBook(id: 101) }")).await;
    assert_eq!(error(delete), "Book 101 belongs to another user");

    let unchanged = data(schema.execute("{ book(id: 101) { title } }").await);
    assert_eq!(unchanged, json!({ "book": { "title": "Mine" } }));

    // The owner is allowed.
    let update = schema
        .execute(as_user(r#"mutation { updateBook(id: 101, input: { title: "Still Mine" }) { title } }"#))
        .await;
    assert_eq!(data(update), json!({ "updateBook": { "title": "Still Mine" } }));

    let delete = schema.execute(as_user("mutation { deleteBook(id: 101) }")).await;
    assert_eq!(data(delete), json!({ "deleteBook": true }));
}

#[tokio::test]
async fn any_user_can_change_a_seed_book() {
    let schema = test_schema().await;
    let response = schema
        .execute(as_other_user("mutation { updateBook(id: 1, input: { year: 1938 }) { year } }"))
        .await;

    assert_eq!(data(response), json!({ "updateBook": { "year": 1938 } }));
}

// ---- Subscription authentication ----

#[tokio::test]
async fn subscription_needs_login() {
    let schema = test_schema().await;
    let mut stream = schema.execute_stream("subscription { bookAdded { title } }");

    let response = stream.next().await.unwrap();
    assert_eq!(error(response), "Not authenticated");
}

// ---- Query limits ----

#[tokio::test]
async fn query_that_is_too_deep_is_rejected() {
    let schema = test_schema().await;
    // author { books { author { books { ... } } } }, twenty levels down.
    let levels = 10;
    let query = format!(
        "{{ author(id: 1) {{ {} name {} }} }}",
        "books { author { ".repeat(levels),
        "} } ".repeat(levels)
    );
    let response = schema.execute(query).await;

    assert_eq!(error(response), "Query is nested too deep.");
}

#[tokio::test]
async fn query_that_is_too_complex_is_rejected() {
    let schema = test_schema().await;
    let response = schema
        .execute(
            "{ books(limit: 50) { items {
                id title year
                author { id name booksConnection(first: 50) { edges { node { id title year } } } }
            } } }",
        )
        .await;

    assert_eq!(error(response), "Query is too complex.");
}

#[tokio::test]
async fn ordinary_nested_query_is_within_limits() {
    let schema = test_schema().await;
    let response = schema
        .execute(
            "{ books(limit: 50) { items { id title year author { id name books { id title } } } totalCount }
               authors(limit: 50) { items { id name books { id title year } } totalCount } }",
        )
        .await;

    data(response);
}

// GraphiQL loads the schema with this query to build its docs and autocomplete.
// The limits must leave room for it.
#[tokio::test]
async fn introspection_is_within_limits() {
    let schema = test_schema().await;
    let response = schema
        .execute(
            "query IntrospectionQuery {
                __schema {
                    queryType { name }
                    mutationType { name }
                    subscriptionType { name }
                    types { ...FullType }
                    directives { name description locations args { ...InputValue } }
                }
            }
            fragment FullType on __Type {
                kind name description
                fields(includeDeprecated: true) {
                    name description
                    args { ...InputValue }
                    type { ...TypeRef }
                    isDeprecated deprecationReason
                }
                inputFields { ...InputValue }
                interfaces { ...TypeRef }
                enumValues(includeDeprecated: true) { name description isDeprecated deprecationReason }
                possibleTypes { ...TypeRef }
            }
            fragment InputValue on __InputValue { name description type { ...TypeRef } defaultValue }
            fragment TypeRef on __Type {
                kind name
                ofType { kind name ofType { kind name ofType { kind name ofType { kind name
                ofType { kind name ofType { kind name ofType { kind name ofType { kind name
                ofType { kind name } } } } } } } } }
            }",
        )
        .await;

    data(response);
}
