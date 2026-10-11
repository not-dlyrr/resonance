# Automatic signed updates

Default feed: `https://github.com/not-dlyrr/resonance/releases/latest/download/latest.json`.

The manager checks automatically on launch, then every six hours while open, when enabled in Settings. Checks do not install silently; an available signed release offers Update and restarts the manager on installation. No background service is required. Spotify version changes are detected separately on focus/status refresh and before mutation; profiles are retained and compatible reapply is offered.

Tauri updater signatures are mandatory. The public key is in `src-tauri/tauri.conf.json`; installer `.sig` files are published beside releases. HTTPS is required for user-selected feeds. An unsigned/tampered installer cannot pass the embedded release key. Windows Authenticode signing is separate; initial builds are not Authenticode-signed.

## Release maintainers

The private signing key is deliberately outside the source tree and repository. Keep it safe; losing it prevents future updates to existing installations. Never commit keys. Current key location is provided locally in delivery notes rather than the public repository.

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY='ABSOLUTE_PATH_TO_PRIVATE_KEY'
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD=''
npm ci
npm run tauri build
```

Update package version, Rust package versions, and Tauri config together. Publish installer and signature plus `latest.json`:

```json
{
  "version": "0.1.1",
  "notes": "Release notes",
  "pub_date": "2026-10-11T09:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "url": "https://github.com/not-dlyrr/resonance/releases/download/v0.1.1/Resonance_0.1.1_x64-setup.exe",
      "signature": "BASE64_CONTENT_OF_THE_SIG_FILE"
    }
  }
}
```

Run tests and live release verification before publishing. See the official [Tauri updater documentation](https://v2.tauri.app/plugin/updater/) for format/signing details.
