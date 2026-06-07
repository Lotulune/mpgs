# Use Service Self-Restart for Managed Restarts

The management UI will support one-click restart after server-side key configuration changes, but the public discovery service will not control Docker, mount the Docker socket, or use a separate restart helper. Managed restart will be implemented by having the service validate the pending configuration, record the restart request, gracefully exit its own process, and rely on the Docker Compose restart policy to bring the container back.

**Consequences**

Docker Compose deployments must configure an appropriate restart policy such as `unless-stopped`. Restart requests still require admin authentication, explicit confirmation, rate limiting, audit logging, and startup safeguards that prevent a bad pending configuration from causing an unrecoverable restart loop.
