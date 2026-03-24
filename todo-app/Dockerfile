FROM rust:1.85-slim

RUN cargo install cargo-watch --locked

WORKDIR /app
COPY . .

CMD ["cargo", "watch", "-x", "run"]
