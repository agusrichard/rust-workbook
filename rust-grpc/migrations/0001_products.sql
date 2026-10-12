CREATE TABLE products (
    id              INTEGER PRIMARY KEY,
    name            TEXT NOT NULL,
    stock           INTEGER NOT NULL CHECK (stock >= 0)
);


INSERT INTO products (name, stock) VALUES ("television", 10);
INSERT INTO products (name, stock) VALUES ("refrigerator", 10);