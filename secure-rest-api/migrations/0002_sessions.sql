CREATE TABLE auth_sessions (
    id              INTEGER PRIMARY KEY,
    user_id         INTEGER NOT NULL,
    expires_at      INTEGER NOT NULL,
    revoked_at      INTEGER,

    FOREIGN KEY (user_id)
        REFERENCES users(id)
        ON DELETE CASCADE
);

CREATE TABLE refresh_tokens (
    id              INTEGER PRIMARY KEY,
    session_id      INTEGER NOT NULL,
    token_hash      BLOB NOT NULL UNIQUE,
    consumed_at         INTEGER,

    FOREIGN KEY (session_id)
        REFERENCES auth_sessions(id)
        ON DELETE CASCADE,

    CHECK (length(token_hash) = 32)
);

CREATE UNIQUE INDEX one_active_token_per_session
    ON refresh_tokens(session_id)
    WHERE consumed_at IS NULL;

CREATE INDEX refresh_tokens_by_session
    ON refresh_tokens(session_id);