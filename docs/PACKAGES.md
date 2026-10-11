# Package format and runtime API

Import a local ZIP with these files at its root:

```json
{
  "id": "my-extension",
  "name": "My extension",
  "version": "1.0.0",
  "kind": "extension",
  "description": "What it does",
  "source": "Author and source URL",
  "compatibility": "spotify-1.3.4-dom-v1",
  "entry": "extension.js"
}
```

For themes use `kind: "theme"`, `entry: "theme.css"`. IDs use lowercase ASCII letters/digits/hyphens, max 64 characters. Built-in IDs are reserved. Entry must be a filename. ZIP traversal, backslash/drive paths, symlinks, >10,000 entries, >256 MB expanded total or >64 MB individual files are rejected. CSS <=256 KB; JS <=512 KB. No CSS `url()` or `@import`. Source metadata is descriptive, not a certificate or trust guarantee.

`window.Resonance` exposes:

- `version`, `adapter`, read-only `config`, `selectors`.
- `on(target,event,handler)`: add listener and register cleanup; returns disposer.
- `register(disposer)`: register timer/DOM cleanup on pagehide.
- `button('play'|'next'|'previous')`: click the version adapter's native control; returns false and explains if absent.
- `isPlaying()`: native pause-control state; English aria-label fallback plus pause-icon test ID.
- `track()`: `{title,artist}` from current DOM.
- `notify(text)`, `report(packageId,error)`.

No authentication tokens, internal Cosmos/GraphQL/React/webpack APIs, network proxy, or filesystem IPC are supplied. Disabled extension code is not injected. Packaged extensions execute in separate startup try/catch blocks; later asynchronous errors belong to the package. Package authors must use cleanup registration and avoid busy polling.

Sleep timer: click its Spotify button to start/cancel; pauses once after configured minutes. Closing/restarting Spotify cancels the timer. Playback shortcuts: Alt+Shift+Space / Left / Right / I, ignored in editable fields. UI extensions are English-oriented; localized aria-label fallbacks are not certified.
