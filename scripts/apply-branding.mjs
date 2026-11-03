// Apply branding.json to the Tauri config and regenerate icons from the logo.
// Usage: npm run branding
import { execSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const branding = JSON.parse(readFileSync(path.join(root, "branding.json"), "utf8"));
const confPath = path.join(root, "src-tauri", "tauri.conf.json");
const conf = JSON.parse(readFileSync(confPath, "utf8"));

if (!/^#[0-9a-fA-F]{6}$/.test(branding.accent)) throw new Error("branding.accent must look like #12ab34");
if (!/^[a-z0-9]+(\.[a-z0-9-]+)+$/.test(branding.identifier)) throw new Error("branding.identifier must be reverse-DNS, e.g. com.example.launcher");

conf.productName = branding.productName;
conf.identifier = branding.identifier;
conf.app.windows[0].title = branding.windowTitle ?? branding.productName;
conf.bundle.publisher = branding.productName;
writeFileSync(confPath, JSON.stringify(conf, null, 2) + "\n");

const html = path.join(root, "index.html");
writeFileSync(html, readFileSync(html, "utf8").replace(/<title>.*<\/title>/, `<title>${branding.productName}</title>`));

console.log(`Applied branding "${branding.productName}" (${branding.identifier}).`);
console.log("Regenerating icons from", branding.logo);
execSync(`npx tauri icon "${path.join(root, branding.logo)}" -o "${path.join(root, "src-tauri", "icons")}"`, { stdio: "inherit", cwd: root });
for (const extra of ["android", "ios"]) {
  try {
    execSync(process.platform === "win32" ? `rmdir /s /q "${path.join(root, "src-tauri", "icons", extra)}"` : `rm -rf "${path.join(root, "src-tauri", "icons", extra)}"`, { stdio: "ignore" });
  } catch {
    /* not present */
  }
}
console.log("Done. The default accent colour in the app is also read from branding.json at build time.");
