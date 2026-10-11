# Resonance implementation and acceptance

Windows-first Tauri 2 / React / Rust. Independent engine and CLI; no Spicetify executable dependency. MIT original source, upstream research attribution.

## Milestones
1. Research installed Spotify and upstream; preserve existing customization. Establish charcoal/blue token system. **Research complete**: Spotify 1.3.4.258, clean xpui.spa, older extracted Spicetify setup. Upstream commit 8c7f83e3067de9659caca4c9da7979da3897458e27 inspected.
2. Transactional engine: installation fingerprint, lock, clean archive backup, bounded extraction, staging, journal, version-safe rollback/restore. Fixture tests before live writes.
3. Themes (Slate, Ink, Paper), custom CSS, small DOM adapter/runtime, keyboard controls and sleep timer. Profiles and local manifest packages; limited Spicetify CSS import.
4. Accessible custom-titlebar desktop GUI; signed automatic updater, release repository/feed. Review pending changes before apply.
5. Package, install, inspect GUI; live Spotify apply/switch/play/navigation/restore; release benchmarks and evidence.

## Acceptance
- No login/cache/library/prefs mutations; existing Spicetify config/backups preserved.
- Apply is idempotent. Interrupted rename transaction rolls back. Corrupt/stale backup, unsafe archives, untested versions fail closed.
- CLI and GUI share engine; Spotify works after manager closes.
- Three themes, two deliberately installed extensions, named persistent profiles/import/export, search/manifests/local imports.
- Restore matching stock archive behavior and preserve previous customization separately.
- Signed updater checks on launch and bounded interval only while manager is open. Spotify update invalidation detected on refresh/focus/apply.
- Packaged Windows installer/source/docs; real runtime evidence separately from fixture results. Repeated startup/idle CPU/working-set metrics, report failures honestly.

## Design decisions
Skill search performed. User brief overrides generated green accent and marketing layout: blue #78a9ff, charcoal neutral surfaces, San Francisco family with Segoe UI fallback, compact functional navigation. Complete layout from the second v0 design pass is implemented in `src/main.tsx` / `src/index.css`. No green status controls. Reduced motion, focus rings, accessible custom selects/dialogs and responsive resizing.

## Verified progress

18 transaction fixtures and four runtime tests pass. Live Slate, playback shortcuts, one-minute timer pause, matching stock Restore and protected hash checks passed on Spotify 1.3.4.258. Marketplace discovery and a converted snippet installation passed. The complete v0 0.1.1 manager is packaged, installed, and all screens/dialogs inspected. Profile GUI persistence and incompatible-package blocking were checked. Startup and idle CPU meet measured targets; the 200 MB combined-memory aim was missed. See verification and performance docs for scope and incomplete comparisons.
