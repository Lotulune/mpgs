# Require a Setup Token for First-Run Configuration

The management UI will require a deployment-provided setup token before allowing first-run configuration. This prevents an exposed but unconfigured public discovery service from being claimed by the first visitor, while still allowing self-hosted users to complete Steam, LLM, and admin-token setup from the browser.

**Consequences**

Setup mode must be disabled after successful first-run configuration. After setup is complete, management access must use the normal administrator token and session-cookie flow rather than the setup token.
