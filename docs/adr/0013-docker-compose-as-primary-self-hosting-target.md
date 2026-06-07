# Use Docker Compose as the Primary Self-Hosting Target

The first public discovery service release will document Docker Compose as the only official self-hosting deployment target. The service depends on Postgres, Docker restart policy for managed restarts, and optional external image caching configuration, so Compose gives self-hosted users a repeatable deployment shape without requiring the project to support systemd, Kubernetes, or platform-specific bare-metal guides in the first release.

**Consequences**

Bare-metal execution remains a development or advanced self-managed path. Deployment documentation, setup token handling, Postgres persistence, restart policy, and HTTPS examples should be written around Compose first.
