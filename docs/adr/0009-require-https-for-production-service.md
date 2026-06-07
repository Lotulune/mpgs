# Require HTTPS for Production Service Access

Production deployments of the public discovery service must use HTTPS, while development and explicitly marked LAN deployments may use HTTP. Management sessions, setup flows, and service-address validation all cross trust boundaries, so plaintext public access is not an acceptable default even though ordinary clients use anonymous read routes.

**Consequences**

The user client should reject public HTTP service addresses by default and only allow localhost or private-network exceptions when explicitly enabled. Production admin cookies must use `Secure`, and deployment documentation must cover reverse proxy or TLS termination setup.
