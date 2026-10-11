# Complete v0 restyle, 0.1.1

The first design adaptation reused the old screen arrangement. This revision replaces the layout and all screen/dialog structures using the second v0 design pass from chat `jt0HdqhwYoM`.

- New desktop shell, custom titlebar, compact navigation and dedicated Spicetify Store view.
- Themes use v0's player previews and appearance setting rows.
- Extensions have bordered entries, installation/removal controls, enabled switches and timer configuration.
- Profiles show actual saved setups with use/export/remove controls.
- Settings organizes SF fonts, CSS, recovery, signed updates and diagnostics into grouped rows and tools.
- All seven dialogs use the same design: apply, package review, profile save, CSS editor, recovery, recovery confirmation and updater.
- Search, offline states, errors, pending discard and file verification remain connected to real engine state.
- SF Pro Text / Display / Pro with Segoe UI fallback. Local SF font import remains available. SF Mono is used for source and CSS where installed.

Verified the installed release: all five screens and the apply, package review, save profile, CSS, recovery, and updater dialogs were visually inspected. The font menu contains eight choices and matches its 172px trigger width. Rendered the Themes and Settings screens at a 760 × 580 viewport; controls and the footer remain reachable. A real Marketplace package with unmapped selectors displays its exact source/hash and has installation disabled. A profile saved through the GUI persisted across reinstall/restart. Frontend production build, 18 transaction tests and four runtime tests pass.
