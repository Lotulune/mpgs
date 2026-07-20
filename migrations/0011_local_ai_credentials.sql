-- Custom AI secrets are device-owned from M7 onward. Remove any credentials
-- persisted by earlier previews while retaining non-secret mode metadata.
UPDATE user_ai_credentials
SET mode = 'off',
    provider = NULL,
    base_url = NULL,
    model = NULL,
    encrypted_api_key = NULL,
    key_version = NULL
WHERE mode = 'custom' OR encrypted_api_key IS NOT NULL;
