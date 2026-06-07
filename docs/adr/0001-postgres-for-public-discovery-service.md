# Use Postgres for the Public Discovery Service

The public discovery service will use Postgres as its server-side database instead of carrying the current client-local SQLite model forward. Although the existing Tauri implementation already has a substantial SQLite schema, the service is intended to become the shared authority for the public game library, discovery jobs, AI analysis, and management operations, so choosing Postgres now avoids a second database migration immediately after the service boundary is established.

**Consequences**

The user client may still keep local storage for service address and personal game state, but the public game library belongs in Postgres on the service. Local SQLite code can inform the initial schema and migration, but it is not the target persistence layer for the service.
