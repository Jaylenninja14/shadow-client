# Third-party notices

## Built-in mods (embedded in the launcher)

These two mods are redistributed unmodified, byte-identical to their official Modrinth releases (verified by SHA-1/SHA-512), under the MIT License.

### Marlow's Crystal Optimizer 2.0.0 (+mc1.21.11)

* Project: <https://modrinth.com/mod/marlow-crystal-optimizer> · Source: <https://github.com/Bram1903/MarlowsCrystalOptimizer>
* SHA-1 `ff4b0bbfaca3ba35e473b1cc9a1ea5b52cb479f8`

```
MIT License

Copyright (c) 2026 Bram and Marlow

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

### Always Shield 1.2.0 (+mc1.21.11, Fabric)

* Project: <https://modrinth.com/mod/always-shield> · Source: <https://github.com/JustMili0/Always-Shield> · Author: JustMili
* SHA-1 `ee2117eb13caf72d4d58f95c80bdf2d1b1a5fe01`
* License: MIT (as declared in the mod's `fabric.mod.json` and on Modrinth). The jar does not contain a separate license file; the MIT terms above apply with “Copyright (c) JustMili”.

## Mods installed at runtime

Mods, modpacks, resource packs and shaders installed through the launcher are downloaded from Modrinth or CurseForge at the user's request under their own licenses. They are not bundled with Shadow Client. Minecraft game files and Java runtimes are downloaded from Mojang's servers and are never redistributed.

## Libraries used by the launcher

The launcher is built with Tauri, React, Lucide icons and Rust crates including tokio, reqwest, serde, zip, sha1/sha2, sysinfo, winreg, windows-sys, tracing and trash. Their licenses (MIT, Apache-2.0, ISC, BSD) are listed in `package-lock.json` / `Cargo.lock` and can be generated with `cargo about` / `npx license-checker`.

Minecraft is a trademark of Mojang Studios. Shadow Client is not affiliated with Mojang Studios or Microsoft.

## AxolotlClient (no longer used)

Shadow Menu 1.x (`client-mod/`) compiled against AxolotlClient 3.2.2 (LGPL-3.0-or-later, https://codeberg.org/AxolotlClient/AxolotlClient-mod) and the launcher installed it from Modrinth. Since Shadow Menu 2.0 the HUD and client modules are Shadow Client's own code; nothing from AxolotlClient is compiled against, included or installed.
