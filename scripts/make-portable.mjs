// Build the portable zip from an existing release build (run `npm run app:build` first).
// The portable build keeps all data in a `data` folder next to the exe (triggered by portable.txt).
import { copyFileSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const pkg = JSON.parse(readFileSync(path.join(root, "package.json"), "utf8"));
// CARGO_TARGET_DIR is honoured like cargo/tauri do (e.g. a second build dir while the app runs)
const target = process.env.CARGO_TARGET_DIR ? path.resolve(process.env.CARGO_TARGET_DIR) : path.join(root, "target");
const exe = path.join(target, "release", "shadow-client.exe");
if (!existsSync(exe)) {
  console.error("Release exe not found. Run `npm run app:build` first.");
  process.exit(1);
}
const name = `ShadowClient-${pkg.version}-portable-win-x64`;
const outDir = path.join(root, "release", name);
rmSync(outDir, { recursive: true, force: true });
mkdirSync(outDir, { recursive: true });
copyFileSync(exe, path.join(outDir, "Shadow Client.exe"));
writeFileSync(
  path.join(outDir, "portable.txt"),
  "This file switches Shadow Client to portable mode: settings, profiles, game files and worlds are stored in the 'data' folder next to the exe.\r\nDelete this file to use %APPDATA%\\ShadowClient instead.\r\n",
);
copyFileSync(path.join(root, "docs", "PORTABLE.txt"), path.join(outDir, "README.txt"));
copyFileSync(path.join(root, "THIRD_PARTY_NOTICES.md"), path.join(outDir, "THIRD_PARTY_NOTICES.md"));
const zip = path.join(root, "release", `${name}.zip`);
rmSync(zip, { force: true });
// Windows 10+ ships bsdtar (System32\tar.exe), which can write zip archives. Use it explicitly:
// Git Bash puts GNU tar first on PATH, which cannot write zips and misreads "C:" paths.
const systemTar = path.join(process.env.SystemRoot ?? "C:\\Windows", "System32", "tar.exe");
const tar = existsSync(systemTar) ? systemTar : "tar";
execFileSync(tar, ["-a", "-c", "-f", zip, "-C", path.join(root, "release"), name], { stdio: "inherit" });
console.log(`Portable build: ${zip}`);
