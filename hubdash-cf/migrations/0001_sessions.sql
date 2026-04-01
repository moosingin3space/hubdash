CREATE TABLE IF NOT EXISTS sessions (
    id          TEXT    PRIMARY KEY,
    data        JSONB   NOT NULL,   -- Session (user + access_token) as binary JSON
    expires_at  INTEGER NOT NULL    -- Unix timestamp (seconds since epoch)
);

CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions (expires_at);
