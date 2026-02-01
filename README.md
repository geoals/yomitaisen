<p align="center">
  <img src="frontend/logo.svg" alt="Yomitaisen" width="300">
</p>

# 読み対戦 (Yomitaisen)

A real-time multiplayer kanji reading quiz game. Two players compete to type the correct reading (in hiragana) of displayed kanji words.

## Features

- Real-time 1v1 matchmaking via WebSocket
- 15-second round timer
- Best of 3 rounds
- Disconnect handling

## Tech Stack

- **Backend:** Rust, Axum, SQLite
- **Frontend:** Vanilla HTML/CSS/JS
- **Real-time:** WebSocket with tokio

## Running

```bash
# Backend
cd backend
cargo run

# Open frontend
open frontend/index.html
```
