# Shadow Client – Architecture & Design Decisions

## Environment found during Phase 1 (developer machine)

| Item | Finding | Consequence |
|---|---|---|
| OS | Windows 11 Home 10.0.26200 | Windows 10/11 is the primary target |
| Node | v24.21.0, npm 12 | Vite + React frontend |
| Rust | not installed → installed rustup 1.99.1 (stable, MSVC) | Tauri 2 backend |
| MSVC | Visual Studio 18 (2026) Community with VC tools | Rust links with MSVC |
| WebView2 | 156.x present | Tauri runtime available; installer still bundles the bootstrapper for machines without it |
| Java | Oracle JDK 26.0.1, Oracle JRE 1.8.0_491, Mojang runtime inside the Microsoft Store launcher package | Java is selected per Minecraft version; the Mojang runtime is downloadable on demand |
| Official launcher | Microsoft Store edition (`Microsoft.4297127D64EC6_8wekyb3d8bbwe`) | “Open official launcher” uses the Store app id |
| `.minecraft` | `%APPDATA%\.minecraft` holds launcher JSON files and version JSONs (26.1.2 … 26.3-snapshot-5, fabric-loader-0.20.1-1.21.11) but no libraries/assets | Detection must not assume a complete game tree |
| Ports | 8080 is used by Steam and the user’s server panel | Dev server uses 1420 |

## Stack decision

**Tauri 2 + React 19 + TypeScript (Vite) for the UI, Rust for every launcher service.**

* Tauri uses the system WebView2, so the launcher idles at a fraction of Electron’s memory and ships as a few-MB installer. Nothing in the spec required Electron.
* All logic lives in the Rust crate **`crates/pc-core`**, which has *no* Tauri dependency. It is unit/integration tested with `cargo test` using temp directories and local mock HTTP servers. `src-tauri` is a thin command/event layer.
* Java is used only as the game runtime; no Java code is shipped.
* Styling is hand-written CSS with CSS variables (no component library, no heavy blur/animation) to stay light on integrated GPUs.

## Repository layout

```
branding.json            Name, colours, tagline – single place to rebrand (scripts/apply-branding.mjs)
src/                     React UI (pages/, components/, api.ts typed command wrappers)
src-tauri/               Tauri shell: commands.rs (IPC), state.rs, tauri.conf.json, resources/
crates/pc-core/          Launcher engine (no UI)
  src/paths.rs           Data-root resolution (portable mode, %APPDATA%\ShadowClient)
  src/safepath.rs        Path-traversal protection used by every archive/override/asset write
  src/http.rs            HTTP client, User-Agent, disk cache, rate-limit handling
  src/download.rs        Download manager: bounded concurrency, resume, hash verify, cancel/retry, history
  src/minecraft/         Version manifest & JSON parsing, rules, install, launch, loaders, detection, options.txt
  src/java.rs            Java discovery + Mojang runtime download
  src/memory.rs          RAM recommendation & validation
  src/jvmargs.rs         JVM argument validation / defaults
  src/auth.rs            Microsoft OAuth device-code flow, Xbox/XSTS, entitlement check, offline profiles
  src/secrets.rs         DPAPI-protected storage for tokens and the CurseForge key
  src/profiles.rs        Isolated profiles, delete safeguards, duplicate, export/import
  src/mods/              Jar metadata, installed index, Modrinth, CurseForge, resolver, compat DB, built-ins
  src/performance/       Hardware detection, system benchmark, presets
  src/modpacks.rs        .mrpack and CurseForge pack install/export
  src/resources.rs       Resource packs & shader packs
  src/integration.rs     Official launcher detection / open / profile registration
docs/                    User + developer documentation
```

## Data layout (`%APPDATA%\ShadowClient`, or `data\` next to the exe in portable mode)

```
settings.json, state.json         settings & UI state (atomic writes)
profiles/<id>.json                one file per profile
instances/<id>/                   default game directory of a managed profile (mods, config, saves …)
libraries/ assets/ versions/      shared, hash-verified game files (never duplicated per profile)
runtime/<component>/              Mojang Java runtimes downloaded on demand
cache/http/                       API response cache (TTL based)
logs/                             launcher logs (secrets never written; viewer redacts again)
preserved-worlds/                 worlds rescued from deleted profiles
secrets.dat                       DPAPI-encrypted (current Windows user) tokens & API keys
```

The data root intentionally differs from Tauri’s bundle-identifier folder, so the uninstaller’s optional “delete app data” checkbox can never remove instances or worlds.

## Key design rules

1. **Never fake anything.** Every number shown (download counts, compatibility, FPS, benchmark) comes from an API or a real measurement. In-game FPS is not observable from outside the game without a companion mod, so the UI says so instead of estimating.
2. **Isolation by default.** New modded setups get their own game directory. Existing installations are only modified on explicit user action, always with backups.
3. **Compatibility first.** Mods are chosen through the resolver: Modrinth/CurseForge metadata filtered by the profile’s Minecraft version + loader, dependency walk, compat-DB conflict check, then the jar’s own `fabric.mod.json`/`quilt.mod.json` is verified after download and before it is moved into `mods/`.
4. **Safe downloads.** HTTPS only (loopback HTTP allowed for tests), hash verification, `.part` files with HTTP Range resume, disk-space check, bounded concurrency, cancellable jobs, no file is ever executed except the Java runtime launching Minecraft.
5. **Event-driven UI.** Progress is pushed via Tauri events (throttled to ~4 Hz). The only periodic work while the game runs is a 2-second process sample (RAM/CPU of the game process) which stops when the game exits.
6. **Secrets.** Microsoft refresh tokens and the CurseForge key are encrypted with Windows DPAPI. They are never logged, never placed on a command line that is logged, and the log viewer redacts tokens again before display/export.

## External services

| Service | Use | Notes |
|---|---|---|
| piston-meta.mojang.com / libraries.minecraft.net / resources.download.minecraft.net | versions, libraries, assets | SHA-1 verified |
| launchermeta.mojang.com java-runtime | Mojang Java runtimes | SHA-1 verified per file |
| meta.fabricmc.net / maven.fabricmc.net | Fabric loader | |
| meta.quiltmc.org / maven.quiltmc.org | Quilt loader | |
| api.modrinth.com / cdn.modrinth.com | mods, packs, resource packs, shaders | User-Agent, cache, 429 handling |
| api.curseforge.com | CurseForge browse/install | needs user-supplied API key; `downloadUrl == null` ⇒ show project page |
| login.microsoftonline.com, user.auth.xboxlive.com, xsts.auth.xboxlive.com, api.minecraftservices.com | Microsoft account login | needs an Azure app (client) id approved for Minecraft – see docs/AUTH_SETUP.md |

## Loader support matrix

| Loader | Install | Launch | Notes |
|---|---|---|---|
| Vanilla | ✅ | ✅ | all versions in Mojang’s manifest, incl. legacy asset layouts |
| Fabric | ✅ | ✅ | primary loader |
| Quilt | ✅ | ✅ | Fabric mods are offered when Quilt-compatible |
| Forge / NeoForge | ❌ (requires running their installer, which this launcher refuses to execute) | ⚠️ experimental: launches versions already installed by the official installer into a `.minecraft` | the UI links to the official installers |
