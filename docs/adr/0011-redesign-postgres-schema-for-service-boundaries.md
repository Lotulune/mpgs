# Redesign the Postgres Schema Around Service Boundaries

The server-side Postgres schema will be designed around the public discovery service boundary instead of directly copying the current client-local SQLite tables. The existing SQLite schema remains useful as a source of field semantics and import mapping, but it mixes local dashboard payloads, client state, configuration, jobs, and cache concerns that should be separated in the service architecture.

**Consequences**

The existing local SQLite data does not need to be migrated into the new service. Public catalog data, operations data, and client-local personal state must not be collapsed back into one table model for convenience.
