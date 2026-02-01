use sqlx::SqlitePool;

pub struct Word {
    pub kanji: String,
    pub reading: String,
}

pub struct WordRepository {
    pool: SqlitePool,
}

impl WordRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn get_random(&self) -> Option<Word> {
        let row: (String, String) = sqlx::query_as(
            "SELECT kanji, reading FROM words ORDER BY RANDOM() LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .ok()??;

        Some(Word {
            kanji: row.0,
            reading: row.1,
        })
    }
}
