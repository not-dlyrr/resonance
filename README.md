# Resonance

Independent Windows desktop customization for the official Spotify client. Spotify handles authentication, playback, library, and downloads. Resonance changes its interface and adds optional page extensions.

Tauri 2 + React/TypeScript + shared Rust engine/CLI. No Spicetify executable dependency. MIT original code.

## Supported installation

Standalone Windows Spotify at `%APPDATA%/Spotify`, **1.3.4.x only** in the initial adapter. Other versions fail closed for apply. Store and nonstandard GUI installations are unsupported in this release; the CLI accepts an explicit standalone path. Verify against the tested patch version in [verification](docs/VERIFICATION.md). There is no universal Spicetify API compatibility.

## Workflow

1. Choose Slate, Ink, or Paper. Set accent, font, track density, corner radius, and artwork visibility.
2. Deliberately install Playback shortcuts and/or Sleep timer from Extensions; enable desired packages.
3. Review pending changes. Close Spotify. Apply. Reopen Spotify.
4. Save a named profile in Profiles. Loading/importing a profile makes pending changes; review/apply switches the client.
5. Settings provides custom CSS validation/reset, signed manager updates, backup, repair, recovery mode, and restore.

The manager can close completely. Applied customization remains in Spotify. No startup service or resident tray process is installed. Automatic checks run only while the manager is open. New Spotify binaries/archives invalidate previous state; profiles remain. Reapply only if the adapter supports the new version. Stale restores are refused.

## Build and test

Windows x64, Rust stable, Node 22+, Visual Studio C++ build tools, WebView2 runtime.

```powershell
npm ci
npm run build
cargo test -p resonance-engine
node --test tests/runtime.test.mjs
cargo build --release -p resonance-engine --bin resonance-cli
npm run tauri build
```

For signed release builds, follow [UPDATES.md](docs/UPDATES.md). A local build without the release signing key can use a configuration override with `bundle.createUpdaterArtifacts=false`; it cannot publish trusted updates.

CLI:

```powershell
./target/release/resonance-cli.exe status
./target/release/resonance-cli.exe backup
./target/release/resonance-cli.exe install keyboard
./target/release/resonance-cli.exe apply profile.json
./target/release/resonance-cli.exe safe profile.json
./target/release/resonance-cli.exe restore
./target/release/resonance-cli.exe recover
```

Use `--root PATH --state PATH` for a disposable installation. `fixture-version` is exclusively a test-fixture marker; never place it in a real installation.

## Packages and trust

Catalog data comes from bundled manifests, locally imported packages, and Spicetify Marketplace's public GitHub topics/manifests and snippet catalog. Store browsing is cached; refresh checks the top 15 repositories per topic, with bounded download concurrency. Review the downloaded source and compatibility assessment before deliberate installation. Packages depending on unsupported Spicetify APIs or selectors require conversion. Import ZIPs containing `manifest.json` and a single CSS or JS entry. See [PACKAGES.md](docs/PACKAGES.md).

Extensions run JavaScript in Spotify's signed-in page. They can access its page storage, DOM, and network capabilities. Install only trusted code. There is no privileged Rust bridge in the injected page. Package code is never executed by the manager backend.

## Recovery and data

The managed interface replaces `Apps/xpui`. The original `xpui.spa` is relocated unchanged to `Apps/.resonance-original-xpui` so Spotify loads the extracted interface; Restore returns it to its original filename. Login resources, preferences, cached/downloaded music, and Spicetify configuration/backups are untouched. Backups record original archive/executable SHA-256 and version. Extracted existing customizations are renamed to `Apps/.resonance-preserved-*` on first apply. Restore loads the matching stock SPA; prior third-party customization is retained separately.

Directory replacements use a journal and same-volume rename, with hashed staging and verified rollback. A failed transaction blocks launch through the manager until recovery. Pre-existing modifications and applied revisions are retained for recovery; repeated applies consume disk space (about the extracted interface size per retained revision). Inspect before manually removing older retained directories. Do not restore old directories after Spotify updates.

See [COMPATIBILITY.md](docs/COMPATIBILITY.md), [PERFORMANCE.md](docs/PERFORMANCE.md), and [VERIFICATION.md](docs/VERIFICATION.md) for measured scope and limitations.

The full interface uses a v0 design adapted to accessible working controls. San Francisco families are supported through installed fonts or Settings → Import local SF Pro font. Apple fonts are not distributed. See [DESIGN.md](docs/DESIGN.md).

## Research attribution

Researched [Spicetify CLI](https://github.com/spicetify/cli), commit `8c7f83e3067de9659caca4c9da7979da3897458e27`, and its [theme](https://spicetify.app/docs/development/themes) and [extension](https://spicetify.app/docs/development/extensions) documentation on 2026-10-11. Upstream is LGPL-2.1. No upstream source code or helper runtime is bundled/reused; the SPA layout, entrypoint and adapter approach were independently implemented. Upstream fixtures used only for local compatibility analysis, not redistributed. Third-party Rust/npm dependencies retain their own licenses.

Not affiliated with Spotify or Spicetify.
