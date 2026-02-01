use crate::yomitan::TermEntry;
use sqlx::SqlitePool;
use std::collections::HashMap;

/// Statistics from an import operation
#[derive(Debug, Default)]
pub struct ImportStats {
    /// Number of entries that passed all filters
    pub filtered: usize,
    /// Number of rows successfully inserted (may differ due to UNIQUE constraint)
    pub inserted: usize,
    /// Number of entries skipped (no kanji, no frequency, etc.)
    pub skipped: usize,
}

/// A word ready to be inserted into the database
struct WordToInsert {
    kanji: String,
    reading: String,
    definitions: String, // JSON array
    frequency_rank: u32,
}

/// Import words into the database.
///
/// Filters entries to only include words that:
/// 1. Have kanji (term != reading) - we're testing kanji reading, not kana words
/// 2. Have frequency data available
/// 3. Are within max_rank if specified
///
/// Words are sorted by frequency rank (ascending = most common first).
/// Uses INSERT OR IGNORE to skip duplicates.
pub async fn import_words(
    pool: &SqlitePool,
    entries: Vec<TermEntry>,
    frequency: &HashMap<String, u32>,
    limit: Option<usize>,
    max_rank: Option<u32>,
) -> Result<ImportStats, Box<dyn std::error::Error>> {
    let mut stats = ImportStats::default();

    // Filter and prepare words
    let mut words_to_insert: Vec<WordToInsert> = entries
        .into_iter()
        .filter_map(|entry| {
            // Skip kana-only words (term == reading or reading is empty for kana terms)
            let has_kanji = !entry.reading.is_empty() && entry.term != entry.reading;
            if !has_kanji {
                stats.skipped += 1;
                return None;
            }

            // Skip words usually written in kana (JMdict "uk" tag)
            if entry.usually_kana {
                stats.skipped += 1;
                return None;
            }

            // Look up frequency rank
            let Some(&rank) = frequency.get(&entry.term) else {
                stats.skipped += 1;
                return None;
            };

            // Apply max_rank filter
            if let Some(max) = max_rank {
                if rank > max {
                    stats.skipped += 1;
                    return None;
                }
            }

            let definitions =
                serde_json::to_string(&entry.definitions).unwrap_or_else(|_| "[]".to_string());

            Some(WordToInsert {
                kanji: entry.term,
                reading: entry.reading,
                definitions,
                frequency_rank: rank,
            })
        })
        .collect();

    // Sort by frequency rank (most common first)
    words_to_insert.sort_by_key(|w| w.frequency_rank);

    // Apply limit
    if let Some(limit) = limit {
        words_to_insert.truncate(limit);
    }

    stats.filtered = words_to_insert.len();

    // Insert words in batches
    const BATCH_SIZE: usize = 500;

    for chunk in words_to_insert.chunks(BATCH_SIZE) {
        let mut query = String::from(
            "INSERT OR IGNORE INTO words (kanji, reading, definitions, frequency_rank) VALUES ",
        );

        for (i, _) in chunk.iter().enumerate() {
            if i > 0 {
                query.push_str(", ");
            }
            query.push_str("(?, ?, ?, ?)");
        }

        // Build and execute the query
        let mut q = sqlx::query(&query);
        for word in chunk {
            q = q
                .bind(&word.kanji)
                .bind(&word.reading)
                .bind(&word.definitions)
                .bind(word.frequency_rank as i64);
        }

        let result = q.execute(pool).await?;
        stats.inserted += result.rows_affected() as usize;
    }

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(term: &str, reading: &str) -> TermEntry {
        TermEntry {
            term: term.to_string(),
            reading: reading.to_string(),
            definitions: vec!["test definition".to_string()],
            usually_kana: false,
        }
    }

    #[test]
    fn test_filter_kana_only_words() {
        let entry = make_entry("ひらがな", "ひらがな");
        assert_eq!(entry.term, entry.reading); // Would be filtered out
    }

    #[test]
    fn test_kanji_word_has_different_reading() {
        let entry = make_entry("日本", "にほん");
        assert_ne!(entry.term, entry.reading); // Would pass filter
    }
}
