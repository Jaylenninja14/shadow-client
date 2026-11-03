# Troubleshooting

Start with **Home → Pre-launch check**: it lists every problem it can detect (missing Java, mod conflicts, missing dependencies, account, memory) with a button that fixes or explains it. When a launch fails, the launcher shows a diagnosis taken from the game's own output and keeps the full log.

**Logs:** Settings → Advanced → Diagnostics & logs. Logs never contain tokens; the viewer redacts again. *Create diagnostics report* also removes your Windows user name and copies a shareable summary. Nothing is uploaded.

| Symptom | Likely cause | What to do |
|---|---|---|
| PLAY is disabled, “Java N needed” | Required Java not installed | Click **Download Java** (official Mojang runtime, hash-verified) or pick a runtime in Profiles → Edit |
| “The selected Java is too old…” | Manual Java override older than the version needs | Profiles → Edit → Java: *Automatic* |
| “Java could not reserve the requested memory” | Max memory too high for free RAM | Lower *Maximum memory* (Profiles → Edit) or leave it empty for the recommendation |
| “The mod loader refused to start because of a mod problem” | Missing dependency / incompatible mod | Mods → Installed shows warnings per mod; install the named dependency or disable the mod |
| “A mod failed to patch the game (Mixin error)” | Mod not compatible with this Minecraft version or another mod | Disable recently added mods; check Mods → Updates |
| “The graphics driver could not create an OpenGL context” | Old/broken GPU driver, or Java on the wrong GPU | Update the driver from Intel/AMD/NVIDIA; Windows Settings → System → Display → Graphics → add `javaw.exe` and choose *High performance* |
| Low FPS on integrated graphics | Settings too high | Performance → apply **Low-end**; make sure Sodium is installed (Mods → Performance mods); avoid shaders |
| Stutter every few seconds | Heap too large/small or background apps | Keep memory at the recommendation; use the *Mojang defaults (G1)* GC preset |
| Search says “CurseForge needs an API key” | No key configured | See docs/CURSEFORGE_SETUP.md |
| “Microsoft sign-in needs one setup step” | No approved Azure client ID | See docs/AUTH_SETUP.md, or use the official launcher |
| “Minecraft services rejected this app's sign-in” | Azure app not yet approved by Mojang | Wait for approval of your client ID |
| Install blocked: “has no release for Minecraft X” | The mod has no build for this version/loader | The dialog lists versions that do have builds and can create a matching profile |
| Install blocked: “Conflict: sodium ↔ optifabric” | Two mods replace the same renderer | Remove one of them first |
| “This file is for a different Minecraft version” after download | Mod's own metadata contradicts the API | Nothing was installed; report it to the mod author |
| Download failed / hash mismatch | Network or server problem | Downloads → **Retry** (verified files are skipped, partial files resume) |
| Game window opens and closes | See the diagnosis on Home; open the log | Usually Java/memory/mod issue as above |
| Changes to resource packs/presets are “undone” | Minecraft rewrote options.txt on exit | Close the game before changing settings in the launcher |
| Something changed that you want back | Backups | Mods → Backups (mods), Performance → Settings backups (options.txt; “Original” = state before the launcher first changed it) |

## Repairing files

Versions → **Repair** re-verifies every library, asset and the client jar against Mojang's hashes and re-downloads anything wrong.

## Resetting

Settings → **Reset to defaults** resets launcher settings only (not profiles, mods, accounts or worlds).
