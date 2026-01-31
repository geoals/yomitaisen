# KanjiQuiz - Multiplayer Kanji Reading Game

A real-time multiplayer quiz game where players race to type the correct reading of Japanese kanji/words. Features ranked matchmaking, ELO-style ratings, and adaptive difficulty based on word frequency.

## Core Concept

- Show a kanji/word, players race to type the reading (hiragana)
- First correct answer wins points
- Points scale by speed + word difficulty
- Competitive ranking system (Glicko-2)

## Core Game Loop

```
┌─────────────────────────────────────────────────────┐
│  Round starts: 漢字 appears                          │
│  ↓                                                   │
│  Players race to type reading (かんじ)               │
│  ↓                                                   │
│  First correct answer wins points                    │
│  Points scale by: speed + word difficulty            │
│  ↓                                                   │
│  After X rounds → match ends → ELO adjusts           │
└─────────────────────────────────────────────────────┘
```

## Difficulty System

Based on word frequency rank from a corpus (like BCCWJ or Netflix frequency list):
For the MVP we should base it on one dict from https://github.com/MarvNC/yomitan-dictionaries and one frequency dict

| Tier         | Frequency Rank | Example        | Point Multiplier |
| ------------ | -------------- | -------------- | ---------------- |
| Common       | 1-1000         | 時間、人、今日 | 1x               |
| Standard     | 1001-5000      | 届ける、比較   | 1.5x             |
| Intermediate | 5001-15000     | 憂鬱、曖昧     | 2x               |
| Advanced     | 15001-30000    | 齟齬、杜撰     | 3x               |
| Obscure      | 30000+         | 魑魅魍魎       | 5x               |

## Game Modes

### 1. Ranked 1v1

- Matchmaking by ELO
- 10-15 rounds
- Words selected near both players' skill level
- Winner gains ELO, loser loses

### 2. Arena (FFA 4-8 players)

- Elimination or point-based
- Word difficulty ramps up each round
- Last standing or highest score wins
- Smaller ELO swings, more variance

### 3. Survival (Solo but async competitive)

- Endless mode, 3 lives
- Miss = lose a life
- Leaderboard by score/streak
- Daily/weekly challenges

### 4. Custom Rooms

- Friends only, configurable rules
- Filter by JLPT level, specific kanji sets, themes

## Ranking System

Using **Glicko-2** (better than basic ELO):

- Handles rating uncertainty (new players stabilize faster)
- Accounts for volatility (inconsistent players)
- Used by Lichess, works well for 1v1 games

```
Starting Rating: 1500

Tiers:
  Bronze:   0-1199
  Silver:   1200-1499
  Gold:     1500-1799
  Platinum: 1800-2099
  Diamond:  2100-2399
  Master:   2400+
```

## Technical Architecture

### MVP (Single Machine)

```
┌──────────────┐     WebSocket      ┌──────────────────────────────┐
│   Frontend   │◄──────────────────►│   Axum Server                │
│   (React)    │                    │                              │
└──────────────┘                    │  ┌────────────────────────┐  │
                                    │  │ In-memory game state   │  │
                                    │  │ (DashMap + tokio       │  │
                                    │  │  broadcast channels)   │  │
                                    │  └────────────────────────┘  │
                                    │              │               │
                                    │              ▼               │
                                    │  ┌────────────────────────┐  │
                                    │  │ SQLite (WAL mode)      │  │
                                    │  │ users, words, matches  │  │
                                    │  └────────────────────────┘  │
                                    └──────────────────────────────┘
```

### Scaled (Multi-Pod / Learning K8s)

```
┌─────────────────────────────────────────────────────────────────┐
│  Kubernetes Cluster (GKE)                                       │
│                                                                 │
│  ┌─────────────┐  ┌─────────────┐                              │
│  │ Game Pod 1  │  │ Game Pod 2  │     (Axum + WebSocket)       │
│  └──────┬──────┘  └──────┬──────┘                              │
│         │                │                                      │
│         └───────┬────────┘                                      │
│                 │                                               │
│         ┌───────▼────────┐                                      │
│         │ Valkey/Redis   │  ← Pub/sub for cross-pod events     │
│         └───────┬────────┘                                      │
│                 │                                               │
│         ┌───────▼────────┐                                      │
│         │  PostgreSQL    │  ← Shared persistent state          │
│         └────────────────┘                                      │
└─────────────────────────────────────────────────────────────────┘
```

## Tech Stack

| Layer                | Tech                | Why                                                |
| -------------------- | ------------------- | -------------------------------------------------- |
| Frontend             | React + TypeScript  | Standard, good WS libraries                        |
| Realtime             | Native WebSocket    | Axum has built-in WS support via tokio-tungstenite |
| Backend              | Rust + Axum         | Fast, memory-safe, great async story with Tokio    |
| Database (MVP)       | SQLite + WAL mode   | In-process, no network hop, fast for single server |
| Database (Scaled)    | PostgreSQL          | Multiple writers, horizontal scaling               |
| Cache/State (MVP)    | In-memory (DashMap) | No external deps, tokio::broadcast for pub/sub     |
| Cache/State (Scaled) | Valkey/Redis        | Cross-pod pub/sub, shared ephemeral state          |
| Auth                 | argon2 + JWT        | Roll own for MVP, simple and sufficient            |

## Architecture Decisions

**Monolith first, split later.** No separate matchmaking service until pain points emerge:

- Avoids distributed transaction complexity
- Simpler deployment and local dev
- Split candidates for later: background workers (rating calc, stats rollup)

**SQLite for MVP because:**

- No network latency (in-process)
- WAL mode handles concurrent reads well
- Survives restarts (unlike pure in-memory)
- Easy migration path to PostgreSQL when needed

**No Redis for single machine:**

- Use `tokio::sync::broadcast` for in-memory pub/sub
- Use `DashMap` for concurrent game state
- Add Valkey/Redis only when scaling to multiple pods

## Deployment Options

| Environment           | Stack                        | Purpose            |
| --------------------- | ---------------------------- | ------------------ |
| Local dev             | Single binary + SQLite       | Fast iteration     |
| Home server (Coolify) | Single binary + SQLite       | Production MVP     |
| GKE (learning)        | 2 pods + PostgreSQL + Valkey | Learn k8s patterns |

## Rust Dependencies

```toml
[dependencies]
# Web framework
axum = { version = "0.7", features = ["ws"] }
tokio = { version = "1", features = ["full"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }

# Database
sqlx = { version = "0.7", features = ["runtime-tokio", "sqlite"] }

# Auth
argon2 = "0.5"
jsonwebtoken = "9"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Utilities
uuid = { version = "1", features = ["v4", "serde"] }
dashmap = "5"                    # Concurrent HashMap for game state
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1"
tracing = "0.1"
tracing-subscriber = "0.3"

# Rating system
glicko2 = "1"                    # Or implement manually
```

## Database Schema

SQLite-compatible schema for MVP. Notes:

- Use TEXT for UUIDs (generate in application with `uuid` crate)
- Use JSON instead of JSONB (SQLite's JSON1 extension)
- Use TEXT with comma separation or JSON arrays instead of `TEXT[]`
- Use INTEGER for timestamps (Unix epoch) or TEXT (ISO 8601)

### words

```sql
CREATE TABLE words (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  kanji TEXT NOT NULL,
  readings TEXT NOT NULL,            -- JSON array: ["かんじ", "かん"]
  frequency_rank INTEGER NOT NULL,
  jlpt_level INTEGER,                -- 5, 4, 3, 2, 1 (or NULL)
  difficulty_tier TEXT NOT NULL,     -- common, standard, intermediate, advanced, obscure
  tags TEXT,                         -- JSON array: ["noun", "jlpt3"]
  created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX idx_words_frequency ON words(frequency_rank);
CREATE INDEX idx_words_difficulty ON words(difficulty_tier);
CREATE INDEX idx_words_jlpt ON words(jlpt_level);
```

### users

```sql
CREATE TABLE users (
  id TEXT PRIMARY KEY,               -- UUID generated in app
  username TEXT UNIQUE NOT NULL,
  email TEXT UNIQUE,
  password_hash TEXT,

  -- Glicko-2 rating fields
  rating REAL DEFAULT 1500,
  rating_deviation REAL DEFAULT 350,
  volatility REAL DEFAULT 0.06,

  -- Stats
  games_played INTEGER DEFAULT 0,
  games_won INTEGER DEFAULT 0,

  created_at TEXT DEFAULT (datetime('now')),
  last_active_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX idx_users_rating ON users(rating DESC);
```

### matches

```sql
CREATE TABLE matches (
  id TEXT PRIMARY KEY,               -- UUID generated in app
  mode TEXT NOT NULL,                -- ranked_1v1, arena, survival

  -- Game data (JSON)
  rounds TEXT NOT NULL,              -- JSON: [{word_id, winner_id, time_ms, ...}, ...]
  final_scores TEXT NOT NULL,        -- JSON: {player_id: score, ...}
  winner_id TEXT,

  -- Rating changes
  rating_changes TEXT,               -- JSON: {player_id: {before, after, delta}, ...}

  started_at TEXT NOT NULL,
  ended_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX idx_matches_ended ON matches(ended_at DESC);
```

### match_players (junction table for players in a match)

```sql
CREATE TABLE match_players (
  match_id TEXT NOT NULL REFERENCES matches(id),
  user_id TEXT NOT NULL REFERENCES users(id),
  PRIMARY KEY (match_id, user_id)
);

CREATE INDEX idx_match_players_user ON match_players(user_id);
```

### user_word_stats

```sql
CREATE TABLE user_word_stats (
  user_id TEXT NOT NULL REFERENCES users(id),
  word_id INTEGER NOT NULL REFERENCES words(id),

  times_seen INTEGER DEFAULT 0,
  times_correct INTEGER DEFAULT 0,
  times_first INTEGER DEFAULT 0,     -- Times answered first in multiplayer
  avg_response_ms INTEGER,
  best_response_ms INTEGER,

  last_seen_at TEXT,

  PRIMARY KEY (user_id, word_id)
);
```

## WebSocket Message Protocol

Using serde for JSON serialization. Tagged enum pattern for type-safe messages.

### Client → Server

```rust
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    QueueJoin { mode: GameMode },
    QueueLeave,
    Answer { round_id: String, answer: String },
    RoomJoin { room_code: String },
    RoomCreate { settings: RoomSettings },
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum GameMode {
    Ranked1v1,
    Arena,
    Survival,
}
```

### Server → Client

```rust
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMessage {
    MatchStart {
        match_id: String,
        players: Vec<Player>,
        settings: MatchSettings,
    },
    RoundStart {
        round_id: String,
        word: WordDisplay,  // { id, kanji } - no reading exposed
        round_num: u32,
    },
    RoundEnd {
        winner_id: Option<String>,
        correct_reading: String,
        times: HashMap<String, u64>,  // player_id -> ms
    },
    MatchEnd {
        final_scores: HashMap<String, u32>,
        rating_changes: HashMap<String, RatingChange>,
    },
    QueueStatus {
        position: u32,
    },
    Error {
        message: String,
    },
}
```

## Features

### MVP (v1)

- [ ] User registration/login
- [ ] Basic 1v1 ranked matchmaking
- [ ] Word database with frequency data (start with top 10k words)
- [ ] Real-time game with WebSockets
- [ ] Simple ELO/Glicko-2 rating
- [ ] Match history
- [ ] Basic leaderboard

### v2

- [ ] Arena mode (4-8 players)
- [ ] Survival mode with daily leaderboard
- [ ] Custom rooms with invite links
- [ ] Post-match word review
- [ ] Personal word stats and weak areas
- [ ] JLPT level filtering

### v3

- [ ] Spectator mode
- [ ] Match replays
- [ ] Seasonal rankings with rewards
- [ ] Friend system
- [ ] Study list integration (export to Anki)
- [ ] Achievement system
- [ ] Adaptive difficulty (pick words at edge of skill)

## Word Data Sources

Potential sources for frequency-ranked vocabulary:

- [Wikipedia frequency list](https://en.wiktionary.org/wiki/Wiktionary:Frequency_lists/Japanese)
- Netflix frequency list (Anime/Drama corpus)
- BCCWJ (Balanced Corpus of Contemporary Written Japanese)
- JMdict for readings and definitions
- JLPT vocabulary lists for level tagging

## Reading Validation

Accept multiple valid readings:

- 今日 → きょう, こんにち (context-dependent, accept both)
- 生 → なま, せい, しょう, いきる, etc.

Normalization:

- Convert romaji input to hiragana
- Handle long vowels (ou → おう, ō → おう)
- Trim whitespace

## Scoring Formula

```
base_points = 100
difficulty_multiplier = { common: 1, standard: 1.5, intermediate: 2, advanced: 3, obscure: 5 }
speed_bonus = max(0, (time_limit - response_time) / time_limit * 50)

round_score = base_points * difficulty_multiplier + speed_bonus
```

Only the first correct answer gets points (or partial points for 2nd/3rd in arena mode).

## Development Phases

### Phase 1: Foundation

1. Set up project structure
   - Rust workspace: `backend/` (Axum) + `frontend/` (React)
2. SQLite schema + migrations (sqlx with compile-time checked queries)
3. Basic auth: argon2 password hashing + JWT (jsonwebtoken crate)
4. Import word dataset (parse JMdict or frequency list)
5. Simple single-player quiz (REST API, no WS yet)

### Phase 2: Multiplayer Core

1. WebSocket server with Axum (`axum::extract::ws`)
2. In-memory game state (`DashMap<MatchId, GameRoom>`)
3. Room pub/sub with `tokio::sync::broadcast`
4. Basic 1v1 game loop
5. Answer validation (hiragana normalization, multiple readings)
6. Match persistence to SQLite

### Phase 3: Ranking & Matchmaking

1. Implement Glicko-2 (or use `glicko2` crate)
2. Matchmaking queue (in-memory, match by rating proximity)
3. Rating updates after matches
4. Leaderboard endpoint

### Phase 4: Polish & Modes

1. Arena mode
2. Survival mode
3. Custom rooms
4. UI polish
5. Mobile responsiveness

### Phase 5 (Optional): Scale to K8s

1. Migrate SQLite → PostgreSQL
2. Add Valkey/Redis for cross-pod pub/sub
3. Deploy to GKE with 2 pods
4. Add WebSocket sticky sessions (Ingress annotation)
5. Health checks and graceful shutdown
