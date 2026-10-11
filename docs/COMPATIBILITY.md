# Compatibility subset

Initial adapter: standalone Windows Spotify 1.3.4.x; observed patch 1.3.4.258. Version family allowlist plus clean `xpui.spa` entrypoint validation; this does not certify every future 1.3.4 patch. Unknown major/minor families block apply. GUI uses the default roaming installation path. Microsoft Store is unsupported.

| Input | Supported | Limits / conversion |
|---|---|---|
| Self-contained CSS | Yes | Parser + delimiter checks, no external resources |
| Spicetify `user.css` with `--spice-text`, `main`, `sidebar`, `player`, `card`, `subtext`, `button` | Subset | Import in Settings. Hash-class CSS mappings are not rewritten. Semantic `.main-*` classes introduced by Spicetify need conversion to real current test IDs. |
| `color.ini` schemes | Manual conversion | Choose colors in manager; no automatic INI importer |
| JS using `Spicetify.showNotification` and `Spicetify.LocalStorage.get/set/remove` | Subset | Wrap in a Resonance manifest ZIP and deliberately install. No arbitrary raw JS auto-import. |
| `Spicetify.Player`, Platform, CosmosAsync, GraphQL, React, Menu, URI, webpack modules | Unsupported | Rewrite to documented Resonance DOM API where practical |
| Spicetify Marketplace catalog | Subset | Public GitHub manifests and snippets; cached discovery, source/hash review, compatible CSS and minimal DOM JavaScript only |
| Custom Apps / asset replacement / theme.js | Unsupported | No routing or webpack bundle replacements |

Representative tests: original notification/storage example runs unchanged under the shim; current upstream SpicetifyDefault CSS is analyzed locally for mapped-selector dependence. Its native element/color-variable rules can be converted; the full theme is not advertised as compatible. An original CSS example using `--spice-text` and current data-testid selectors is fixture-tested. Bundled playback shortcuts/sleep timer target actual native controls. See verification for live results.

Spotify-specific selectors are centralized in runtime/runtime.js and runtime/theme.css. There is no injection into binary modules or V8 snapshot extraction. The client retains its original JS and authentication/streaming stack.
