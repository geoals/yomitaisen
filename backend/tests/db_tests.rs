use sqlx::SqlitePool;

#[sqlx::test]
async fn migrations_run_successfully(pool: SqlitePool) {
    // Verify users table has 2 test users
    let user_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(user_count.0, 2);

    // Verify words table has 10 test words
    let word_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM words")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(word_count.0, 10);
}

#[sqlx::test]
async fn can_lookup_word_by_kanji(pool: SqlitePool) {
    let reading: (String,) = sqlx::query_as("SELECT reading FROM words WHERE kanji = $1")
        .bind("日本")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(reading.0, "にほん");
}
