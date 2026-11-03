// Copy the built installers into website/downloads with stable names and write files.json
// (version, sizes, SHA-256) for the download page. Run after `npm run app:build` and
// `npm run portable`. Usage: npm run website
import { createHash } from "node:crypto";
import { copyFileSync, existsSync, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");
const pkg = JSON.parse(readFileSync(path.join(root, "package.json"), "utf8"));
const branding = JSON.parse(readFileSync(path.join(root, "branding.json"), "utf8"));
const target = process.env.CARGO_TARGET_DIR ? path.resolve(process.env.CARGO_TARGET_DIR) : path.join(root, "target");
const bundle = path.join(target, "release", "bundle");
const v = pkg.version;
const sources = {
  setup: [path.join(bundle, "nsis", `${branding.productName}_${v}_x64-setup.exe`), "Shadow-Client-Setup.exe"],
  msi: [path.join(bundle, "msi", `${branding.productName}_${v}_x64_en-US.msi`), "Shadow-Client.msi"],
  portable: [path.join(root, "release", `ShadowClient-${v}-portable-win-x64.zip`), "Shadow-Client-Portable.zip"],
};

const out = path.join(root, "website", "downloads");
mkdirSync(out, { recursive: true });
const files = {};
for (const [key, [src, name]] of Object.entries(sources)) {
  if (!existsSync(src)) {
    console.warn(`missing ${src} (skipped)`);
    continue;
  }
  const dest = path.join(out, name);
  copyFileSync(src, dest);
  const sha256 = createHash("sha256").update(readFileSync(dest)).digest("hex");
  files[key] = { name, size: statSync(dest).size, sha256 };
  console.log(`${name}  ${files[key].size} bytes  ${sha256}`);
}
writeFileSync(path.join(out, "files.json"), JSON.stringify({ version: v, base: branding.downloadsUrl ?? null, files }, null, 2) + "\n");
copyFileSync(path.join(root, branding.logo), path.join(root, "website", "images", "logo.svg"));
console.log(`website/downloads ready (${Object.keys(files).length} files).`);
