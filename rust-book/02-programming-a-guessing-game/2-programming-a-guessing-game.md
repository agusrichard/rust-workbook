# Programming a Guessing Game

This chapter introduces fundamental Rust concepts (`let`, `match`, methods, external crates) by building a simple game where the player guesses a random number between 1 and 100.

### 1. Setting Up the Project

Create a new project using Cargo:

```bash
$ cargo new guessing_game
$ cd guessing_game

```

This generates a `Cargo.toml` file and a `src/main.rs` file.

### 2. Processing a Guess

To handle user input, you must import the `io` library from the standard library (`std`).

```rust
use std::io;

fn main() {
    println!("Guess the number!");
    println!("Please input your guess.");

    let mut guess = String::new(); // Create a mutable variable to store the string

    io::stdin()
        .read_line(&mut guess) // Read input and append to the string
        .expect("Failed to read line"); // Handle potential failure

    println!("You guessed: {guess}");
}

```

- **`let mut`**: Creates a **mutable** variable (variables are immutable by default).
- **`read_line`**: Takes a mutable reference (`&mut`) to the string buffer.
- **`expect`**: Handles the `Result` returned by `read_line`. If the result is an error, the program crashes with the specified message.

### 3. Generating a Secret Number

Rust’s standard library does not include random number generation, so you must use the external `rand` crate.

**Add Dependency:**
In `Cargo.toml`, add `rand` under `[dependencies]`:

```toml
[dependencies]
rand = "0.8.5"

```

After saving, running `cargo build` will download and compile the new dependency.

**Generate the Number:**

```rust
use rand::Rng; // Import the Rng trait

// ... inside main ...
let secret_number = rand::thread_rng().gen_range(1..=100);

```

- **`1..=100`**: A range expression usually used to generate numbers between 1 and 100 (inclusive).

### 4. Comparing the Guess

To compare the user's guess with the secret number, use the `std::cmp::Ordering` enum and a `match` expression.

```rust
use std::cmp::Ordering;

// ... inside main ...
match guess.cmp(&secret_number) {
    Ordering::Less => println!("Too small!"),
    Ordering::Greater => println!("Too big!"),
    Ordering::Equal => println!("You win!"),
}

```

### 5. Handling Type Mismatches

The `guess` variable starts as a `String`, but `secret_number` is a number type (inferred as `i32`). To compare them, you must convert `guess` into a number. Rust allows **shadowing**, meaning you can reuse the variable name `guess`.

```rust
// Shadow the previous 'guess' string with a new parsed number
let guess: u32 = guess.trim().parse().expect("Please type a number!");

```

- **`trim()`**: Removes whitespace and newlines (like the generic `\n` from pressing Enter).
- **`parse()`**: Parses the string into a number. The `: u32` annotation tells Rust specifically which numeric type we want.

### 6. Allowing Multiple Guesses (Looping)

To allow the user to keep guessing, wrap the main logic in a `loop`. Use `break` to exit when the user wins.

```rust
loop {
    // ... input and parsing logic ...

    match guess.cmp(&secret_number) {
        Ordering::Less => println!("Too small!"),
        Ordering::Greater => println!("Too big!"),
        Ordering::Equal => {
            println!("You win!");
            break; // Exit the loop
        }
    }
}

```

### 7. Handling Invalid Input

Instead of crashing when the user types non-numeric characters (via `expect`), you can handle the error gracefully using `match` on the result of `parse()`.

```rust
let guess: u32 = match guess.trim().parse() {
    Ok(num) => num,     // If successful, return the number
    Err(_) => continue, // If error, skip to the next iteration of the loop
};

```

### Final Complete Code

```rust
use std::io;
use std::cmp::Ordering;
use rand::Rng;

fn main() {
    println!("Guess the number!");

    let secret_number = rand::thread_rng().gen_range(1..=100);

    loop {
        println!("Please input your guess.");

        let mut guess = String::new();

        io::stdin()
            .read_line(&mut guess)
            .expect("Failed to read line");

        // Convert String to u32, handling non-number inputs
        let guess: u32 = match guess.trim().parse() {
            Ok(num) => num,
            Err(_) => continue,
        };

        println!("You guessed: {guess}");

        match guess.cmp(&secret_number) {
            Ordering::Less => println!("Too small!"),
            Ordering::Greater => println!("Too big!"),
            Ordering::Equal => {
                println!("You win!");
                break;
            }
        }
    }
}

```
