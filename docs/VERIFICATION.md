# Verification, 2026-10-11

Windows x64; standalone Spotify 1.3.4.258. Evidence below separates real application checks from fixtures.

## Actual installed applications

- Installed the NSIS manager and opened the release UI. Installed executable matches the payload extracted from the installer byte for byte. The portable build differs only in Tauri's three-byte `UNK` / `NSS` bundle-type marker.
- Applied Slate with playback shortcuts and a one-minute sleep timer to the actual Spotify client. Verified the runtime and computed palette, visually inspected the client, toggled playback with Alt+Shift+Space, and observed the timer pause real playback after one minute.
- Restore returned the matching original SPA filename. Restarted Spotify; Resonance runtime was absent and native playback controls loaded.
- SHA-256 checks after Restore matched the original Spotify xpui/login archives and existing Spicetify config and both backup archives. No login/preferences/cache/library files are mutated by the engine.
- Fetched 163 live Spicetify Marketplace entries (25 themes, 45 extensions, 93 CSS snippets). Downloaded, assessed and installed the converted Rounded Now Playing Bar snippet. Other package/API compatibility is not certified.
- Verified every restyled GUI screen and major dialog in the installed 0.1.1 release. Tested dropdown width, source/hash review, incompatible install disabled, and a profile save persisting across restart. Rendered minimum-size viewport checks; this is separate from a full physical Windows scaling matrix. See RESTYLE.md.
- Published the public GitHub release/feed, installed 0.1.0, detected 0.1.1 through the updater, installed the signed download and observed restart into the five-view 0.1.1 design. The updated installed executable matched the published installer payload hash.
- SF font selection and import pipeline are implemented. No licensed Apple font file was supplied or bundled; actual San Francisco glyph rendering is not certified on this machine. Windows falls back to Segoe UI.

## Automated fixtures

18 Rust transaction tests cover repeat apply, apply/switch/restore, ZIP traversal, corrupt backups, stale version restore, unsupported versions, modified archive rejection, locks, interrupted archive relocation/rename/commit/finalization, profile persistence, extension consent, recovery mode, CSS inputs and Marketplace conversion.

Four JavaScript runtime tests cover idempotence/dormancy, keyboard disposal and ignored edit fields, timer cancellation/pause/cleanup, and the supported Spicetify notification/storage shim. Production TypeScript/Vite and signed Tauri installer builds pass.

## Limits

Only the observed standalone Windows adapter is supported. The three bundled palettes are fixture-tested; the live run above certified Slate, not every setting on every Spotify screen. Universal Marketplace compatibility, custom apps, internal Spicetify Player/Platform/Cosmos APIs, Microsoft Store installations and a full Windows DPI matrix are not supported or certified. Signed manager release delivery is separate from Authenticode signing. Initial installers have no Authenticode certificate.
