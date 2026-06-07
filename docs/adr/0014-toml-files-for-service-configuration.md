# Use TOML Files for Service Configuration Managed by the UI

The management UI will write structured TOML configuration files in a mounted config directory rather than editing the Docker Compose `.env` file directly. Compose environment variables should only locate the config and data directories or select deployment profiles, while service settings, secrets, provider credentials, limits, and scheduling configuration live in service-managed TOML files outside Postgres.

**Consequences**

The service must write configuration atomically, protect sensitive config file permissions, mark changed settings as restart-required when necessary, and avoid storing Steam, LLM, R2, or admin credentials in Postgres.

Configuration files should use an active/pending structure. The management UI writes pending configuration, the service validates it before restart, and a successful restart promotes it to active so bad configuration does not create an unrecoverable restart loop.

Pending secret changes should use patch or inheritance semantics. Because the management UI must not display existing secret values, saving unrelated configuration must not clear active Steam, LLM, R2, or other third-party API keys.
