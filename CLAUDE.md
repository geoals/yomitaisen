# Yomitaisen (読み対戦)

A multiplayer kanji reading quiz game. Two players compete to identify the correct hiragana reading of Japanese kanji words.

## Game Flow (Ephemeral Mode)

1. **Create/Join**: Player 1 creates a game and shares the game ID. Player 2 joins with the ID.
2. **Rounds**: Each round shows a kanji word with possible readings. Players race to type the correct reading.
3. **Round End**: First correct answer wins the round. If both players skip, or 30s timeout, no one wins.
4. **Game End**: First to 10 wins, or highest score after 30 rounds. Tie possible.
5. **Rematch**: Both players must agree to rematch.

Future game modes (matchmaking, user accounts) are planned but not yet active.

## Tech Stack

- **Backend:** Rust + Axum + SQLite (sqlx)
- **Frontend:** Vanilla JS (single HTML file)
- **Async:** Tokio
- **Game state:** In-memory (DashMap) with WebSocket communication
- **Testing:** `#[tokio::test]`, integration tests in `backend/tests/`

## Code Structure

```
backend/
├── src/
│   ├── main.rs              # Entry point
│   ├── config.rs            # Configuration
│   ├── lib.rs               # Router, app setup
│   └── game/
│       ├── core/            # Shared: Word, Session, Messages
│       ├── engine/          # ActiveGame, Registry, round logic
│       ├── ephemeral/       # Create-game flow (current)
│       └── matchmaking/     # Quick-match flow (future)
└── tests/
    ├── common/              # Test utilities
    └── *_tests.rs           # Integration tests by feature

frontend/
└── index.html               # UI

tools/seed/                  # Database seeding tool (imports kanji data)
```

## Rust Conventions

Idiomatic patterns used in this codebase:

- `let ... else` for early returns over nested `if let`
- `?` for Option/Result chaining
- Pattern matching with guards over if-else chains
- Named structs over tuples for return values
- Enums with struct variants for results with context

## Commit Scopes

`game`, `frontend`, `config`, `db`, `matchmaking`, `ephemeral`, `engine`

## Running

```bash
cd backend && cargo run    # Start server
cargo test                 # Run tests
```
