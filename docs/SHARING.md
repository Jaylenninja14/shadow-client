# Sharing Shadow Client with friends

## What to send

| File | Size | Best for |
|---|---|---|
| `release\Shadow Client_<ver>_x64-setup.exe` | ~4 MB | most people: normal installer, Start-menu shortcut, uninstaller |
| `release\Shadow Client_<ver>_x64_en-US.msi` | ~5 MB | people/IT who prefer MSI |
| `release\ShadowClient-<ver>-portable-win-x64.zip` | ~4 MB | USB sticks / no install: unzip and run `Shadow Client.exe` |

Send it any way you like (Discord, Google Drive, OneDrive, a GitHub release…). The app downloads Minecraft, Java and mods itself, from the official sources, so the files stay small. Nothing from your own PC (profiles, worlds, accounts, keys) is inside them.

## What your friends will see

* **Windows SmartScreen warning** (“Windows protected your PC”) because the build isn't code-signed. They click *More info → Run anyway*. To remove the warning you'd need a code-signing certificate (paid), then sign the installer with `signtool` or configure `bundle.windows.certificateThumbprint` in `src-tauri/tauri.conf.json`.
* Requirements: Windows 10/11 64-bit. WebView2 is installed automatically by the installer if missing.
* First start shows the welcome screen (scan of existing installs, choices A–D).

## Playing online with a shared build

Out of the box your friends can create profiles, install mods/packs and play **offline** (singleplayer/LAN testing), or keep using the official launcher for online play.

For **Microsoft sign-in in the shared build**:

1. Register an Azure app and get it approved for Minecraft (docs/AUTH_SETUP.md).
2. Put its Application (client) ID into `branding.json` → `"msaClientId": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"`.
3. Rebuild (`npm run app:build && npm run portable`) and share the new files. New installs then have Microsoft sign-in ready; it's still changeable under Settings.

Client IDs are public identifiers, so baking one in is fine. **Never** bake in a CurseForge API key: those are personal; each friend adds their own under Settings → Accounts & services (or just uses Modrinth, which needs no key).

## Built-in mods

They're always on in every Fabric/Quilt profile Shadow Client creates (your friends can't switch them off by accident) and aren't listed, neither in the launcher's mod list nor in the in-game Mods screen. The two built-in mods (Marlow's Crystal Optimizer, Always Shield) are MIT-licensed, so redistributing them inside the app is allowed; their notices are in `THIRD_PARTY_NOTICES.md`, which ships with the portable zip.

## Updating your friends

Bump `"version"` in `package.json`, `src-tauri/tauri.conf.json` and the Cargo workspace (`Cargo.toml`), rebuild, and send the new installer. Installing over the old version keeps everyone's profiles and worlds.
