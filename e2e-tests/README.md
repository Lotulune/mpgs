# MPGS native desktop E2E

This suite follows Tauri 2's official `tauri-driver` WebDriver setup. It drives
the compiled native application, not a browser-hosted Vite page. Direct desktop
WebDriver execution is supported on Windows and Linux; macOS remains a bundle
smoke target because WKWebView has no native desktop WebDriver client.

Prerequisites:

- `tauri-driver` 2.0.6
- Windows: a Microsoft EdgeDriver matching the installed Edge version
- Linux: `webkit2gtk-driver` and `xvfb`

From the repository root:

```powershell
cargo install tauri-driver --version 2.0.6 --locked
pnpm desktop:e2e:build
pnpm desktop:e2e
```

Linux runs the last command through a virtual display:

```sh
xvfb-run -a pnpm desktop:e2e
```

The runner starts `mpgs-server` against a unique temporary SQLite database with
the deterministic demo seed, sets `MPGS_CLIENT_DATA_DIR` to isolate the desktop
SQLite state, and stops both processes after the suite. Successful layout
screenshots, failure screenshots, and process logs are written under
`e2e-tests/artifacts/`.

The E2E build merges `tauri.e2e.conf.json`, which enables WebView2's ephemeral
remote-debugging port for EdgeDriver. Production builds use only
`tauri.conf.json` and do not expose that browser argument.

The latest checked-in Windows execution evidence, including tool versions and
screenshot hashes, is recorded in `docs/M4_DESKTOP_E2E_RUN.md`.

Local execution is intentionally blocked before a usable WebDriver session when
`tauri-driver` or the platform driver is absent. On Windows, install/run the
official `msedgedriver-tool` (or put a matching `msedgedriver.exe` on `PATH`);
the CI job provisions both drivers reproducibly.

Official references:

- <https://v2.tauri.app/develop/tests/webdriver/>
- <https://v2.tauri.app/develop/tests/webdriver/ci/>
