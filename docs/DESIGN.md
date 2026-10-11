# Design provenance and fonts

The complete desktop design was generated with v0 in [Resonance UI design](https://v0.app/chat/jt0HdqhwYoM). The second design pass replaced the previous desktop layout: `src/index.css` contains the v0 design with accessible production states; `src/main.tsx` uses its app shell, dedicated Store navigation, theme player previews, appearance rows, extension lists, profile cards, Settings tool cards, and redesigned package, CSS, update, recovery, profile, and apply dialogs. These are connected to the actual Rust engine. Radix provides keyboard accessible selects and focus-trapped dialogs. Generated sample package records and claims are not shipped.

The manager uses the San Francisco family when installed. Settings supports importing a local WOFF2, OTF, or TTF file for SF Pro. The imported font is stored privately on this computer, embedded in the manager style, and copied into the Spotify customization on the next apply. SF Pro, SF Pro Text, SF Pro Display, and SF Mono are selectable for Spotify. Only one local imported font is supported; it supplies SF Pro. Other variants use locally installed families or the fallback.

No Apple font files are shipped or downloaded. Use a font you have permission to use. Without one, Windows uses Segoe UI. A local import affects the next Spotify apply; existing customizations retain their current font until then.
