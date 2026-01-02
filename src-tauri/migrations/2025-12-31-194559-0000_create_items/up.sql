CREATE TABLE IF NOT EXISTS items (
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
            text TEXT,
            image TEXT,
            image_width INTEGER,
            image_height INTEGER,
            timestamp BIGINT NOT NULL,
            size_bytes INTEGER NOT NULL,
            source_app TEXT
        )