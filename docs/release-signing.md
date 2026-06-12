# Release and Signing Notes

## Current state

The repository has release and validation workflows:

- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`

The current release posture is intentionally staged:

- Windows client builds are generated without code signing.
- The public discovery service is released as a Docker image archive plus Docker Compose deployment assets.
- macOS and Linux desktop client packages are not published in the first split-architecture release.
- The release workflow creates a draft prerelease when a `v*` tag is pushed.

This gives the project a repeatable split-architecture release pipeline now, while keeping the upgrade path to formal signing and notarization clear.

Before cutting a tag, run the local release readiness check:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File deploy/scripts/test-mpgs-release-readiness.ps1
```

This verifies version alignment, release workflow shape, deployment validation script packaging, and regenerated OpenAPI consistency.

## What the workflows do

### CI workflow

`ci.yml` runs on pull requests and on pushes to `main`.

It validates source compatibility on:

- `windows-latest`
- `macos-latest`

The macOS validation job is not a release job and does not publish a macOS client package.

Each run performs:

1. `npm ci`
2. `npm test`
3. `npm run build`
4. `cargo test --manifest-path src-tauri/Cargo.toml --locked`

### Release workflow

`release.yml` runs when a Git tag matching `v*` is pushed.

It publishes:

- Windows Tauri user client installer
- `mpgs-server` Docker image archives for `linux/amd64` and `linux/arm64`
- Docker Compose deployment files
- optional Caddy Compose profile
- example service configuration files
- generated OpenAPI JSON

It intentionally does not publish macOS or Linux desktop client packages for the first split-architecture release.

## User-facing limitations right now

### Windows

Because the Windows installer is not code-signed yet:

- users downloading from the browser should expect a SmartScreen warning
- Microsoft Store submission is not ready

## How to cut a release

1. Bump the application version in:
   - `package.json`
   - `src-tauri/Cargo.toml`
   - `src-tauri/tauri.conf.json`
2. Commit the version change.
3. Create and push a tag such as `v0.1.0`.
4. Wait for `Release Split Architecture` to finish in GitHub Actions.
5. Open the generated draft prerelease and review the uploaded assets before publishing or sharing them.
6. Verify the draft release contains the Windows client installer, `mpgs-server` `linux-amd64` and `linux-arm64` image archives, deployment assets archive, and OpenAPI JSON.
7. Download the draft release assets locally and run:

   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass `
     -File deploy/scripts/test-mpgs-release-readiness.ps1 `
     -ArtifactsDir .\release-assets
   ```

## Future upgrade: formal macOS signing and notarization

When Apple credentials are available, replace the current ad-hoc approach with a real `Developer ID Application` certificate plus notarization.

The Tauri-side inputs that matter are:

- `APPLE_SIGNING_IDENTITY`
- either App Store Connect API credentials:
  - `APPLE_API_ISSUER`
  - `APPLE_API_KEY`
  - `APPLE_API_KEY_PATH`
- or Apple ID credentials:
  - `APPLE_ID`
  - `APPLE_PASSWORD`
  - `APPLE_TEAM_ID`

In addition, the CI runner must have the signing certificate imported into the macOS keychain before the Tauri build step runs. This repository does not yet include that certificate import step because no Apple certificate is available yet.

Recommended next change when the Apple credentials exist:

1. Store the `Developer ID Application` certificate securely for CI import.
2. Add a macOS release job and keychain import step before `tauri-action`.
3. Add notarization credentials as repository or environment secrets.
4. Confirm the release policy has been expanded beyond Windows client packages.

## Future upgrade: formal Windows signing

When a Windows code-signing certificate is available, the Tauri docs workflow can be added to the Windows release job.

The common GitHub secrets are:

- `WINDOWS_CERTIFICATE`
- `WINDOWS_CERTIFICATE_PASSWORD`

Typical follow-up change:

1. Decode the Base64 `.pfx` certificate in the workflow.
2. Import it into the Windows certificate store on the runner.
3. Add the Tauri Windows signing configuration that matches the chosen certificate flow.
4. Rebuild and confirm the installer is signed.

## Scope boundary

This setup deliberately does **not** include:

- macOS or Linux desktop client release packages
- macOS notarization
- Apple certificate import
- Windows certificate import
- Microsoft Store packaging
- Tauri updater signing

Those are separate follow-up tasks once credentials and release policy are ready.
