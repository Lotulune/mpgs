# Local WSL Docker Validation

Date: 2026-06-12

Scope: validate that the Windows client can connect to `mpgs-server` running in WSL Docker. This is a local development validation path, not a production deployment guide.

## Validated State

- WSL Docker is installed and reachable by the normal WSL user.
- `docker info` succeeds without `sudo`.
- Docker server version observed during validation: `29.1.3`.
- `deploy-postgres-1` reached `healthy`.
- `deploy-mpgs-server-1` reached `healthy`.
- Windows API probes succeeded against `http://127.0.0.1:4311`.
- Windows browser client at `http://127.0.0.1:5173` connected to the Docker service.
- Public REST calls were observed for `/api/v1/discovery-home` and `/api/v1/games`.
- Sample public detail responses include rich display fields: `shortDescription`, `capsuleUrl`, `storeScreenshotUrls`, `tags`, `multiplayerModes`, and `reviewSnippets`.
- The Windows browser client rendered seeded sample images and public detail fields without falling back to Steam header URLs.
- Public service mode hid local AI assistant, sync, and maintenance entries.

The default Compose mapping is `127.0.0.1:4310:4310`. During this validation, Windows port `4310` was already occupied by another process, so the local override mapped the service to `127.0.0.1:4311`.

## Files and Local Artifacts

- Repo root: `D:\AI Coding\mpgs`
- Compose file: `deploy/compose.yml`
- Local ignored secret file: `deploy/config/active/secrets.toml`
- Temporary local env file in WSL: `/tmp/mpgs-docker.env`
- Temporary local Compose override in WSL: `/tmp/mpgs-compose-local.yml`
- Validation screenshots:
  - `test-results/docker-wsl-client-saved-flow.png`
  - `test-results/docker-wsl-client-connect-flow.png`
  - `test-results/docker-wsl-client-seeded-catalog.png`

`deploy/config/active/secrets.toml` is ignored by git and must remain local. Do not commit raw setup tokens, admin tokens, Steam keys, LLM keys, R2 credentials, or Postgres passwords.

## Prerequisites

Run the Windows commands from PowerShell and the Linux commands through `wsl.exe`.

```powershell
wsl.exe bash -lc "docker info"
```

If `docker info` only works as `root`, add the WSL user to the `docker` group and restart the WSL session:

```powershell
wsl.exe -u root bash -lc "usermod -aG docker aiyo"
wsl.exe --shutdown
```

After restarting WSL, verify again:

```powershell
wsl.exe bash -lc "docker info"
```

## Local Configuration

Use a local-only env file so the validation does not depend on a committed `.env`:

```powershell
wsl.exe bash -lc 'cat >/tmp/mpgs-docker.env <<EOF
MPGS_IMAGE=mpgs-server:local
POSTGRES_DB=mpgs
POSTGRES_USER=mpgs
POSTGRES_PASSWORD=mpgs-local-postgres
EOF'
```

Create a temporary override when Windows port `4310` is unavailable:

```powershell
wsl.exe bash -lc 'cat >/tmp/mpgs-compose-local.yml <<EOF
services:
  mpgs-server:
    ports:
      - "127.0.0.1:4311:4310"
EOF'
```

Create `deploy/config/active/secrets.toml` locally. This file must stay ignored:

```powershell
wsl.exe bash -lc 'cd "/mnt/d/AI Coding/mpgs" && \
ADMIN_TOKEN="local-admin-token" && \
ADMIN_TOKEN_HASH=$(printf "%s" "$ADMIN_TOKEN" | openssl dgst -sha256 -binary | openssl base64 -A | tr -d "=") && \
SESSION_SECRET=$(openssl rand -base64 48 | tr -d "\n") && \
cat > deploy/config/active/secrets.toml <<EOF
[database]
url = "postgres://mpgs:mpgs-local-postgres@postgres:5432/mpgs"

[admin]
token_hash = "sha256:${ADMIN_TOKEN_HASH}"
session_secret = "${SESSION_SECRET}"
EOF'
```

For local client validation, either keep `deploy/config/active/service.toml` as-is and connect manually to `http://127.0.0.1:4311`, or temporarily set:

```toml
[service_connection]
public_base_url = "http://127.0.0.1:4311"
```

Production deployments must use HTTPS unless they are explicitly marked development or LAN deployments.

## Start the Docker Service

Build the server image if it is not already available:

```powershell
wsl.exe bash -lc "cd '/mnt/d/AI Coding/mpgs' && docker build -f Dockerfile.mpgs-server -t mpgs-server:local ."
```

Start Postgres and `mpgs-server`:

```powershell
wsl.exe bash -lc "cd '/mnt/d/AI Coding/mpgs' && docker compose --env-file /tmp/mpgs-docker.env -f deploy/compose.yml -f /tmp/mpgs-compose-local.yml up -d"
```

Check container health:

```powershell
wsl.exe bash -lc "cd '/mnt/d/AI Coding/mpgs' && docker compose --env-file /tmp/mpgs-docker.env -f deploy/compose.yml -f /tmp/mpgs-compose-local.yml ps"
```

Expected result:

- `postgres` is `healthy`.
- `mpgs-server` is `healthy`.
- The service port maps as `127.0.0.1:4311->4310/tcp`.

## Seed a Sample Public Catalog

The service can import a deterministic sample public catalog for local validation. This avoids using real Steam or LLM credentials when the goal is to test the Windows client against non-empty public data.

The seed command is explicitly guarded by `MPGS_ALLOW_SAMPLE_CATALOG_SEED=1` and should only be used for local validation databases:

```powershell
wsl.exe bash -lc "cd '/mnt/d/AI Coding/mpgs' && docker compose --env-file /tmp/mpgs-docker.env -f deploy/compose.yml -f /tmp/mpgs-compose-local.yml run --rm -e MPGS_ALLOW_SAMPLE_CATALOG_SEED=1 mpgs-server --seed-sample-public-catalog"
```

Expected output:

```json
{
  "sampleCount": 4,
  "publicGames": 4,
  "appids": [920001, 920002, 920003, 920004]
}
```

Recreate or restart `mpgs-server` after seeding so `/api/v1/service-info` refreshes `publicCatalogStatus` from `empty` to `ready`:

```powershell
wsl.exe bash -lc "cd '/mnt/d/AI Coding/mpgs' && docker compose --env-file /tmp/mpgs-docker.env -f deploy/compose.yml -f /tmp/mpgs-compose-local.yml up -d --force-recreate mpgs-server"
```

## Probe from Windows

Run the probes from Windows PowerShell, not from inside WSL:

```powershell
Invoke-WebRequest -UseBasicParsing http://127.0.0.1:4311/healthz
Invoke-WebRequest -UseBasicParsing http://127.0.0.1:4311/api/v1/service-info
Invoke-WebRequest -UseBasicParsing http://127.0.0.1:4311/api/v1/discovery-home
Invoke-RestMethod -Uri 'http://127.0.0.1:4311/api/v1/games?limit=10&offset=0'
Invoke-RestMethod -Uri 'http://127.0.0.1:4311/api/v1/games/920001'
Invoke-RestMethod -Uri 'http://127.0.0.1:4311/api/v1/games/920001/analysis'
```

Expected result:

- Each request returns HTTP `200`.
- Public read responses include a CORS header such as `access-control-allow-origin: *` when `public_cors.allow_any_origin = true`.
- After sample seeding, `/api/v1/service-info` reports `publicCatalogStatus: "ready"`.
- `/api/v1/discovery-home` includes non-empty sections.
- `/api/v1/games` reports at least 4 total public games.
- `/api/v1/games/920001` includes:
  - `shortDescription: "A compact four-player harbor defense game built around quick co-op sessions."`
  - `capsuleUrl` beginning with `data:image/svg+xml;base64,`
  - at least one `storeScreenshotUrls` entry
  - tags including `Co-op`
  - multiplayer modes including `Online Co-op`
  - a review snippet containing `weeknight squad`
- `/api/v1/games/920001/analysis` returns read-only public rule analysis.

## Validate the Windows Client

Start the Vite client on Windows:

```powershell
npm run dev -- --host 127.0.0.1 --port 5173
```

Open:

```text
http://127.0.0.1:5173
```

Validation checklist:

- With a saved service connection to `http://127.0.0.1:4311`, the client loads the public service dashboard.
- The browser performs REST requests to `/api/v1/discovery-home` and `/api/v1/games`.
- After sample seeding, the dashboard shows non-empty public game sections.
- Opening `Harbor Havoc Co-op` loads the public detail page and read-only analysis.
- The detail page renders the seeded sample SVG image, short description, tags, multiplayer modes, current players, and review snippet from the public API response.
- Playwright validation observed `0` console errors and `0` page errors.
- Favorite, wishlist, and viewed/history state stay local to the Windows client and are partitioned by `serviceInstanceId`.
- Local AI assistant and local sync entries are hidden in public service mode.
- The Settings page shows the connected public service and can revalidate it.
- In a Tauri-like first-run state with no saved service connection, the first screen is the MPGS service connection page, not the old Steam onboarding.
- Entering `http://127.0.0.1:4311` and clicking connect saves the connection and loads the dashboard.

Screenshots from the 2026-06-12 validation:

- `test-results/docker-wsl-client-saved-flow.png`
- `test-results/docker-wsl-client-connect-flow.png`
- `test-results/docker-wsl-client-seeded-catalog.png`

## Stop Local Validation Services

Stop the Vite dev server from Windows if it is still running:

```powershell
Get-Process node -ErrorAction SilentlyContinue | Where-Object { $_.Path -like '*node*' }
```

Then stop the matching process, or use the terminal where `npm run dev` is running.

Stop Docker containers:

```powershell
wsl.exe bash -lc "cd '/mnt/d/AI Coding/mpgs' && docker compose --env-file /tmp/mpgs-docker.env -f deploy/compose.yml -f /tmp/mpgs-compose-local.yml down"
```

## Known Limits

- This validation proves Windows-client-to-WSL-Docker connectivity and public-service client behavior.
- It does not prove production HTTPS, Caddy, DNS, remote deployment, backup, or key rotation.
- The sample public catalog is deterministic local validation data. It is not a substitute for real Steam discovery or LLM-backed production catalog generation.
- The local Docker database can be empty and still healthy. Use the sample seed only when the validation target is non-empty public reads and Windows client behavior.

## Next Validation Slice

After the sample catalog baseline is stable, repeat the same checks against real catalog data from admin setup and discovery jobs. That slice should cover:

- discovery home with non-empty game sections
- paginated game list
- game detail hydration with rich display fields
- read-only public analysis display
- local-only favorite, wishlist, and history state partitioned by `serviceInstanceId`

Use the live public catalog validation script for that slice:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File deploy/scripts/test-mpgs-public-catalog-live.ps1 `
  -BaseUrl https://mpgs.example.com `
  -MinGames 1
```

For local sample-only rechecks, pass `-AllowHttp -AllowSampleCatalog` and point `-BaseUrl` at the local override such as `http://127.0.0.1:4311`. Do not count that sample-only run as real production catalog validation.
