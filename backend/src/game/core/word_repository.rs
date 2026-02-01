use super::word::Word;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct WordRepository {
    pool: SqlitePool,
}

impl WordRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn get_random(&self) -> Option<Word> {
        let row: (String, String) =
            sqlx::query_as("SELECT kanji, reading FROM words ORDER BY RANDOM() LIMIT 1")
                .fetch_optional(&self.pool)
                .await
                .ok()??;

        Some(Word {
            kanji: row.0,
            reading: row.1,
        })
    }

    /// Check if the given reading is valid for the given kanji.
    /// Returns true if there's a word entry with this kanji/reading pair.
    pub async fn is_valid_reading(&self, kanji: &str, reading: &str) -> bool {
        sqlx::query_scalar::<_, i32>("SELECT 1 FROM words WHERE kanji = ? AND reading = ?")
            .bind(kanji)
            .bind(reading)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten()
            .is_some()
    }
}
