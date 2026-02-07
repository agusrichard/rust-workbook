# Rust Code Style Guide

## Formatting
- Use `rustfmt` for all code formatting.
- Indentation: 4 spaces.
- Max line length: 100 characters (default).

## Naming Conventions
- Types (structs, enums, traits): `UpperCamelCase`
- Functions, methods, variables, modules: `snake_case`
- Constants and statics: `SCREAMING_SNAKE_CASE`

## Best Practices
- Use `Result` and `Option` for error handling; avoid `unwrap()` in production code.
- Prefer idiomatic iterators over C-style loops.
- Document public APIs using doc comments (`///`).
