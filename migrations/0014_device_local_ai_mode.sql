-- Device-owned custom AI credentials still need durable server-side mode
-- metadata so the server can enforce builtin/custom/off selection.
CREATE TABLE user_ai_credentials_v2 (
    user_id TEXT PRIMARY KEY REFERENCES user_accounts (user_id) ON DELETE CASCADE,
    mode TEXT NOT NULL CHECK (mode IN ('builtin', 'custom', 'off')),
    provider TEXT,
    base_url TEXT,
    model TEXT,
    encrypted_api_key BLOB,
    key_version INTEGER,
    updated_at_ms INTEGER NOT NULL,
    CHECK (
        (mode = 'custom' AND provider IS NOT NULL AND base_url IS NOT NULL AND model IS NOT NULL
             AND ((encrypted_api_key IS NULL AND key_version IS NULL)
                  OR (encrypted_api_key IS NOT NULL AND key_version IS NOT NULL)))
        OR mode IN ('builtin', 'off')
    )
);

INSERT INTO user_ai_credentials_v2(
    user_id, mode, provider, base_url, model, encrypted_api_key, key_version, updated_at_ms
)
SELECT user_id, mode, provider, base_url, model, encrypted_api_key, key_version, updated_at_ms
FROM user_ai_credentials;

DROP TABLE user_ai_credentials;
ALTER TABLE user_ai_credentials_v2 RENAME TO user_ai_credentials;
