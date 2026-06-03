# hello – Multithreaded Web Server

A simple HTTP server built as the final project for *The Rust Programming Language* (Chapter 21). It demonstrates a hand-rolled thread pool, ownership/borrowing patterns for shared state, and channel-based message passing.

## Running

```bash
cargo run
# Server listens on http://127.0.0.1:7878
```

Routes:
- `GET /` — serves `hello.html` immediately
- `GET /sleep` — waits 5 seconds, then serves `hello.html` (simulates a slow request)
- anything else — serves `404.html`

---

## Project Structure

```
src/
├── main.rs   – TCP listener, request routing, response formatting
└── lib.rs    – ThreadPool and Worker implementation
```

---

## `src/main.rs` — The Server

### Entry point

```rust
fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    let pool = ThreadPool::new(4);

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        pool.execute(|| {
            handle_connection(stream);
        });
    }
}
```

- `TcpListener::bind` opens a TCP socket on port 7878.
- `ThreadPool::new(4)` creates a pool with 4 worker threads.
- `listener.incoming()` is an iterator that yields a new `TcpStream` for every incoming connection. It blocks until a connection arrives.
- Each connection is handed off to the pool via `pool.execute(|| { ... })`. The closure captures `stream` by move, so ownership transfers into the worker thread.

### `generate_response`

```rust
fn generate_response(status_line: &str, html_file: &str) -> String {
    let contents = fs::read_to_string(html_file).unwrap();
    let length = contents.len();
    format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}")
}
```

Builds a minimal HTTP/1.1 response string. The `Content-Length` header tells the browser exactly how many bytes to expect in the body. `\r\n\r\n` is the blank line that separates headers from the body, as required by the HTTP spec.

### `handle_connection`

```rust
fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&stream);
    let request_line = buf_reader.lines().next().unwrap().unwrap();

    let response = match &request_line[..] {
        "GET / HTTP/1.1"     => generate_response("HTTP/1.1 200 OK", "hello.html"),
        "GET /sleep HTTP/1.1" => {
            thread::sleep(Duration::from_secs(5));
            generate_response("HTTP/1.1 200 OK", "hello.html")
        }
        _ => generate_response("404 NOT FOUND", "404.html"),
    };

    stream.write_all(response.as_bytes()).unwrap();
}
```

- `BufReader` wraps the stream for line-by-line reading. `.lines().next()` reads only the first line of the HTTP request (e.g. `"GET / HTTP/1.1"`).
- The `match` on `&request_line[..]` (a `&str` slice) routes to the correct handler.
- `stream.write_all` writes the entire response in one call. The function takes ownership of `stream`, which is dropped at the end — this closes the TCP connection.

---

## `src/lib.rs` — The Thread Pool

### Type alias

```rust
type Job = Box<dyn FnOnce() + Send + 'static>;
```

A `Job` is a heap-allocated, one-shot closure that:
- `FnOnce()` — can be called exactly once.
- `Send` — can be moved across thread boundaries.
- `'static` — contains no borrowed references that could dangle.

`Box<dyn ...>` is needed because closures have unique, unsized types; boxing them lets us store them in a homogeneous `Vec` or send them through a channel.

### `ThreadPool` struct

```rust
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Job>,
}
```

- `workers` — the fixed set of threads that execute jobs.
- `sender` — the sending end of an `mpsc` (multi-producer, single-consumer) channel. `ThreadPool::execute` sends jobs here; workers pull them off the other end.

### `ThreadPool::new`

```rust
pub fn new(size: usize) -> ThreadPool {
    assert!(size > 0);

    let (sender, receiver) = mpsc::channel();
    let receiver = Arc::new(Mutex::new(receiver));

    let mut workers = Vec::with_capacity(size);
    for id in 0..size {
        workers.push(Worker::new(id, Arc::clone(&receiver)));
    }

    ThreadPool { workers, sender }
}
```

**Why `Arc<Mutex<Receiver>>`?**

`mpsc::channel` gives one `Receiver`, but we need *all* worker threads to share it (each worker should be able to pick up the next available job). Two Rust rules make wrapping necessary:

| Wrapper | Reason |
|---|---|
| `Mutex<T>` | Only one thread can call `recv()` at a time. Without it, multiple threads would race to read from the channel. |
| `Arc<T>` | Multiple threads need shared ownership of the `Mutex`. `Rc<T>` is not `Send`, so `Arc` (atomic reference counting) is used instead. |

`Arc::clone` increments the reference count and gives each `Worker` its own handle to the same underlying `Mutex<Receiver>`.

### `ThreadPool::execute`

```rust
pub fn execute<F>(&self, f: F)
where
    F: FnOnce() + Send + 'static,
{
    let job = Box::new(f);
    self.sender.send(job).unwrap();
}
```

Boxes the closure into a `Job` and sends it into the channel. The next idle worker will receive and run it. `send` is non-blocking; it returns immediately after placing the job in the channel buffer.

### `Worker` struct

```rust
struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>,
}
```

Each `Worker` owns one OS thread. `JoinHandle<()>` is the handle returned by `thread::spawn`; calling `.join()` on it waits for the thread to finish.

### `Worker::new`

```rust
impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                let job = receiver.lock().unwrap().recv().unwrap();
                println!("Worker {id} got a job; executing.");
                job();
            }
        });

        Worker { id, thread }
    }
}
```

The closure passed to `thread::spawn` is `move` — it takes ownership of `receiver` (the `Arc` clone). Inside the loop:

1. `.lock()` acquires the mutex, blocking if another thread holds it.
2. `.recv()` blocks until a job arrives in the channel.
3. The mutex guard is **dropped at the end of the `let` statement**, before `job()` runs. This is important: it means the lock is released while the job executes, so other workers can pick up new jobs concurrently.
4. `job()` calls the closure once and drops it.

---

## Concurrency Model Diagram

```
main thread
    │
    ├─ TcpListener (blocks on incoming connections)
    │
    └─ ThreadPool
            │
            │  mpsc channel  (Job queue)
            │  ══════════════════════════════
            │
            ├─ Worker 0  ──► Arc<Mutex<Receiver>> ──► recv() → job()
            ├─ Worker 1  ──► Arc<Mutex<Receiver>> ──► recv() → job()
            ├─ Worker 2  ──► Arc<Mutex<Receiver>> ──► recv() → job()
            └─ Worker 3  ──► Arc<Mutex<Receiver>> ──► recv() → job()
```

All workers compete to lock the `Mutex` and call `recv()`. Whichever worker wins the lock pulls the next job. This is the classic *work-stealing via shared queue* pattern.

---

## Key Rust Concepts Illustrated

| Concept | Where |
|---|---|
| `Arc<Mutex<T>>` for shared mutable state across threads | `ThreadPool::new` |
| `move` closures to transfer ownership into threads | `Worker::new` |
| `Box<dyn Trait>` for type-erased heap values | `type Job = ...` |
| Channel (`mpsc`) for message passing | `sender` / `receiver` |
| Trait bounds (`Send`, `'static`) for thread safety | `ThreadPool::execute` |
| Lock scope narrowing (drop before `job()`) | `Worker::new` loop |
