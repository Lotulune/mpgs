# Use Paginated Public Library APIs

The public discovery service will expose paginated and filterable game-list APIs instead of directly carrying forward the current Tauri `get_dashboard` payload shape. The local dashboard payload mixes public sections, local collections, runtime stats, and configuration state, while the service boundary requires anonymous public reads, local-only personal state, and separate management data.

**Consequences**

`GET /api/v1/discovery-home` should provide a compact public summary and preview endpoint. Full browsing, filtering, and sorting should use paginated `/api/v1/games` queries, with detail and analysis data fetched by appid.
