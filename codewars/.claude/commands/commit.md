---
description: Generate a commit message, show it, then commit
allowed-tools: Bash(git add:*), Bash(git status:*), Bash(git diff:*), Bash(git log:*), Bash(git commit:*)
context: fork
---

## Context

- Current git status: !`git status`
- Staged and unstaged changes in codewars: !`git diff HEAD -- codewars/`
- Current branch: !`git branch --show-current`
- Recent commits (for style reference): !`git log --oneline -5`

## Your task

### 1. Generate the commit message

Based on the changes above, write a conventional commit message following this format and don't use co-author by Claude:

```
<type>(<scope>): <short summary>

<optional body explaining what and why>
```

Types: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`

### 2. Show the commit message

Display the proposed commit message to the user in a code block clearly labeled "Proposed commit message:".

### 3. Stage and commit

Stage only changes inside the `codewars/` directory with `git add codewars/`, then commit using the generated message.
