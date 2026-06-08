# MPGS Server Docker Compose Deployment

This deployment target runs `mpgs-server` with Postgres and an optional Caddy reverse proxy.
The server image also includes the built management UI and serves it from `/admin` on the same origin as the management API.

## Build Locally

Build the service image on a local development machine or CI runner. Do not build on the VPS.

```bash
docker build -f Dockerfile.mpgs-server -t mpgs-server:local .
docker save mpgs-server:local -o mpgs-server-local.tar
```

PowerShell users can run the checked local build script instead:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File deploy/scripts/build-mpgs-server-image.ps1 `
  -ImageTag mpgs-server:local `
  -OutputTar mpgs-server-local.tar
```

For an Arm VPS such as `ora_vps`, build a `linux/arm64` image locally or in CI before uploading:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File deploy/scripts/build-mpgs-server-image.ps1 `
  -ImageTag mpgs-server:local `
  -OutputTar mpgs-server-linux-arm64.tar `
  -UseBuildx `
  -Platform linux/arm64
```

Upload `mpgs-server-local.tar` plus the `deploy/` directory to the server, then load the image there:

```bash
docker load -i mpgs-server-local.tar
```

## Configure

Create a server-side `.env` from the example:

```bash
cp deploy/mpgs-server.env.example deploy/.env
```

Set at least:

- `POSTGRES_PASSWORD`
- `CADDY_DOMAIN` if using the Caddy profile

The Compose `.env` only locates the config directory indirectly through the compose file and sets container-level values such as the Postgres container credentials. Service settings and service secrets live in TOML files under `deploy/config`.

For first-run browser setup, create a setup token config:

```bash
cp deploy/config/setup.toml.example deploy/config/setup.toml
```

Set `setup.token_hash` to a SHA-256 hash in the format expected by `mpgs-server`:

```text
sha256:<base64-no-padding sha256(setup-token)>
```

The setup token allows the first-run setup API to write `deploy/config/active/service.toml` and `deploy/config/active/secrets.toml`. After setup writes active config, normal management access must use the admin token, not the setup token.

Do not put the raw setup token in `.env`, Postgres, docs, or logs.

Management configuration changes are written under `deploy/config/pending` first and report `restartRequired=true`. The service does not copy active secrets into pending files for service identity edits, so saving non-secret settings must not clear Steam, LLM, R2, or admin credentials.

`/api/v1/admin/restart` validates pending service configuration, requires admin authentication and explicit confirmation, then gracefully exits the service process so Docker Compose can restart it. It does not use the Docker socket, a restart-helper container, or arbitrary host commands. On the next startup, the service validates pending service configuration before promoting it to active.

The management diagnostics API reports the configured restart policy as deployment metadata rather than probing Docker from inside the container:

```toml
[deployment]
restart_policy = "compose:unless-stopped"
```

For manual offline configuration instead, create the active secrets file:

```bash
cp deploy/config/active/secrets.toml.example deploy/config/active/secrets.toml
```

Edit:

- `deploy/config/active/service.toml` for non-sensitive service identity and bind settings.
- `deploy/config/active/secrets.toml` for the Postgres URL, admin token hash, session secret, and future server-side secrets.

Set `service_connection.public_base_url` in `deploy/config/active/service.toml` to the HTTPS address clients should import or type. With the Caddy profile this is usually:

```toml
[service_connection]
public_base_url = "https://mpgs.example.com"
```

The admin connection-share API uses this value to generate a keyless service connection file. It does not infer the public URL from request headers and it must not include setup or admin tokens.

Enable public CORS for ordinary clients that fetch the anonymous REST API directly:

```toml
[public_cors]
allow_any_origin = true
```

This only affects public read routes such as `/api/v1/service-info` and `/api/v1/discovery-home`. Management, setup, and restart routes stay same-origin and must be used through the served management surface.

`/api/v1/admin/diagnostics` is admin-only and reports public base URL, HTTPS suitability, public CORS mode, restart policy metadata, and provider configuration presence using redacted statuses such as `configured` or `missing`. It must not return Steam, LLM, R2, setup, or admin token values.

## Client Connection Handoff

The user client does not contain a default official service address. Give users either:

- the HTTPS service address from `service_connection.public_base_url`
- the keyless connection file from the admin connection-share API

Before saving the service, the client validates `/api/v1/service-info`, confirms API `v1`, checks for the `public_catalog_read` capability, and successfully probes an anonymous public read endpoint such as `/api/v1/discovery-home`.

The public client reads anonymous REST APIs directly over HTTPS. Tauri Rust must not proxy public catalog reads. Personal game state stays in client local storage and is isolated by service instance ID.

For the default Compose network, `deploy/config/active/secrets.toml` should use:

```toml
[database]
url = "postgres://mpgs:change-this-postgres-password@postgres:5432/mpgs"
```

Keep the database password in `deploy/.env` and `deploy/config/active/secrets.toml` in sync.

Set `admin.token_hash` to a SHA-256 hash in the format expected by `mpgs-server`:

```text
sha256:<base64-no-padding sha256(admin-token)>
```

Set `admin.session_secret` to a long random value used to sign short-lived admin session cookies. Do not put the raw admin token in `.env`, Postgres, docs, or logs.

Do not put Steam, LLM, R2, setup token, or admin token secrets in Postgres.

## Key Rotation

Rotate service-side secrets from the management UI whenever possible:

- Steam, LLM, and R2 keys are written as pending service configuration and are never echoed back to the UI.
- Admin token changes must store only `admin.token_hash`, never the raw token.
- Saving unrelated non-secret settings must inherit active secrets and must not clear Steam, LLM, R2, setup, or admin credentials.
- Pending changes report `restartRequired=true`.
- `/api/v1/admin/restart` validates pending configuration, records audit data, and exits the service so Compose restarts it.

For manual emergency rotation, edit `deploy/config/active/secrets.toml` on the server, keep file permissions restricted to operators, then restart the Compose service. Do not rotate secrets by editing `deploy/.env` except for the Postgres container password itself.

## Start Without Public HTTPS

This binds the service only to localhost on the host, which is suitable behind another reverse proxy:

```bash
docker compose --env-file deploy/.env -f deploy/compose.yml up -d
curl http://127.0.0.1:4310/healthz
curl http://127.0.0.1:4310/api/v1/service-info
```

## Start With Caddy

Point DNS for `CADDY_DOMAIN` at the server first. Then run:

```bash
docker compose --env-file deploy/.env \
  -f deploy/compose.yml \
  -f deploy/compose.caddy.yml \
  --profile caddy \
  up -d
```

The public client should use HTTPS:

```bash
curl https://$CADDY_DOMAIN/healthz
curl https://$CADDY_DOMAIN/api/v1/service-info
```

The management surface is served by `mpgs-server` itself:

```bash
curl https://$CADDY_DOMAIN/admin
```

The Docker image sets `MPGS_ADMIN_STATIC_DIR=/usr/local/share/mpgs/admin`, which points to the Vite `dist` output copied into the runtime image. For local development outside Docker, `mpgs-server` defaults to `dist` unless `MPGS_ADMIN_STATIC_DIR` is set.

## Backup and Restore

Back up both Postgres data and server configuration. A database-only backup is not enough because service identity, public base URL, admin token hash, session secret, and provider credentials live in TOML files.

Create a database backup from the server:

```bash
docker compose --env-file deploy/.env -f deploy/compose.yml \
  exec -T postgres pg_dump -U mpgs -d mpgs > mpgs.sql
```

Back up these files to a protected location:

- `deploy/.env`
- `deploy/config/active/service.toml`
- `deploy/config/active/secrets.toml`
- `deploy/config/setup.toml` if safe-mode repair access must be preserved
- selected files under `deploy/config/pending/` only when you intentionally want to preserve pending changes

Restore by placing `deploy/.env` and config files back on the server, restoring the Postgres dump into the `postgres` service, then running `docker compose up -d`. Verify `/healthz`, `/api/v1/service-info`, and `/admin` after restore.

## OpenAPI Generation

The REST contract is generated from Rust server types. Regenerate OpenAPI locally or in CI, not on the VPS:

```powershell
cargo run -p mpgs-server -- --export-openapi > docs/openapi/mpgs-server.openapi.json
npm run generate:api-types
```

Commit both `docs/openapi/mpgs-server.openapi.json` and `src/api/generated/mpgsServerApi.ts` when API response shapes change. Public clients and the admin UI should use generated TypeScript contract types rather than hand-written copies.

## Remote Deploy Verification

After the image archive exists locally and the server has its real `deploy/.env` plus either `deploy/config/setup.toml` or active service config, the checked remote deploy script can upload the image archive and compose assets, load the image, start Compose, and verify both `/healthz` and `/api/v1/service-info`:

Before uploading, run the read-only remote preflight. It checks the local image archive path when provided, remote Docker/Compose access, deployment directory presence, redacted `.env` keys, active/setup config presence, 80/443 occupancy, and the health/service-info/admin probe URLs. It does not load images, start Compose, build, or compile on the VPS:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File deploy/scripts/test-mpgs-server-remote-preflight.ps1 `
  -RemoteHost ora_vps `
  -RemotePath '~/mpgs-server' `
  -ImageTar mpgs-server-linux-arm64.tar `
  -UseSudoDocker `
  -UseCaddy `
  -PublicBaseUrl https://$env:CADDY_DOMAIN
```

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File deploy/scripts/deploy-mpgs-server-remote.ps1 `
  -RemoteHost ora_vps `
  -RemotePath '~/mpgs-server' `
  -ImageTar mpgs-server-linux-arm64.tar `
  -UseSudoDocker `
  -UseCaddy `
  -PublicBaseUrl https://$env:CADDY_DOMAIN
```

The remote script uploads only compose files, Caddy config, example TOML files, and the image archive. It does not overwrite remote `deploy/.env`, active secrets, or active service config. The server-side steps are limited to `docker load`, `docker compose up -d`, `curl` probes, and `docker compose ps`.

If the remote deploy user cannot access `/var/run/docker.sock` directly but has passwordless sudo for Docker, pass `-UseSudoDocker`. The remote commands then use `sudo -n docker load`, `sudo -n docker compose up -d`, probes, and `sudo -n docker compose ps`; they still do not compile or build anything on the VPS.

## Operational Boundary

- The VPS only runs `docker load` and `docker compose up`; it must not compile Rust or build the image.
- The service container runs database migrations on startup.
- The mounted config directory is writable so setup and later management APIs can write TOML configuration files; `.env` still must not contain service secrets.
- Managed restart relies on the Compose `restart: unless-stopped` policy and service self-exit.
- Public `/healthz` is intentionally minimal and does not expose configuration or secrets.
- Empty public catalog state is healthy; public library population belongs to later discovery/admin slices.
