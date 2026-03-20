---
description: Generate a new Cargo project for a Codewars challenge based on the title.
context: fork
---


Generate a new Cargo project for a Codewars challenge based on the title.

The user will provide a challenge title as the argument (e.g., "Return the first M multiples of N").

Your task is to:

1. Convert the challenge title into snake_case format suitable for a Rust crate name with prefix the latest challenge
   number + 1.

- Example: "Return the first M multiples of N" -> "001_return_the_first_m_multiples_of_n"
- Use only lowercase letters, numbers, and underscores.

2. Execute the command `cargo new <crate_name> --bin` to create the project directory.
3. Create a `README.md` file inside the new project directory with the challenge title as the header.

- Content: `# <Original Challenge Title>\n\nLink to challenge: ...` (leave placeholder or empty if no link provided).
- Do NOT solve the challenge.

4. Update the `README.md` file in the current directory (`codewars/README.md`) to include the new challenge in a list.

- If the file does not exist, create it with the header `# Codewars Challenges`.
- Append a line: `- [<Original Challenge Title>](./<crate_name>)`

5. Confirm to the user that the project has been created at `<crate_name>/` and added to the catalog.

Run the commands sequentially.

$ARGUMENTS
