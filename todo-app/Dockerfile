FROM rust:slim

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-watch --locked

WORKDIR /app
COPY . .

CMD ["cargo", "watch", "-x", "run"]
