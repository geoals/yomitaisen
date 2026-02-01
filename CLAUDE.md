# Claude Code Guidelines for KanjiQuiz

## Development Workflow

This project uses agent-driven development with active human steering. Follow these principles:

### 1. Small, Atomic Units of Work

- One function, one test, one module at a time
- Never implement multiple concerns in a single pass
- If a task feels like it has multiple parts, stop and break it down first
- Prefer many small PRs/commits over large changes

### 2. Test-Driven Development (TDD)

Follow the red-green-refactor cycle strictly:

1. **Propose test first** - Before writing any implementation, propose the test that specifies the behavior
2. **Get approval** - Wait for confirmation that the test captures the intended behavior
3. **Write the test** - Implement the test (it should fail)
4. **Implement minimally** - Write just enough code to make the test pass
5. **Refactor if needed** - Clean up while keeping tests green

Never write implementation code without a corresponding test. Tests are the specification.

**Bugfixes follow TDD too:**
1. Write a failing test that reproduces the bug
2. Verify it fails for the expected reason
3. Fix the bug
4. Verify the test passes

### 3. Propose Before Implementing

For any non-trivial work:

1. Explain the approach in plain language
2. Show the interface/types/function signatures
3. Wait for approval before writing the implementation
4. If uncertain between approaches, present options with tradeoffs

Do NOT:
- Silently make architectural decisions
- Add "nice to have" code that wasn't discussed
- Refactor unrelated code while implementing a feature

### 4. Explain Choices and Teach

The goal is learning, not just shipping. When writing code:

- **Explain why** - Not just what the code does, but why this approach over alternatives
- **Name the concepts** - If using a pattern (e.g., "this is the repository pattern"), name it so it can be researched
- **Show alternatives** - When there are multiple valid approaches, briefly mention what they are and why this one was chosen
- **Explain non-obvious APIs** - If using something like `tower::ServiceExt::oneshot()`, explain what it does and when you'd use it
- **Link to docs** - When introducing a new crate or concept, mention where to learn more

Examples of good explanations:
- "Using `EnvFilter::from_default_env()` which reads RUST_LOG. Alternative: hardcoded filter, but env-based is more flexible for debugging."
- "This is dependency injection via traits—the handler takes `impl UserRepository` so we can swap in a mock for tests."

Don't over-explain obvious things, but err on the side of teaching when introducing new patterns or APIs.

### 5. Clean Code Principles

**High cohesion, low coupling:**
- Each module should have a single, clear responsibility
- Depend on abstractions (traits), not concretions
- Use dependency injection for testability
- Keep functions small and focused (< 20 lines ideal)

**Separation of concerns:**
- Domain logic should not know about HTTP/WebSocket
- Database access through repository traits
- No business logic in handlers

**Naming:**
- Names should reveal intent
- No abbreviations unless universally understood
- Test names describe the behavior being tested: `test_rejects_invalid_reading`

**Idiomatic Rust:**
- Early returns with `let ... else` over nested `if let`:
  ```rust
  let Some(x) = opt else { return; };
  ```
- `?` operator for Option/Result chaining
- Pattern matching with guards over if-else chains:
  ```rust
  match player_id {
      id if id == self.player1 => self.scores.0 += 1,
      id if id == self.player2 => self.scores.1 += 1,
      _ => {}
  }
  ```
- Named structs over tuples when returning multiple values
- Enums with struct variants for results with context

### 6. Code Structure

```
backend/
├── src/
│   ├── main.rs           # Entry point, wiring
│   ├── config.rs         # Configuration loading
│   ├── lib.rs            # Router, re-exports
│   └── game/
│       ├── core/         # Shared types (Word, WordRepository)
│       └── duel/         # 1v1 mode (matchmaking, session, ws_handler)
└── tests/
    └── *.rs              # Integration tests

frontend/
└── index.html            # Vanilla JS UI
```

Feature-based organization: each feature owns its domain types, data access, and handlers.

### 7. When to Ask vs. Proceed

**Ask first:**
- Module boundaries and interfaces
- Database schema changes
- New dependencies
- Architectural patterns
- Anything with multiple valid approaches

**Proceed directly:**
- Implementing an approved interface
- Writing tests for agreed behavior
- Bug fixes with obvious solutions
- Formatting/linting fixes

### 8. Commit Style

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `refactor`: Code restructuring (no behavior change)
- `docs`: Documentation only
- `test`: Adding/updating tests
- `chore`: Build, deps, config, tooling

**Scopes:** `game`, `duel`, `frontend`, `config`, `db`, or omit for broad changes

**Examples:**
```
feat(duel): add answer checking and round result
refactor(game): separate game logic from WebSocket handling
fix(frontend): use snake_case for message types
docs: update code structure in CLAUDE.md
```

Keep commits small and focused. One logical change per commit.

## Tech Stack Quick Reference

- **Backend:** Rust + Axum + SQLite (sqlx)
- **Auth:** argon2 + JWT
- **Async:** Tokio
- **Game state:** DashMap + tokio::broadcast
- **Testing:** `#[tokio::test]`, `sqlx::test` for DB tests

## Current Phase

Phase 1: Foundation (see plan.md for details)
