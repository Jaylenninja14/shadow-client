# Installing Shadow Client

## Requirements

* Windows 10 (1809+) or Windows 11, 64-bit.
* Microsoft Edge WebView2 Runtime. It is part of Windows 11 and current Windows 10; the installer downloads Microsoft's official bootstrapper if it is missing. The portable build needs it already installed.
* No Java needed up front: each Minecraft version declares the Java it needs, and Shadow Client can download **Mojang's official runtime** for it (you are asked first; every file is hash-verified).
* About 1 GB free disk space per Minecraft version (shared between profiles) plus your mods/worlds.
* No administrator rights. The installer installs per-user.

## Option A — Installer (recommended)

1. Run `Shadow Client_<version>_x64-setup.exe` (NSIS) **or** `Shadow Client_<version>_x64_en-US.msi`.
2. Follow the prompts. It installs to `%LOCALAPPDATA%\Shadow Client` (NSIS, per-user) and adds a Start-menu shortcut.
3. Start **Shadow Client**. On first start it scans for existing installations (read-only) and offers:
   * **A** manage an existing installation in place,
   * **B** create a separate optimized profile (Fabric + Sodium + compatible performance mods + built-in mods),
   * **C** copy an existing setup into a new isolated instance,
   * **D** open the official Minecraft Launcher.

Windows SmartScreen may warn because the build is not code-signed. Choose “More info → Run anyway” only if you built it yourself or trust the source.

## Option B — Portable

Unzip `ShadowClient-<version>-portable-win-x64.zip` anywhere (e.g. a USB drive) and run `Shadow Client.exe`. Because `portable.txt` sits next to the exe, **all data** (settings, profiles, game files, worlds) is stored in the `data` folder beside it. Delete `portable.txt` to use the normal location instead.

## Where data lives

| Item | Installed build | Portable build |
|---|---|---|
| Settings, profiles, accounts (encrypted) | `%APPDATA%\ShadowClient` | `data\` next to the exe |
| Profile game folders (mods, worlds…) | `%APPDATA%\ShadowClient\instances\<id>` | `data\instances\<id>` |
| Shared game files | `…\libraries`, `…\assets`, `…\versions`, `…\runtime` | same under `data\` |

Existing installations you choose to **manage** stay where they are (e.g. `%APPDATA%\.minecraft`).

## Uninstalling

Use *Settings → Apps → Installed apps → Shadow Client → Uninstall* (or the uninstaller in the install folder).

The uninstaller removes the program only. It **never deletes worlds or Minecraft files**: your profiles, instances and worlds stay in `%APPDATA%\ShadowClient`, and folders like `%APPDATA%\.minecraft` are never touched. The NSIS uninstaller's optional “delete application data” checkbox only clears the WebView cache (`%LOCALAPPDATA%\com.shadowclient.launcher`), because the launcher's real data folder is deliberately different. To remove everything, delete `%APPDATA%\ShadowClient` yourself after copying any worlds you want to keep (`instances\<id>\saves` and `preserved-worlds`).
