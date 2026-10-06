-- Who created each book. NULL for the seed books, which nobody owns.
ALTER TABLE books ADD COLUMN created_by INTEGER REFERENCES users (id);
