# TODO

## Completed

- [x] Disconnect handling - Notify opponent and clean up game state
- [x] Round timeout - End rounds after 15 seconds
- [x] Skip mechanics - Both players vote to skip
- [x] Rematch functionality
- [x] Sound effects for wins/losses
- [x] First-to-10 win condition
- [x] Import word dataset (40k words)

## Ephemeral Mode Improvements

- [ ] **Game configuration** - Let host select difficulty, number of rounds before starting
- [ ] **Public game lobby** - Option to list game publicly, let anyone join from a lobby browser
- [ ] **Word filtering** - Skip unsuitable words (mostly-hiragana compounds, single kana, etc.)
- [ ] **Reconnection handling** - Allow players to reconnect to ongoing games after disconnect
- [ ] **Prevent duplicate words** - Don't show the same word twice in the same game

## Dictionary Quality

- [ ] **Frequency data per kanji form** - Currently frequency data is per word, not per specific kanji form. Example: 寇 (こう) was shown as a question, but this is an extremely rare writing of a common word. Need frequency data tied to the actual kanji representation, not just the word/reading pair. This would allow filtering out obscure writings while keeping common ones.

## Bug Fixes

- [ ] **Duplicate user prevention** - Reject join if same username already connected
- [ ] **Fix dead_code warning** - `cleanup` method in `active_game.rs:48` unused

## Future Phases

- [ ] User authentication (argon2 + JWT)
- [ ] Match history & user profiles
- [ ] Glicko-2 rating system
- [ ] Rating-based matchmaking
