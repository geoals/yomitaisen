CREATE TABLE words (
    id INTEGER PRIMARY KEY,
    kanji TEXT NOT NULL,
    reading TEXT NOT NULL,
    definitions TEXT,
    frequency_rank INTEGER,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),

    UNIQUE(kanji, reading)
);

CREATE INDEX idx_words_kanji ON words(kanji);
CREATE INDEX idx_words_frequency ON words(frequency_rank);

CREATE TRIGGER words_updated_at
AFTER UPDATE ON words
BEGIN
    UPDATE words SET updated_at = datetime('now') WHERE id = NEW.id;
END;
