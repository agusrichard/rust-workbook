CREATE TABLE authors (
    id   INTEGER PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE books (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL,
    year INTEGER NULL,
    author_id INTEGER NOT NULL REFERENCES authors(id)
);

INSERT INTO authors (name) VALUES ('J.R.R. Tolkien'), ('Dan Brown');
INSERT INTO books (title, year, author_id) VALUES ('The Hobbit', 1937, 1), ('Angels & Demons', 2000, 2);