# Books & Authors: GraphQL CRUD in Rust

A learning project. This file is a guide: it says what to build and in what order, and gives hints, but leaves the code to you.

## Goal

Build a GraphQL API where a client can create, read, update, and delete **authors** and **books**, and navigate between them (a book's author, an author's books). Data is stored in SQLite, lists are paginated, writes require a logged-in user, and clients can **subscribe** to be told live when a book is added.

By the end you should have practised: defining a schema, writing resolvers, sharing state through the context, input types, nested (relation) resolvers, error handling, database access, pagination, authentication, and subscriptions.

## Scope

**In scope**

- One GraphQL endpoint served over HTTP, with GraphiQL for trying queries
- CRUD for authors and books, stored in SQLite
- The author/book relation in both directions
- Basic validation and meaningful errors
- Offset pagination on the list queries
- Register and login, with a token required for writes
- A live subscription for newly added books, over WebSocket
- A few tests

**Out of scope (for now)**

- Roles or ownership (any logged-in user can change any book or author)
- Authentication on the WebSocket connection
- Cursor-based pagination

These come back as stretch goals at the end.

## Stack

| Purpose | Crate |
| --- | --- |
| GraphQL | `async-graphql` |
| HTTP server | `axum` |
| GraphQL/axum glue | `async-graphql-axum` |
| Async runtime | `tokio` (features: `macros`, `rt-multi-thread`, `sync`) |
| Database | `sqlx` (features: `runtime-tokio`, `sqlite`) |
| Password hashing | `argon2` |
| Tokens (JWT) | `jsonwebtoken`, with `serde` (feature: `derive`) for the claims |
| Config from `.env` | `dotenvy` |
| Streams, for subscriptions | `tokio-stream` (feature: `sync`) |

Add them with `cargo add` so you get current versions. Two things to watch:

- If the build complains about mismatched `axum` types, check which `axum` version `async-graphql-axum` depends on and use that one.
- Recent `jsonwebtoken` versions ask you to choose a crypto backend feature. Its README lists the options.

You do not need all of these on day one. Add each when its milestone starts.

## Requirements

### Data model

Three tables. Ids are `INTEGER PRIMARY KEY` in SQLite and exposed as `ID` in GraphQL.

**authors**

| Column | Type | Notes |
| --- | --- | --- |
| `id` | INTEGER | Primary key |
| `name` | TEXT | Not null, not blank |

**books**

| Column | Type | Notes |
| --- | --- | --- |
| `id` | INTEGER | Primary key |
| `title` | TEXT | Not null, not blank |
| `year` | INTEGER | Nullable publication year |
| `author_id` | INTEGER | Not null, foreign key to `authors(id)`; not exposed directly |

**users**

| Column | Type | Notes |
| --- | --- | --- |
| `id` | INTEGER | Primary key |
| `username` | TEXT | Not null, unique |
| `password_hash` | TEXT | Not null; never exposed through GraphQL |

One author has many books. Every book has exactly one author. Users are only for logging in; they are not linked to books or authors.

### Target schema

This is the contract your server should end up exposing. `async-graphql` generates it from your Rust code, so treat this as the thing to check against, not something to paste in.

```graphql
type Author {
  id: ID!
  name: String!
  books: [Book!]!
}

type Book {
  id: ID!
  title: String!
  year: Int
  author: Author!
}

type AuthorPage {
  items: [Author!]!
  totalCount: Int!
  hasNextPage: Boolean!
}

type BookPage {
  items: [Book!]!
  totalCount: Int!
  hasNextPage: Boolean!
}

type User {
  id: ID!
  username: String!
}

type AuthPayload {
  token: String!
  user: User!
}

input CredentialsInput { username: String!, password: String! }

input CreateAuthorInput { name: String! }
input UpdateAuthorInput { name: String }

input CreateBookInput { title: String!, year: Int, authorId: ID! }
input UpdateBookInput { title: String, year: Int, authorId: ID }

type Query {
  authors(limit: Int! = 10, offset: Int! = 0): AuthorPage!
  author(id: ID!): Author
  books(limit: Int! = 10, offset: Int! = 0): BookPage!
  book(id: ID!): Book
  me: User
}

type Mutation {
  register(input: CredentialsInput!): AuthPayload!
  login(input: CredentialsInput!): AuthPayload!

  createAuthor(input: CreateAuthorInput!): Author!
  updateAuthor(id: ID!, input: UpdateAuthorInput!): Author!
  deleteAuthor(id: ID!): Boolean!

  createBook(input: CreateBookInput!): Book!
  updateBook(id: ID!, input: UpdateBookInput!): Book!
  deleteBook(id: ID!): Boolean!
}

type Subscription {
  bookAdded: Book!
}
```

### Behaviour rules

**Reads**

1. `author(id)` and `book(id)` return `null` when nothing matches. They do not error.
2. `authors` and `books` are ordered by id ascending. `limit` must be between 1 and 50 and `offset` must be 0 or more; anything else is an error.
3. `totalCount` is the number of rows in the whole table, ignoring `limit` and `offset`. `hasNextPage` is true when rows exist beyond the current page.

**Writes**

4. Update and delete on an id that does not exist return an error.
5. Update only changes the fields that were provided; omitted fields keep their value.
6. `name` and `title` cannot be empty or whitespace only.
7. Creating or updating a book with an `authorId` that does not exist returns an error.
8. Deleting an author who still has books returns an error. The client must delete or reassign the books first.

**Authentication**

9. Queries and the subscription are public. Every mutation except `register` and `login` requires a valid token; without one it returns a "not authenticated" error and changes nothing.
10. `register` requires a non-blank, unused username and a password of at least 8 characters. Only a hash of the password is stored.
11. `login` returns the same error for an unknown username and a wrong password.
12. The client sends the token as an `Authorization: Bearer <token>` header. Tokens expire after 24 hours. A missing, invalid, or expired token means the request is treated as anonymous.
13. `me` returns the logged-in user, or `null` when anonymous.

**Subscriptions**

14. `bookAdded` emits each book once, right after a successful `createBook`. A failed `createBook` emits nothing.
15. A subscriber only receives books created after it subscribed, and every connected subscriber receives each one.

## Suggested layout

```
migrations/      // .sql files, one per schema change
src/
  main.rs        // config, server setup, routes
  db.rs          // pool setup, running migrations
  models.rs      // Author, Book, User, page and input types
  auth.rs        // hashing, tokens, the current-user type
  schema/
    mod.rs       // builds the Schema
    query.rs
    mutation.rs
    subscription.rs
.env             // DATABASE_URL, JWT_SECRET (do not commit)
```

Start with everything in `main.rs` if you prefer, and split once it feels crowded. Add `.env` and the `.db` file to `.gitignore`.

## Milestones

Work through these in order. Each ends with a check you can run in GraphiQL.

### 1. Hello GraphQL

- [ ] Create the Cargo project and add `async-graphql`, `async-graphql-axum`, `axum`, `tokio`
- [ ] Write a `Query` with one field, `hello`, that returns a string
- [ ] Build a `Schema` with `EmptyMutation` and `EmptySubscription`
- [ ] Serve it with axum: `POST /graphql` for queries, `GET /` for GraphiQL

Hints: `async_graphql_axum::GraphQL` wraps a schema as an axum service. `async_graphql::http::GraphiQLSource` builds the GraphiQL page.

**Done when:** opening `http://localhost:8000` shows GraphiQL and `{ hello }` returns your string.

### 2. Database

- [ ] Add `sqlx` and `dotenvy`; put `DATABASE_URL=sqlite:books.db` in `.env`
- [ ] Write migrations that create `authors` and `books`, including the foreign key
- [ ] On startup: open a `SqlitePool`, creating the file if missing, and run the migrations
- [ ] Seed two authors and three books (a migration is the easiest place)
- [ ] Define `Author` and `Book` structs that can be loaded from a row
- [ ] Register the pool on the schema so resolvers can reach it

Hints:

- Migrations are plain `.sql` files in `migrations/`, named like `0001_create_authors.sql`. `sqlx::migrate!()` embeds and runs them. The `sqlx-cli` tool can generate the files, but creating them by hand is fine.
- `SqliteConnectOptions` has `create_if_missing(true)`. sqlx turns on SQLite foreign key enforcement by default.
- `#[derive(sqlx::FromRow)]` lets `sqlx::query_as` map rows to your struct. Stick to the `query_as` *function* for now; the `query_as!` *macro* checks SQL at compile time but needs extra setup.
- SQLite integers are `i64` in Rust.
- Attach the pool with `Schema::build(...).data(pool).finish()`. A pool is cheap to clone and already safe to share.

**Done when:** the server starts, `books.db` exists, and `sqlite3 books.db 'select * from books'` shows your seed rows.

### 3. Queries

- [ ] `authors` and `books` return everything, as plain lists for now
- [ ] `author(id)` and `book(id)` return one item or `null`
- [ ] Expose only scalar fields for now (`id`, `name`, `title`, `year`)

The lists become paginated in milestone 7. Changing a field's return type later is a breaking change for clients, which is worth experiencing once.

Hints: `#[derive(SimpleObject)]` turns a struct into a GraphQL type. A resolver gets the pool through `ctx.data::<SqlitePool>()`, where `ctx` is a `&Context<'_>` parameter. Use `#[graphql(skip)]` to hide `author_id`. An `i64` field is exposed as `Int`, so to get `ID` in the schema use `async_graphql::ID` and convert to and from `i64`. `fetch_optional` returns an `Option`, which suits the single-item queries.

**Done when:**

```graphql
{ books { id title year } author(id: "1") { name } }
```

returns your seed data, and an unknown id gives `null`.

### 4. Relations

- [ ] Add `author` to `Book`
- [ ] Add `books` to `Author`

This is the step where GraphQL starts to feel different from REST. These fields are not columns on the row; they are computed by a resolver that runs its own SQL query.

Hints: keep `SimpleObject` and add `#[graphql(complex)]`, then put the extra resolvers in a `#[ComplexObject] impl` block. Inside it, `&self` is the parent object.

**Done when:**

```graphql
{ authors { name books { title author { name } } } }
```

works at any nesting depth.

Think about: how many SQL queries does that run for 100 authors? This is the N+1 problem; you will fix it in a stretch goal.

### 5. Mutations

- [ ] Define the author and book input types
- [ ] Implement create, update, delete for authors
- [ ] Implement create, update, delete for books
- [ ] Replace `EmptyMutation` with your `Mutation` type

Leave authentication out for now; it arrives in milestone 8.

Hints: `#[derive(InputObject)]` for inputs. Optional input fields are `Option<T>`. SQLite supports `INSERT ... RETURNING` and `UPDATE ... RETURNING`, so one statement can write and give back the row. For partial updates, either load the row, change it, and write it back, or look at SQL's `COALESCE`. `rows_affected()` tells you whether a delete or update matched anything.

**Done when:** you can create an author, create a book for them, rename the book, then delete it, and the changes survive a server restart.

### 6. Validation and errors

- [ ] Enforce the write rules (4 to 8)
- [ ] Return clear messages ("Author 42 not found", not a raw SQL error)

Hints: return `async_graphql::Result<T>` from a resolver and build errors with `async_graphql::Error::new(...)` or `?` on your own error type. The foreign key already makes the database refuse a bad `author_id` or deleting an author with books; your job is to turn that into a readable message, either by checking first or by translating the database error. Look at how a failure appears in the response: an `errors` array alongside `data`, still with HTTP 200.

**Done when:** each write rule has a query in GraphiQL that triggers its error.

### 7. Pagination

- [ ] Define `AuthorPage` and `BookPage`
- [ ] Add `limit` and `offset` arguments with defaults to `authors` and `books`
- [ ] Return one page of items plus `totalCount` and `hasNextPage`
- [ ] Enforce rule 2 on the arguments

`Author.books` stays a plain list.

Hints: `#[graphql(default = 10)]` on a resolver argument sets its default. SQL does the paging with `ORDER BY id LIMIT ? OFFSET ?`; without a fixed order, pages can overlap. `totalCount` is a separate `SELECT COUNT(*)`, which comes back as `i64` while GraphQL `Int` is 32-bit. `hasNextPage` can be worked out from offset, page size, and total. Two near-identical page types is fine; a generic one is possible but adds macro noise.

**Done when:** with at least five books,

```graphql
{ books(limit: 2, offset: 2) { items { id title } totalCount hasNextPage } }
```

returns the third and fourth book with the right count, the last page reports `hasNextPage: false`, and `limit: 0` or `limit: 500` is an error.

Think about: what happens to page 2 if someone deletes a row from page 1 between your two requests? That weakness is why cursor pagination exists.

### 8. Authentication

Two separate jobs: issuing tokens (`register`, `login`) and checking them on each request.

**Issuing**

- [ ] Add a migration for `users`; add `JWT_SECRET` to `.env`
- [ ] `register`: validate, hash the password, insert the user, return a token
- [ ] `login`: find the user, verify the password, return a token

**Checking**

- [ ] Replace the ready-made `GraphQL` service with your own axum handler for `POST /graphql`
- [ ] In the handler, read the `Authorization` header and validate the token
- [ ] If valid, attach the current user to that one request before executing it
- [ ] Add the `me` query
- [ ] Make every author and book mutation require a current user

Hints:

- `argon2` hashes and verifies passwords; each hash carries its own random salt, so two users with the same password get different hashes.
- A JWT is a signed JSON payload. Put the user id in `sub` and an expiry in `exp`; `jsonwebtoken` checks the signature and expiry when decoding. Share the secret with resolvers through schema data.
- Schema data (the pool) is shared by all requests. Per-request data (the current user) is added to the request instead: your handler takes a `GraphQLRequest`, calls `.into_inner()`, adds `.data(...)`, then runs `schema.execute(...)`.
- In a resolver, `ctx.data_opt::<T>()` returns `None` when that data was not attached. That is your "anonymous" case.
- Start by checking for the user at the top of each mutation. Once that works, look at async-graphql's `Guard` trait and `#[graphql(guard = "...")]` to remove the repetition.
- GraphiQL has a Headers panel at the bottom for setting `Authorization`.

**Done when:**

1. `createBook` without a header fails with your "not authenticated" error.
2. `register` returns a token, and `login` with the same credentials returns one too.
3. With `{"Authorization": "Bearer <token>"}` in GraphiQL's Headers panel, `createBook` succeeds and `{ me { username } }` returns you.
4. A wrong password and an unknown username give the same error.
5. `select * from users` shows a hash, not the password.

### 9. Subscriptions

Queries and mutations are one request, one response. A subscription is one request followed by a stream of responses, pushed by the server whenever something happens. It runs over a WebSocket instead of a plain HTTP request.

You need three pieces: a channel that carries events, a mutation that publishes to it, and a subscription resolver that turns it into a stream.

- [ ] Create a broadcast channel for `Book` events and add its sender to the schema data
- [ ] Publish the new book from `createBook`, after the insert has succeeded
- [ ] Write a `Subscription` type with a `book_added` resolver that returns a stream of `Book`
- [ ] Replace `EmptySubscription` with your `Subscription` type
- [ ] Add a WebSocket route (for example `/ws`) and tell GraphiQL about it

Hints:

- `tokio::sync::broadcast` fits because every subscriber should get every event. Each subscriber calls `.subscribe()` on the `Sender` to get its own `Receiver`.
- A subscription resolver lives in a `#[Subscription] impl` block and returns `impl Stream<Item = Book>` instead of a single value.
- `tokio_stream::wrappers::BroadcastStream` turns a `Receiver` into a stream. Its items are `Result`s (a slow subscriber can miss events), so decide what to do with the error case; dropping it is fine here.
- `Book` must be `Clone` to go through a broadcast channel.
- Sending on a channel with no subscribers returns an error. That is normal, not a failure of `createBook`.
- `async_graphql_axum::GraphQLSubscription::new(schema)` is the WebSocket service. On `GraphiQLSource`, set `.subscription_endpoint("/ws")`.

**Done when:** with two GraphiQL tabs open, you run this in the first:

```graphql
subscription { bookAdded { title author { name } } }
```

then run `createBook` (with your token) in the second, and the new book appears in the first tab without re-running anything. Note that the relation resolver from milestone 4 works here too.

Extra, if you want more: add `bookDeleted: ID!`, or a single `bookChanged` that reports a kind (`CREATED`, `UPDATED`, `DELETED`) using a GraphQL enum.

### 10. Tests

- [ ] Write a helper that builds a schema on a fresh, migrated, in-memory database
- [ ] Test one query, one mutation, and one validation error
- [ ] Test pagination: a middle page and the last page
- [ ] Test that a protected mutation fails when anonymous and succeeds with a user
- [ ] Test that `bookAdded` emits after `createBook`

Hints:

- No HTTP server needed. Call `schema.execute(...).await` directly and assert on `response.data` and `response.errors`. Use `#[tokio::test]`.
- `sqlite::memory:` gives a throwaway database, but each connection gets its own, so limit the test pool to one connection. Run the same migrations on it.
- To act as a logged-in user, build an `async_graphql::Request` and attach the current user with `.data(...)`, exactly as your handler does. No token needed.
- For the subscription, `schema.execute_stream(...)` gives a stream of responses. Start it and poll it once before running the mutation, because subscribers only see books created after they subscribed.

**Done when:** `cargo test` passes.

## Stretch goals

Pick any, in any order.

- **DataLoader:** batch the `Book.author` lookups to fix N+1 (`async_graphql::dataloader`).
- **Cursor pagination:** replace offset paging with Relay-style connections (`async_graphql::connection`), and paginate `Author.books` too.
- **Filtering:** `books(authorId: ID, titleContains: String)`.
- **Ownership:** record which user created a book and only let them change it.
- **Subscription auth:** require a token on the WebSocket. Browsers cannot set headers there, so it travels in the connection's init payload.
- **Query limits:** set a maximum depth and complexity on the schema.
- **Compile-time SQL:** switch to the `sqlx::query_as!` macros.

## References

- [async-graphql book](https://async-graphql.github.io/async-graphql/en/index.html)
- [async-graphql examples](https://github.com/async-graphql/examples) (see the axum folder)
- [sqlx README](https://github.com/launchbadge/sqlx)
- [GraphQL: Learn](https://graphql.org/learn/)
- `graphql-concepts.html` in this folder
