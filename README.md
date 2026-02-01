<p align="center">
  <img src="frontend/logo.svg" alt="Yomitaisen" width="300">
</p>

# 読み対戦 (Yomitaisen)

A real-time multiplayer kanji reading quiz game. Two players compete to type the correct reading (in hiragana) of displayed kanji words. First to 10 wins!

## Current Features

- **Real-time 1v1 gameplay** via WebSocket
- **Casual games** with shareable 6-character game codes (no account required)
- **15-second round timer** with automatic timeout handling
- **First to 10 wins** the match
- **Skip mechanics** - both players must agree to skip a round
- **Rematch functionality** - play again against the same opponent
- **Sound effects** for wins/losses
- **Disconnect handling** - notifies opponent and cleans up game state
- **Romaji input support** - converts to hiragana automatically (via wanakana.js)

## Architecture

```
┌─────────────────────────────────────────────────┐
│  Transport Layer (WebSocket)                    │
│  - Generic connection handler                   │
│  - Casual (ephemeral) + Ranked (matchmaking)   │
└──────────────────┬──────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────┐
│  Game Engine Layer                              │
│  - ActiveGame: session + broadcast channels     │
│  - GameRegistry: coordination & timeout mgmt    │
└──────────────────┬──────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────┐
│  Core Domain Layer (pure logic, no I/O)         │
│  - GameSession: state machine                   │
│  - Word, Round, RoundOutcome types              │
│  - Type-safe message protocol                   │
└─────────────────────────────────────────────────┘
```

## Tech Stack

| Layer | Technology | Purpose |
|-------|------------|---------|
| Backend | Rust + Axum 0.7 | Type-safe, high-performance async web framework |
| Database | SQLite + sqlx | In-process DB with compile-time checked queries |
| Concurrency | DashMap + tokio::broadcast | Thread-safe game state + pub/sub messaging |
| Frontend | Vanilla HTML/CSS/JS | MVP playground (will be rebuilt later) |
| Input | wanakana.js | Romaji → hiragana conversion |

## Project Structure

```
yomitaisen/
├── backend/
│   ├── src/
│   │   ├── main.rs              # Entry point, wiring
│   │   ├── lib.rs               # Router, AppState
│   │   ├── config.rs            # Environment config
│   │   └── game/
│   │       ├── core/            # Pure domain logic
│   │       │   ├── session.rs   # Game state machine
│   │       │   ├── messages.rs  # Protocol types
│   │       │   └── word*.rs     # Word types & repository
│   │       ├── engine/          # Game coordination
│   │       │   ├── active_game.rs
│   │       │   ├── registry.rs
│   │       │   └── ws.rs        # Generic WS handler
│   │       ├── ephemeral/       # Casual mode (game codes)
│   │       └── matchmaking/     # Authenticated mode (future)
│   └── migrations/
├── frontend/
│   └── index.html               # Single-file MVP app
├── docker-compose.yml
└── Caddyfile
```

## Running Locally

```bash
# Backend
cd backend
cp .env.example .env
cargo run

# Frontend (open in browser)
open frontend/index.html
```

Or with Docker:

```bash
docker compose up --build
```

## Development Status

**Phase 1 (Foundation)** - ✅ Complete
- Basic game loop with first-to-10 scoring
- WebSocket communication
- Casual games with shareable codes
- Core mechanics (skip, timeout, rematch, disconnect handling)
- 40k word database imported

**Current Focus: Ephemeral Mode Polish**
- Game configuration (difficulty, rounds)
- Public game lobby
- Word filtering (skip unsuitable words)
- Reconnection handling

**Future Phases**
- User authentication & profiles
- Rating system (Glicko-2)
- Ranked matchmaking

See [TODO.md](TODO.md) for detailed roadmap.
