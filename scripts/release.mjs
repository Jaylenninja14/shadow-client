// Publish a Shadow Client version in one go: version bump, tests, build, GitHub release, website.
//
//   npm run release -- 0.2.3            bump to 0.2.3 and publish it
//   npm run release                     publish the version already in package.json
//   options: --skip-tests  --dry-run (build and prepare everything, publish nothing)
//
// Release notes come from website/RELEASE_NOTES_<version>.md (bullet lines starting with "- " also become the
// website's "What's new" entry); without that file a short default is written. Checksums are filled in.
// Needs the GitHub CLI signed in (gh auth login). Open launchers see the update within about 15 minutes.
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { cpSync, existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync, copyFileSync, mkdirSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);
const flags = new Set(args.filter((a) => a.startsWith("--")));
const wanted = args.find((a) => !a.startsWith("--"));
const dryRun = flags.has("--dry-run");
const branding = JSON.parse(readFileSync(path.join(root, "branding.json"), "utf8"));
const repo = branding.githubRepo;
const targetDir = path.join(root, "src-tauri", "target", "b2");
const env = { ...process.env, CARGO_TARGET_DIR: targetDir };

function step(title) {
  console.log(`\n=== ${title}`);
}
function run(cmd, cmdArgs, opts = {}) {
  const r = spawnSync(cmd, cmdArgs, { cwd: root, env, stdio: opts.capture ? "pipe" : "inherit", encoding: "utf8", shell: process.platform === "win32" && !opts.noShell, ...opts });
  if (r.status !== 0 && !opts.allowFail) {
    if (opts.capture) process.stderr.write((r.stdout ?? "") + (r.stderr ?? ""));
    throw new Error(`${cmd} ${cmdArgs.join(" ")} failed (exit ${r.status})`);
  }
  return r;
}
const gh = (ghArgs, opts = {}) => run("gh", ghArgs, { noShell: true, ...opts });
const readJson = (f) => JSON.parse(readFileSync(path.join(root, f), "utf8"));
const writeJson = (f, v) => writeFileSync(path.join(root, f), JSON.stringify(v, null, 2) + "\n");
const sha256 = (f) => createHash("sha256").update(readFileSync(f)).digest("hex");
const escapeHtml = (s) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

// ------------------------------------------------------------------ version
let version = readJson("package.json").version;
if (wanted && wanted !== version) {
  if (!/^\d+\.\d+\.\d+$/.test(wanted)) throw new Error(`Version must look like 1.2.3, got "${wanted}"`);
  step(`Version ${version} -> ${wanted}`);
  const pkg = readJson("package.json");
  pkg.version = wanted;
  writeJson("package.json", pkg);
  const lock = readJson("package-lock.json");
  lock.version = wanted;
  if (lock.packages?.[""]) lock.packages[""].version = wanted;
  writeJson("package-lock.json", lock);
  const conf = readJson("src-tauri/tauri.conf.json");
  conf.version = wanted;
  writeJson("src-tauri/tauri.conf.json", conf);
  const cargoToml = path.join(root, "Cargo.toml");
  const cargo = readFileSync(cargoToml, "utf8");
  const bumped = cargo.replace(/(\[workspace\.package\][^[]*?\nversion = ")[^"]+(")/, `$1${wanted}$2`);
  if (bumped === cargo) throw new Error("Couldn't find the version in Cargo.toml [workspace.package]");
  writeFileSync(cargoToml, bumped);
  version = wanted;
}
const tag = `v${version}`;

// ------------------------------------------------------------------ checks
step("Checks");
gh(["auth", "status"], { capture: true });
const existing = gh(["release", "view", tag, "-R", repo], { capture: true, allowFail: true });
if (existing.status === 0) throw new Error(`${tag} is already published. Pick a new version: npm run release -- <version>`);
console.log(`${tag} is not published yet; publishing to ${repo}${dryRun ? " (dry run: nothing is published)" : ""}.`);

if (!flags.has("--skip-tests")) {
  step("Tests");
  run("npm", ["run", "typecheck"]);
  run("npm", ["test"]);
  run("cargo", ["test", "-p", "pc-core"]);
}

// ------------------------------------------------------------------ build
step("Build installers");
run("npx", ["tauri", "build"]);
run("node", ["scripts/make-portable.mjs"]);
run("node", ["scripts/build-website.mjs"]);
const downloads = path.join(root, "website", "downloads");
const files = readJson("website/downloads/files.json");
if (files.version !== version || Object.keys(files.files).length !== 3) throw new Error("website/downloads is incomplete; see the build output above.");
const assets = Object.values(files.files).map((f) => path.join(downloads, f.name));
const bundle = path.join(targetDir, "release", "bundle");
mkdirSync(path.join(root, "release"), { recursive: true });
copyFileSync(path.join(bundle, "nsis", `${branding.productName}_${version}_x64-setup.exe`), path.join(root, "release", `${branding.productName}_${version}_x64-setup.exe`));
copyFileSync(path.join(bundle, "msi", `${branding.productName}_${version}_x64_en-US.msi`), path.join(root, "release", `${branding.productName}_${version}_x64_en-US.msi`));

// ------------------------------------------------------------------ notes and changelog
step("Release notes");
const notesFile = path.join(root, "website", `RELEASE_NOTES_${version}.md`);
let notes = existsSync(notesFile)
  ? readFileSync(notesFile, "utf8")
  : `${branding.productName} ${version} for Windows 10 and 11 (64-bit). Run the new installer over your current version; your profiles, worlds and settings stay. Installed copies also update themselves from the "Update needed" message.\n\n### Changes\n- Fixes and improvements.\n\nNot an official Minecraft product. Not approved by or associated with Mojang or Microsoft.\n`;
const sums = "```\n" + assets.map((f) => `${sha256(f)} *${path.basename(f)}`).join("\n") + "\n```";
const block = /### Checksums \(SHA-256\)\n```[\s\S]*?```/;
notes = block.test(notes) ? notes.replace(block, `### Checksums (SHA-256)\n${sums}`) : notes.replace(/\n*(Not an official Minecraft product[\s\S]*)?$/, (tail) => `\n\n### Checksums (SHA-256)\n${sums}\n\n${tail.trim() || ""}\n`);
writeFileSync(notesFile, notes);
console.log(`Notes: ${path.relative(root, notesFile)}`);

const indexFile = path.join(root, "website", "index.html");
let index = readFileSync(indexFile, "utf8");
if (!index.includes(`<h3>${branding.productName} ${version} <span>`)) {
  const bullets = notes.split(/\r?\n/).filter((l) => l.startsWith("- ")).map((l) => escapeHtml(l.slice(2)).replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>").replace(/`(.+?)`/g, "$1"));
  const date = new Date().toLocaleDateString("en-US", { month: "long", day: "numeric", year: "numeric" });
  const article = `        <article>\n          <h3>${branding.productName} ${version} <span>${date}</span></h3>\n          <ul>\n${bullets.map((b) => `            <li>${b}</li>`).join("\n")}\n          </ul>\n        </article>\n`;
  const marker = '<div class="changelog">\n';
  if (!index.includes(marker)) throw new Error("website/index.html has no changelog section");
  index = index.replace(marker, marker + article);
  writeFileSync(indexFile, index);
  console.log(`Added ${version} to the website's "What's new".`);
}

if (dryRun) {
  console.log(`\nDry run done: ${tag} is built and ready in website/downloads; nothing was published.`);
  process.exit(0);
}

// ------------------------------------------------------------------ publish
step(`GitHub release ${tag}`);
gh(["release", "create", tag, "-R", repo, "--target", "main", "--title", `${branding.productName} ${version}`, "--notes-file", notesFile, "--latest", ...assets]);

step("Website");
const site = mkdtempSync(path.join(os.tmpdir(), "shadow-site-"));
try {
  gh(["repo", "clone", repo, site, "--", "-q"]);
  const web = path.join(root, "website");
  for (const f of ["index.html", "share.html", "main.js", "style.css"]) copyFileSync(path.join(web, f), path.join(site, f));
  cpSync(path.join(web, "images"), path.join(site, "images"), { recursive: true });
  mkdirSync(path.join(site, "downloads"), { recursive: true });
  copyFileSync(path.join(downloads, "files.json"), path.join(site, "downloads", "files.json"));
  const git = (gitArgs, opts = {}) => run("git", ["-C", site, ...gitArgs], { noShell: true, ...opts });
  git(["add", "-A"]);
  if (git(["diff", "--cached", "--quiet"], { allowFail: true }).status !== 0) {
    git(["commit", "-q", "-m", `${branding.productName} ${version} on the website`]);
    git(["push", "-q", "origin", "HEAD"]);
    const head = git(["rev-parse", "HEAD"], { capture: true }).stdout.trim();
    process.stdout.write("Waiting for GitHub Pages");
    let built = false;
    for (let i = 0; i < 40 && !built; i++) {
      spawnSync(process.execPath, ["-e", "setTimeout(() => {}, 10000)"]);
      const b = gh(["api", `repos/${repo}/pages/builds/latest`, "-q", '.status + " " + .commit'], { capture: true, allowFail: true }).stdout?.trim() ?? "";
      built = b === `built ${head}`;
      process.stdout.write(".");
    }
    console.log(built ? " deployed." : " still building (it usually finishes within a few minutes).");
  } else {
    console.log("The website files were already up to date.");
  }
} finally {
  rmSync(site, { recursive: true, force: true });
}

step("Check");
const latest = gh(["api", `repos/${repo}/releases/latest`, "-q", ".tag_name"], { capture: true }).stdout.trim();
if (latest !== tag) throw new Error(`GitHub's latest release is ${latest}, expected ${tag}`);
console.log(`${branding.productName} ${version} is live: ${branding.websiteUrl}`);
console.log(`Release: https://github.com/${repo}/releases/tag/${tag}`);
console.log("Open launchers on an older version show \"Update needed\" within about 15 minutes.");
