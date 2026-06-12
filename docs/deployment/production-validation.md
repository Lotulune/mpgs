# MPGS Production Validation

This checklist closes the three validation gaps that remain after local WSL Docker sample-catalog validation:

- real public catalog validation
- production deployment validation
- release readiness validation

The scripts are read-only from the client side. They do not seed data, write configuration, rotate keys, restart services, push tags, or publish releases.

## 1. Real Public Catalog Validation

Use this after first-run setup and at least one real admin discovery path has produced public catalog data. A sample-only catalog is useful for local UI testing, but it is not enough for production catalog validation.

Prerequisites:

- The service has active configuration.
- Steam is configured on the server if real catalog generation is expected.
- At least one public game has been imported by admin setup/discovery jobs.
- The public service URL is reachable from the validation machine.

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File deploy/scripts/test-mpgs-public-catalog-live.ps1 `
  -BaseUrl https://mpgs.example.com `
  -MinGames 1
```

The script verifies:

- `GET /api/v1/service-info`
- `GET /api/v1/discovery-home`
- `GET /api/v1/games`
- `GET /api/v1/games/{appid}`
- `GET /api/v1/games/{appid}/analysis`
- API version `v1`
- `public_catalog_read` capability
- non-empty public catalog metadata
- rich display fields such as description, capsule image, screenshots, tags, and multiplayer modes
- public read analysis for a real catalog item

By default it rejects a catalog that appears to contain only deterministic sample appids. For local sample checks only, pass `-AllowSampleCatalog`.

## 2. Production Deployment Validation

Use this against a deployed service address after remote preflight and deploy have succeeded.

Public-only check:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File deploy/scripts/test-mpgs-production-readiness.ps1 `
  -BaseUrl https://mpgs.example.com `
  -RequirePublicCors
```

Admin diagnostics check:

```powershell
$env:MPGS_ADMIN_TOKEN = "<admin-token>"
powershell -NoProfile -ExecutionPolicy Bypass `
  -File deploy/scripts/test-mpgs-production-readiness.ps1 `
  -BaseUrl https://mpgs.example.com `
  -RequirePublicCors `
  -RequireAdminDiagnostics `
  -RequireSteamConfigured
Remove-Item Env:\MPGS_ADMIN_TOKEN
```

The public-only check verifies:

- HTTPS unless `-AllowHttp` is explicitly passed for local/LAN validation
- `/healthz`
- `/api/v1/service-info`
- public read CORS when required
- same-origin `/admin` static entry

The admin diagnostics check additionally verifies:

- admin session cookie login
- active config readiness
- configured public base URL
- HTTPS suitability
- Compose restart policy metadata
- public CORS diagnostics
- optional Steam provider readiness
- keyless connection-share response
- no secret-looking fields in the connection-share payload

## 3. Release Readiness Validation

Use this before tagging a split-architecture release.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File deploy/scripts/test-mpgs-release-readiness.ps1
```

The script verifies:

- `package.json`, Tauri config, `src-tauri`, `mpgs-core`, and `mpgs-server` versions match
- release tag shape is compatible with the `v*` workflow trigger
- release workflow publishes the Windows client installer
- release workflow publishes `linux/amd64` and `linux/arm64` server image archives
- release workflow creates a draft prerelease
- deployment assets include the validation scripts
- release documentation still states the unsigned Windows and draft-prerelease posture
- OpenAPI contains the expected public/admin contract paths
- regenerated OpenAPI matches `docs/openapi/mpgs-server.openapi.json`

After GitHub Actions creates a draft release, download its assets into a local directory and run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File deploy/scripts/test-mpgs-release-readiness.ps1 `
  -ArtifactsDir .\release-assets
```

That additional check verifies that the downloaded release assets contain:

- a Windows installer
- `linux-amd64` server image tar
- `linux-arm64` server image tar
- deployment assets tarball
- OpenAPI JSON

## Full Local Verification

After changing validation scripts or release/deployment docs, run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File docs/deployment/deployment_contract_tests.ps1
npm test
npm run build
cargo test --workspace
```

Postgres smoke tests that require a live test database only run when `MPGS_TEST_DATABASE_URL` is set. They are still part of the production confidence path and should be run in CI or a local environment with a disposable Postgres database before publishing a release.
