# Use REST API v1 for the Service Contract

The public discovery service will expose a versioned REST API under `/api/v1` and publish an OpenAPI document for client and admin integrations. This matches the current Tauri command boundaries closely enough to migrate behavior incrementally while keeping the user client, admin UI, and future self-hosted deployments independent from a single frontend runtime or TypeScript-only RPC stack.

**Consequences**

Public read routes and admin write routes must be separated explicitly in the route tree. Breaking response changes require a new API version or a compatibility layer for existing user clients.
