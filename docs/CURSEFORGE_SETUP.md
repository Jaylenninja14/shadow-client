# CurseForge setup

CurseForge's API requires a personal API key. Shadow Client does not include one.

1. Sign in at <https://console.curseforge.com/> and create an API key (for personal, non-commercial use this is free; read and follow CurseForge's terms).
2. In Shadow Client: **Settings → Accounts & services → CurseForge API key** → paste → **Save key**.

How the key is handled:

* Encrypted with Windows DPAPI for your Windows user (`secrets.dat` in the data folder). It cannot be decrypted by another user or on another PC.
* Sent only in the `x-api-key` header to `api.curseforge.com`. Never logged, never shown again, never exported with profiles or diagnostics.
* For development you can instead set the environment variable `SHADOW_CURSEFORGE_API_KEY` (never commit it).

## Download restrictions

Some authors disable third-party downloads (`downloadUrl` is `null` in the API). Shadow Client respects that: such files are listed as **manual downloads** with a button that opens the official CurseForge page and the target folder. It never tries to work around the restriction.

## Without a key

Modrinth browsing/installing works fully without any key. CurseForge modpack zips whose mods are listed by ID can only be installed with a key (the launcher explains this instead of failing silently).
