Shadow Client 0.4.0 for Windows 10 and 11 (64-bit). Run the new installer over your current version; your profiles, worlds and settings stay. Installed copies also update themselves from the "Update needed" message.

**Download:** `Shadow-Client-Setup.exe` is the normal installer. `Shadow-Client.msi` is for managed PCs, and `Shadow-Client-Portable.zip` runs from a folder without installing.

Windows may say "Windows protected your PC" because the installer isn't code-signed yet. Click **More info**, then **Run anyway**.

### New in 0.4.0
- **Share your mod pack:** on Profiles, press **Share** on a profile and send the link. Your friend clicks it, presses **Open in Shadow Client**, and gets a new profile with the same Minecraft version, loader, mods, resource packs and shaders, downloaded from Modrinth and CurseForge. They see the full list before anything installs. Nothing is uploaded: the pack is inside the link. You can also paste a link in Profiles → **Add from share link**.
- **Add an existing profile by copying it:** Profiles → **Add existing profile** (and the first-start screen) now copies the profile's mods, settings and config into a new Shadow Client profile with the same name. Your original folder isn't changed, and the copy gets the built-in mod menu like any other profile.
- **The MSI version updates itself:** copies installed with `Shadow-Client.msi` now update from the "Update needed" message too, through Windows Installer (Windows may ask for admin rights if it was installed for all users). MSI copies on 0.3.2 or older need this one update installed by hand first.
- Mods you added by hand are no longer downloaded twice when the launcher adds the built-in mods (for example Fabric API).

### Checksums (SHA-256)
```
7de29c6a23a389fb8fde3303d5419513dcf7ffc20798dfbaf899e84851ee8ca7 *Shadow-Client-Setup.exe
37f25dfa2fc9856d48ed5d099ce4938e531a324e63c64f8c639ec0fcf32758f8 *Shadow-Client.msi
a46ea3b6ff2c4f9d48139eb104d31485fc39183805ccdb5d3c4f5dea75a15fde *Shadow-Client-Portable.zip
```

Not an official Minecraft product. Not approved by or associated with Mojang or Microsoft.
