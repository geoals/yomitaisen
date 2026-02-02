# Security Review - Yomitaisen Kanji Quiz

**Date:** 2026-02-02
**Status:** Early Development (MVP)
**Recommendation:** Do NOT deploy to production without addressing Phase 1 and Phase 2 items

---

## Summary

| Severity | Count |
|----------|-------|
| Critical | 3 |
| High | 12 |
| Medium | 15 |
| Low | 2 |
| Good Practice | 1 |

---

## Critical Findings

### 1.1 No Authentication Implemented
- **Files:** `backend/src/lib.rs`, `backend/src/game/matchmaking/ws_handler.rs`
- **Lines:** lib.rs:56-57, matchmaking/ws_handler.rs:17-20
- **Issue:** The system accepts `user_id` as a simple string from any client with no validation. Any user can impersonate any other user by sending a `Join` message with any `user_id`.
- **Impact:** Complete authentication bypass - users can access games belonging to other players, manipulate scores, engage in account takeover
- **Fix:** Implement JWT-based authentication with token validation on WebSocket handshake

### 4.1 No WebSocket Authentication/Authorization
- **File:** `backend/src/lib.rs:26-40`
- **Issue:** WebSocket endpoints accept connections without any authentication checks
- **Impact:** Any client can connect and create arbitrary games
- **Fix:** Extract/validate JWT token from WebSocket handshake, implement origin validation

### 8.1 Invalid Rust Edition
- **File:** `backend/Cargo.toml:4`
- **Issue:** `edition = "2024"` - Rust editions are: 2015, 2018, 2021. There is no 2024 edition.
- **Impact:** Build may fail or use unexpected behavior
- **Fix:** Change to `edition = "2021"`

---

## High Severity Findings

### 1.2 User Table Created But Unused
- **File:** `backend/migrations/001_create_users.sql:1-28`
- **Issue:** Schema includes `users` and `local_credentials` tables but no authentication logic uses them
- **Fix:** Either implement full auth system or remove unused tables

### 2.1 No Validation on Player Names
- **Files:** `backend/src/game/ephemeral/ws_handler.rs:17`, `backend/src/game/matchmaking/ws_handler.rs:17`
- **Issue:** Player names accepted without length limits, character validation, or XSS protection
- **Impact:** Memory exhaustion, stored XSS, DOM-based XSS
- **Fix:** Enforce max length (50 chars), whitelist allowed characters, sanitize before echoing

### 4.2 No Rate Limiting on WebSocket Messages
- **File:** `backend/src/game/engine/ws.rs:74-98`
- **Issue:** Receive loop processes all incoming messages without rate limits
- **Attack Vector:** Brute force game codes or submit infinite answers
- **Fix:** Implement token bucket rate limiter (e.g., 10 msg/sec max)

### 5.1 Raw Player Names Used as User IDs
- **File:** `backend/src/game/ephemeral/state.rs:59-66`
- **Issue:** Display names used directly as player identifiers, visible in logs and broadcasts
- **Fix:** Use UUIDs as internal identifiers, store display names separately

### 6.1 No CORS Configuration
- **File:** `backend/src/lib.rs:42-59`
- **Issue:** Router has no CORS middleware
- **Impact:** Any origin can connect to WebSocket endpoints
- **Fix:** Add tower-http CORS middleware with allowed origins

### 6.2 No WebSocket Origin Validation
- **File:** `backend/src/lib.rs:26-40`
- **Issue:** WebSocket upgrade handler doesn't validate Origin header
- **Attack:** Cross-site WebSocket hijacking (CSWSH)
- **Fix:** Validate Origin header before upgrade

### 7.1 No Rate Limiting on Game Code Guessing
- **File:** `backend/src/game/ephemeral/state.rs:51-92`
- **Issue:** Game join accepts any code without rate limiting
- **Attack:** Brute force 6-character codes (31^6 ≈ 887 million possibilities)
- **Fix:** Limit join attempts per IP (5/minute), implement exponential backoff

### 7.2 No Rate Limiting on WebSocket Connections
- **File:** `backend/src/lib.rs:26-40`
- **Issue:** No connection throttling per IP address
- **Attack:** Create thousands of connections to exhaust resources
- **Fix:** Limit to N connections per IP (e.g., 10)

### 8.3 Missing Security-Related Dependencies
- **File:** `backend/Cargo.toml`
- **Missing:** `jsonwebtoken`, `tower-http` (rate limiting/CORS), `argon2` (unused but schema exists)
- **Fix:** Add security dependencies to implementation plan

### 9.1 Critical `.unwrap()` in Production Code
- **Files:** `main.rs:21,26,30-31`, `matchmaking/state.rs:61`
- **Issue:** Panics crash the entire service (port conflict, opponent channel missing)
- **Fix:** Return `Result` and handle gracefully, log error and attempt recovery

### 11.1 DOM-Based XSS from Player Names
- **File:** `frontend/index.html:623,702`
- **Issue:** Opponent names inserted into DOM without sanitization
- **Attack:** `<img src=x onerror=alert('XSS')>`
- **Fix:** Use `textContent` instead of innerHTML, or sanitize with DOMPurify

### 12.2 No Secrets Management
- **Issue:** When auth is implemented, JWT_SECRET must be properly managed
- **Fix:** Use secrets management (Docker secrets, Vault), never hardcode

---

## Medium Severity Findings

### 2.2 No Validation on Game Codes
- **File:** `backend/src/game/ephemeral/game_id.rs:13-22`
- **Issue:** Input validation only checks length, not charset
- **Fix:** Validate input matches expected charset: `^[abcdefghjkmnpqrstuvwxyz23456789]{6}$`

### 2.3 No Validation on Answers
- **File:** `backend/src/game/core/word_repository.rs:29-38`
- **Issue:** No length limits or rate limiting on answer attempts
- **Fix:** Enforce max length (50 chars), implement per-player rate limiting

### 4.3 Broadcast Channel Buffer Overflow Risk
- **File:** `backend/src/game/engine/ws.rs:44`
- **Issue:** Hardcoded capacity of 16, could cause message drops
- **Fix:** Use configured buffer size, add monitoring for dropped messages

### 4.4 No Validation of Message Types
- **Files:** `backend/src/game/ephemeral/ws_handler.rs:73-78`, `backend/src/game/matchmaking/ws_handler.rs:43-48`
- **Issue:** Wrong message types warn but don't disconnect
- **Fix:** Disconnect client after N invalid message types

### 5.2 Logging of Raw Answer Attempts
- **File:** `backend/src/game/engine/registry.rs:78-79`
- **Issue:** Answer attempts logged with unencrypted values at DEBUG level
- **Fix:** Only log metadata (correct/incorrect), use trace level for details

### 5.3 Configuration Uses Default/Insecure Paths
- **File:** `backend/src/config.rs:15-16`
- **Issue:** SQLite `data.db` with default permissions
- **Fix:** Use absolute paths with restricted permissions (600)

### 5.4 Overly Verbose Error Messages
- **File:** `backend/src/game/ephemeral/ws_handler.rs:75-77`
- **Issue:** Error messages may reveal system state
- **Fix:** Use generic error messages, log details server-side

### 8.2 Dependency Version Pinning Issues
- **File:** `backend/Cargo.toml:6-32`
- **Issue:** Flexible version ranges may accept breaking changes
- **Fix:** Specify min/max versions, run `cargo audit` regularly

### 9.2 Serde Unwraps in WebSocket Handler
- **File:** `backend/src/game/engine/ws.rs:50`
- **Issue:** Serialization failure crashes connection
- **Fix:** Handle error gracefully, close connection cleanly

### 10.1 No Connection Limits Per User
- **File:** `backend/src/game/matchmaking/state.rs:38-41`
- **Issue:** One user_id can have multiple concurrent connections
- **Fix:** Enforce 1 connection per user_id, kick previous on new join

### 10.2 No Timeout for Pending Games
- **File:** `backend/src/game/ephemeral/state.rs:37-48`
- **Issue:** Pending games in DashMap can accumulate indefinitely
- **Attack:** Create millions of games to exhaust memory
- **Fix:** Implement cleanup task (remove games older than 30 minutes)

### 10.3 Race Condition in Game Join
- **File:** `backend/src/game/matchmaking/state.rs:55-62`
- **Issue:** Channel lookup after match could fail if opponent disconnects
- **Fix:** Handle gracefully instead of `.expect()`

### 11.3 No CSRF Protection for WebSocket
- **Issue:** WebSocket connections don't use CSRF tokens
- **Fix:** Validate Origin header server-side

### 12.1 Secrets in Docker Compose
- **File:** `docker-compose.yml:9`
- **Issue:** Environment variables in compose file
- **Fix:** Use `.env` file (not committed)

### 13.1 Limited Security Logging
- **Issue:** No logging of security-relevant events (failed auth, invalid codes, suspicious patterns)
- **Fix:** Add structured logging with user_id, remote_ip, timestamp, event_type

---

## Low Severity Findings

### 11.2 WebSocket URL Hardcoding in Development
- **File:** `frontend/index.html:428-430`
- **Issue:** File protocol check for development
- **Fix:** Use configuration/env-based URLs for all environments

---

## Good Practices Found ✓

### 3.1 Safe SQL Queries (Using SQLx)
- **Status:** All queries use parameterized statements with `.bind()`
- **Note:** Excellent practice - SQLx compile-time checking prevents SQL injection

---

## Priority Roadmap

### Phase 1 - CRITICAL (Do First)
1. Fix `edition = "2021"` in Cargo.toml
2. Implement JWT authentication
3. Add WebSocket origin validation

### Phase 2 - HIGH (Do Soon)
1. Add input validation for player names
2. Add rate limiting on connections and messages
3. Use UUIDs for internal player IDs
4. Add CORS middleware
5. Implement error handling for unwraps

### Phase 3 - MEDIUM (Do Before Production)
1. Add pending game timeout
2. Remove DOM-based XSS vector
3. Add security event logging
4. Fix error messages to be less verbose

---

## References

- [OWASP WebSocket Security](https://cheatsheetseries.owasp.org/cheatsheets/WebSockets_Cheat_Sheet.html)
- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)
- [JWT Best Practices](https://datatracker.ietf.org/doc/html/rfc8725)
