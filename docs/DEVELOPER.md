# Developer guide

## Prerequisites (Windows 10/11)

| Tool | Version used | Install |
|---|---|---|
| Rust (MSVC toolchain) | 1.99 stable | <https://rustup.rs> (`rustup-init.exe -y`) |
| Visual Studio Build Tools / VS with “Desktop development with C++” | VS 2026 (18) | needed by Rust's MSVC linker |
| Node.js | 24.x (npm 12) | <https://nodejs.org> |
| WebView2 Runtime | any current | built into Windows 11 |

## Exact build steps

```bash
npm install
npm run typecheck          # TypeScript
npm test                   # frontend unit tests (vitest)
cargo test -p pc-core      # engine unit + integration tests (mock servers, temp dirs)
npm run app:build          # release exe + NSIS installer + MSI  (tauri build)
npm run portable           # portable zip from the release exe
npm run website            # installers + checksums into website/downloads
```

Outputs:

* `target/release/shadow-client.exe`
* `target/release/bundle/nsis/Shadow Client_<ver>_x64-setup.exe`
* `target/release/bundle/msi/Shadow Client_<ver>_x64_en-US.msi`
* `release/ShadowClient-<ver>-portable-win-x64.zip`

The first `tauri build` downloads the NSIS and WiX toolsets used by Tauri's bundler (official Tauri mechanism).

## Running in development

```bash
npm run app:dev
```

Vite serves the UI on `127.0.0.1:1420` (port 8080 is avoided on purpose). To keep test data away from your real launcher data, set `SHADOW_DATA_DIR` before starting:

```bash
SHADOW_DATA_DIR="$TEMP/pc-dev-data" npm run app:dev
```

For UI automation/inspection, `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` exposes the WebView over the Chrome DevTools Protocol.

## Layout

```
crates/pc-core/      engine (no UI dependency) — all logic and tests
  data/              compat-db.json, builtin-mods.json, builtin/*.jar (embedded), bedrock/ (Shadow panorama)
  src/tests_integration.rs   end-to-end tests against mock HTTP servers
src-tauri/           Tauri shell: commands.rs (IPC), state.rs, lib.rs (setup, logging, events)
src/                 React UI: api.ts (typed commands), store.tsx (event-driven state), pages/, components/
client-mod/          Shadow Menu, the in-game mod menu (Fabric); tools/IconGen.java and tools/PackGen.java
server-plugin/       ShadowCombat (Paper plugin)
website/             download page; `npm run website` fills website/downloads
branding.json        name, identifier, accent, logo → `npm run branding`
docs/                user and developer documentation
scripts/             apply-branding.mjs, make-portable.mjs, build-website.mjs, serve-website.mjs
```

### Shadow Menu art

`client-mod/tools/IconGen.java` draws the menu's white module icons; `client-mod/tools/PackGen.java`
draws the texture packs inside the mod and the Shadow background (Java panorama in the mod,
Bedrock panorama in `crates/pc-core/data/bedrock/`, and `src/assets/shadow-background.jpg` for the
launcher's main menu). Run both from `client-mod/` with `java tools/<Name>.java`; after rebuilding the
mod, copy the jar into `crates/pc-core/data/builtin/` and update its hashes in `builtin-mods.json`.

See docs/ARCHITECTURE.md for design decisions.

## Rebranding

Edit `branding.json` (name, identifier, tagline, accent colour, logo path) and replace `branding/logo.svg`, then:

```bash
npm run branding    # updates tauri.conf.json, index.html title, regenerates all icons
```

The UI reads the name, tagline and logo from `branding.json` at build time, and the default accent colour comes from it too.

## Adding a compatibility rule or performance mod

`crates/pc-core/data/compat-db.json`:

* `conflicts` — pairs of mod ids that must not be installed together (blocked with the given reason).
* `rendererMods` — mods that replace the renderer; only one may be active.
* `projectToModId` — Modrinth project id → mod id (lets the resolver detect conflicts before download).
* `performanceMods` — the Performance Mods page and automatic setup list. Availability per version is always checked live; never list a version here.

## Adding a built-in mod

1. Verify the license allows redistribution.
2. Put the exact jar in `crates/pc-core/data/builtin/`.
3. Add an entry to `crates/pc-core/data/builtin-mods.json` with SHA-1/SHA-512/size and the Modrinth project id.
4. Add the file to the `embedded()` match in `crates/pc-core/src/mods/builtin.rs`.
5. `cargo test -p pc-core builtin` verifies the hashes.
6. Add the license text to `THIRD_PARTY_NOTICES.md`.

## Tests

`cargo test -p pc-core` runs ~99 tests, all against temp directories and local mock servers (mockito or a small Range-capable HTTP server). They never touch the real `.minecraft`. Coverage includes: install discovery, version JSON parsing/rules/merging, profile create/delete safeguards/isolation, mod install/remove/restore, dependency resolution, conflicts, download interruption + Range resume + cancel/retry, hash verification, cache TTL + offline fallback, path traversal (incl. malicious .mrpack), Java selection, memory validation, Microsoft auth error paths, launch failures (fake Java) and log redaction, options.txt backups/restore, .mrpack and CurseForge pack install/update/export, built-in mod hashes.

## Secrets in development

Never commit keys. Use `SHADOW_CURSEFORGE_API_KEY` in your shell for CurseForge during development; the app otherwise stores the key with DPAPI.
