# Ephemeral 1v1 MVP

Simplified MVP focused on core game loop. No auth, no persistence, no ratings.

## User Flow

```
┌─────────────────────────────────────────────────────────────┐
│  Host opens app                                             │
│  ↓                                                          │
│  "Enter your name:" → [input] → "Create Game"               │
│  ↓                                                          │
│  Lobby: "Waiting for opponent..."                           │
│  Share link: yomitaisen.app/game/abc123                     │
│  ↓                                                          │
│  Guest opens link                                           │
│  ↓                                                          │
│  "Enter your name:" → [input] → "Join Game"                 │
│  ↓                                                          │
│  Both players see: "Game starting in 3... 2... 1..."        │
│  ↓                                                          │
│  [Game loop - 10 rounds]                                    │
│  ↓                                                          │
│  Results screen: Winner, scores, word review                │
│  ↓                                                          │
│  "Play Again?" (creates new game, both players join)        │
└─────────────────────────────────────────────────────────────┘
```

## Game ID

Short alphanumeric codes for easy sharing:
- 6 characters: `abc123`, `xy7k9m`
- Lowercase + digits, no ambiguous chars (0/O, 1/l)
- ~2 billion combinations, sufficient for ephemeral games

## Architecture

```
┌──────────────┐     WebSocket      ┌──────────────────────────────┐
│   Frontend   │◄──────────────────►│   Axum Server                │
│  (Vanilla)   │                    │                              │
└──────────────┘                    │  ┌────────────────────────┐  │
                                    │  │ Game Sessions          │  │
                                    │  │ DashMap<GameId,        │  │
                                    │  │   GameSession>         │  │
                                    │  └────────────────────────┘  │
                                    │              │               │
                                    │              ▼               │
                                    │  ┌────────────────────────┐  │
                                    │  │ Word Repository        │  │
                                    │  │ (SQLite, read-only)    │  │
                                    │  └────────────────────────┘  │
                                    └──────────────────────────────┘
```

## Player Identity

Ephemeral player for this MVP:

```rust
struct EphemeralPlayer {
    id: PlayerId,           // UUID, generated on connect
    display_name: String,   // User-provided, not unique
}
```

For future authenticated sessions:

```rust
trait PlayerIdentity {
    fn id(&self) -> PlayerId;
    fn display_name(&self) -> &str;
}

impl PlayerIdentity for EphemeralPlayer { ... }
impl PlayerIdentity for AuthenticatedUser { ... }  // Future
```

Game session uses `Box<dyn PlayerIdentity>` or generic `P: PlayerIdentity` so the core game logic is identity-agnostic.

## WebSocket Messages

### Client → Server

```rust
enum ClientMessage {
    // Lobby
    CreateGame { player_name: String },
    JoinGame { game_id: String, player_name: String },

    // Game
    Answer { answer: String },

    // Post-game
    PlayAgain,
    LeaveGame,
}
```

### Server → Client

```rust
enum ServerMessage {
    // Lobby
    GameCreated { game_id: String },
    WaitingForOpponent,
    OpponentJoined { opponent_name: String },
    GameFull,
    GameNotFound,

    // Game
    GameStarting { countdown_seconds: u8 },
    RoundStart { round: u8, total_rounds: u8, kanji: String },
    AnswerResult { correct: bool, correct_reading: String },
    RoundEnd { winner: Option<String>, scores: HashMap<String, u32> },
    GameEnd { winner: Option<String>, final_scores: HashMap<String, u32> },

    // Connection
    OpponentDisconnected,
    OpponentReconnected,

    // Errors
    Error { message: String },
}
```

## State Machine

```
          CreateGame
              │
              ▼
┌─────────────────────────┐
│   WaitingForOpponent    │◄────── (timeout: expire game)
└───────────┬─────────────┘
            │ JoinGame
            ▼
┌─────────────────────────┐
│      Countdown          │
└───────────┬─────────────┘
            │ 3... 2... 1...
            ▼
┌─────────────────────────┐
│      RoundActive        │◄──┐
└───────────┬─────────────┘   │
            │ answer/timeout  │
            ▼                 │
┌─────────────────────────┐   │
│    RoundComplete        │───┘ (if more rounds)
└───────────┬─────────────┘
            │ (all rounds done)
            ▼
┌─────────────────────────┐
│       GameOver          │
└───────────┬─────────────┘
            │ PlayAgain / LeaveGame
            ▼
        (new game or cleanup)
```

## Implementation Tasks

### Phase 1: Core Loop
- [ ] Game ID generation (short codes)
- [ ] Game session state machine
- [ ] Create game endpoint
- [ ] Join game endpoint
- [ ] Round management (start, timeout, end)
- [ ] Answer checking
- [ ] Score calculation
- [ ] Game end and results

### Phase 2: Frontend
- [ ] Landing page with name input + create game
- [ ] Join page (from shared link)
- [ ] Lobby waiting screen
- [ ] Game screen (kanji display, input, timer)
- [ ] Results screen
- [ ] Play again flow

### Future Enhancements (not in MVP)
- [ ] Spectator mode (third+ person watches live)
- [ ] Reconnection handling (pause game, wait for player)
- [ ] Lobby timeout (expire after 10 min if no opponent)
- [ ] Rematch with same opponent

## Design Principles

1. **Identity-agnostic game logic** - Core game session works with any `PlayerIdentity` impl
2. **No persistence required** - All state in memory, games are ephemeral
3. **Stateless frontend** - All state derived from WebSocket messages, URL contains game ID
4. **Easy to layer auth later** - Just add `AuthenticatedUser` implementing `PlayerIdentity`
