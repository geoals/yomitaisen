-- Test users (no passwords needed for MVP gameplay testing)
INSERT INTO users (id, username, auth_provider) VALUES
    ('user-1', 'player1', 'local'),
    ('user-2', 'player2', 'local');

-- 10 common Japanese words with unambiguous readings
INSERT INTO words (kanji, reading, definitions, frequency_rank) VALUES
    ('日本', 'にほん', '["Japan"]', 1),
    ('学校', 'がっこう', '["school"]', 2),
    ('電話', 'でんわ', '["telephone"]', 3),
    ('先生', 'せんせい', '["teacher"]', 4),
    ('時間', 'じかん', '["time"]', 5),
    ('食べる', 'たべる', '["to eat"]', 6),
    ('飲む', 'のむ', '["to drink"]', 7),
    ('書く', 'かく', '["to write"]', 8),
    ('読む', 'よむ', '["to read"]', 9),
    ('聞く', 'きく', '["to hear", "to listen", "to ask"]', 10);
