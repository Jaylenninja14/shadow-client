# Security & privacy overview

## No account, no telemetry

* There is no Shadow Client account and no analytics/telemetry. The launcher talks only to the services in the table below, and only when you use a feature that needs them.
* Nothing you own (worlds, logs, screenshots, credentials) is ever uploaded. Diagnostics reports are shown to you first and only copied to the clipboard when you click *Copy*.

| Service | When |
|---|---|
| Mojang (`piston-meta`, `libraries.minecraft.net`, `resources.download.minecraft.net`, `launchermeta`) | listing/installing versions, Java runtimes |
| Fabric / Quilt meta and Maven | creating/launching Fabric/Quilt profiles |
| Modrinth API + CDN | browsing/installing/updating mods, packs, resource packs, shaders |
| CurseForge API | only with your API key, when you browse/install from CurseForge |
| Microsoft / Xbox / Minecraft services | only when you sign in with Microsoft and when tokens are refreshed for a launch |

## Credentials

* Microsoft passwords are never seen: sign-in uses the device-code flow on microsoft.com.
* Refresh/access tokens and the CurseForge key are encrypted with **Windows DPAPI** (current user) in `secrets.dat`.
* Tokens are never written to logs. The access token passed to Minecraft on its command line is redacted in every log line; all logs and error messages pass through a redaction filter (JWTs, Bearer headers, Microsoft refresh tokens, API keys).
* The official launcher's account files (`launcher_accounts*.json`, `*credentials*`) are never opened, copied or modified.

## Downloads

* HTTPS only (plain HTTP is refused, except loopback addresses used by automated tests).
* Every file is verified against its published SHA-1/SHA-512 before use; failed files are deleted, never used.
* Files are written to `<name>.part` and renamed only after verification; already-verified files are never re-downloaded.
* Disk space is checked before large jobs. Concurrency is bounded and every job can be cancelled.
* Modpack (.mrpack) files may only come from the hosts the format allows (`cdn.modrinth.com`, `github.com`, `raw.githubusercontent.com`, `gitlab.com`).

## Path safety

Every path coming from outside — modpack indexes, override folders, zip entries, asset indexes, runtime manifests, profile imports — is validated: absolute paths, `..`, drive letters, alternate data streams, reserved device names (CON, NUL, …), trailing dots/spaces and junction escapes are rejected. Automated tests cover malicious packs.

## Nothing downloaded is executed

The only program the launcher starts is Java running Minecraft. Mod loader installers (Forge/NeoForge) are **not** run; JVM arguments are accepted only from you (validated: no agents, no classpath overrides), never from mod or pack metadata. The launcher does not need administrator rights.

## Your files

* Existing installations you *manage* are only changed when you install/remove/update something; replaced files are backed up first.
* Deleting a profile never deletes worlds: `saves` and `screenshots` are moved to `preserved-worlds`, the rest goes to the Recycle Bin. Profiles that point at existing folders only lose their entry.
* Uninstalling never deletes worlds or Minecraft files (see INSTALL.md).

## IPC surface

The UI can only call the launcher's own commands (Tauri capabilities allow core events/window and native file dialogs). Opening folders and URLs goes through Rust commands that only allow launcher/game folders and `https` links.

## Reporting

Use Settings → Advanced → Diagnostics to gather a redacted report when asking for help.
