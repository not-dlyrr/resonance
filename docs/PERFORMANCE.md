# Release measurements, 2026-10-11

Windows x64 on Ryzen 7 3700X / RX 580 / 16 GB RAM. Release manager 0.1.1. Three warm process restarts, usable theme cards and installation status, four seconds settle, eight-second idle windows; CDP enabled. No release compiler ran during these manager measurements.

| Metric | Observed | Initial target |
|---|---|---|
| Median usable startup | 0.789 seconds | Under 2 seconds |
| Startup range | 0.736–0.804 seconds | Repeated observations |
| Idle CPU, total machine | 0.000% reported in short windows | Under 1% |
| Combined manager/WebView working set | 407.5 MiB median | Aim under 200 MB — missed |
| Core runtime gzip | 1115 bytes | At most 100 KB |

CPU readings at zero mean no measurable delta at the process timer resolution in those eight-second windows, not a guarantee of zero CPU. Working sets sum shared pages repeatedly across seven processes; this is still the declared combined metric and the memory target is missed. WebView dominates this footprint. The injected runtime uses events and one-shot timers; disabled extensions are absent from applied JavaScript. There is no permanent manager animation/service or continuous DOM scan.

Raw observations are in `docs/measurements/manager.json`. The reproduction driver is supplied in `tests/measure_release.py`; install `psutil` and `websocket-client`, then run `python tests/measure_release.py bench manager ABSOLUTE_EXE_PATH 9223 manager`. Run from the repository root. Use the actual process executable path: Codex's Windows package virtualization exposes a different on-disk path from the shell's local-app-data alias. A first attempt that matched no processes was rejected and overwritten.

Stock Spotify 1.3.4.258 was also measured with the same restart/CPU/working-set method, paused on Home, in `stock.json`. Startup was approximately 3.7–4.4 seconds, but CPU/memory varied substantially as the client loaded recommendations. The older Spicetify observation in `spicetify.json` is excluded: it ran alongside a compiler and its idle playback state was not established. A controlled three-way Spotify performance comparison has therefore not been demonstrated. These observations do not support a claim that Resonance makes Spotify faster or smaller.
