# Tighten Tauri CSP While Allowing HTTPS Service Connections

The user client will replace the current null Tauri CSP with a constrained policy, but `connect-src` may allow HTTPS service connections so users can connect to arbitrary self-hosted MPGS service addresses. This preserves the open-source self-hosting goal without reintroducing a Tauri Rust HTTP proxy for public read APIs.

**Consequences**

The client must validate service addresses before saving them, reject public HTTP by default, and keep non-network CSP directives as narrow as practical. Localhost and explicitly allowed LAN HTTP remain development or private-network exceptions.
