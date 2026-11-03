//! Java runtime discovery, per-version selection and Mojang runtime download.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};

use crate::context::Ctx;
use crate::download::DownloadItem;
use crate::{safepath, util, Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JavaInstall {
    /// Path to javaw.exe (preferred) or java.exe
    pub path: PathBuf,
    pub home: PathBuf,
    pub version: String,
    pub major: u32,
    pub vendor: Option<String>,
    pub arch: String,
    pub source: String,
}

/// Parse "1.8.0_491" → 8, "17.0.2" → 17, "21" → 21, "26.0.1+8" → 26.
pub fn parse_major(version: &str) -> Option<u32> {
    let v = version.trim().trim_matches('"');
    let mut parts = v.split(|c: char| c == '.' || c == '_' || c == '+' || c == '-');
    let first: u32 = parts.next()?.parse().ok()?;
    if first == 1 {
        return parts.next()?.parse().ok();
    }
    Some(first)
}

fn read_release(home: &Path) -> Option<HashMap<String, String>> {
    let text = std::fs::read_to_string(home.join("release")).ok()?;
    let mut map = HashMap::new();
    for line in text.lines() {
        if let Some((k, v)) = line.split_once('=') {
            map.insert(k.trim().to_string(), v.trim().trim_matches('"').to_string());
        }
    }
    Some(map)
}

/// Inspect a Java home directory. Uses the `release` file; if it is missing, runs
/// `java -XshowSettings:properties -version` (a local, already-installed binary).
pub fn inspect_home(home: &Path, source: &str) -> Option<JavaInstall> {
    let bin = home.join("bin");
    let exe = if bin.join("javaw.exe").is_file() {
        bin.join("javaw.exe")
    } else if bin.join("java.exe").is_file() {
        bin.join("java.exe")
    } else if bin.join("java").is_file() {
        bin.join("java")
    } else {
        return None;
    };
    let (version, vendor, arch) = match read_release(home) {
        Some(r) => (
            r.get("JAVA_VERSION").or(r.get("JAVA_RUNTIME_VERSION")).cloned().unwrap_or_default(),
            r.get("IMPLEMENTOR").cloned(),
            r.get("OS_ARCH").cloned().unwrap_or_default(),
        ),
        None => probe(&bin)?,
    };
    let major = parse_major(&version)?;
    let arch = match arch.as_str() {
        "amd64" | "x86_64" | "x64" => "x64",
        "x86" | "i386" | "i586" => "x86",
        "aarch64" | "arm64" => "arm64",
        "" => "unknown",
        _ => "unknown",
    }
    .to_string();
    Some(JavaInstall { path: exe, home: home.to_path_buf(), version, major, vendor, arch, source: source.into() })
}

fn probe(bin: &Path) -> Option<(String, Option<String>, String)> {
    let java = if bin.join("java.exe").is_file() { bin.join("java.exe") } else { bin.join("java") };
    let mut cmd = std::process::Command::new(java);
    cmd.args(["-XshowSettings:properties", "-version"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let out = cmd.output().ok()?;
    let text = String::from_utf8_lossy(&out.stderr);
    let get = |k: &str| text.lines().find_map(|l| l.trim().strip_prefix(&format!("{k} = ")).map(|s| s.trim().to_string()));
    Some((get("java.version")?, get("java.vendor"), get("os.arch").unwrap_or_default()))
}

fn subdirs(p: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(p).map(|rd| rd.flatten().filter(|e| e.path().is_dir()).map(|e| e.path()).collect()).unwrap_or_default()
}

/// Official-launcher style runtime layout: `<runtime>/<component>/<platform>/<component>/`.
fn mojang_layout_homes(runtime_root: &Path) -> Vec<PathBuf> {
    let mut homes = vec![];
    for comp in subdirs(runtime_root) {
        for platform in subdirs(&comp) {
            for inner in subdirs(&platform) {
                homes.push(inner);
            }
        }
        homes.push(comp); // our own flat layout: <runtime>/<component>/bin
    }
    homes
}

#[cfg(windows)]
fn registry_homes() -> Vec<PathBuf> {
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY};
    use winreg::RegKey;
    let mut out = vec![];
    let roots = [
        r"SOFTWARE\JavaSoft\JDK",
        r"SOFTWARE\JavaSoft\JRE",
        r"SOFTWARE\JavaSoft\Java Runtime Environment",
        r"SOFTWARE\JavaSoft\Java Development Kit",
        r"SOFTWARE\Eclipse Adoptium\JDK",
        r"SOFTWARE\Eclipse Adoptium\JRE",
        r"SOFTWARE\Eclipse Foundation\JDK",
        r"SOFTWARE\Microsoft\JDK",
        r"SOFTWARE\Azul Systems\Zulu",
        r"SOFTWARE\BellSoft\Liberica",
        r"SOFTWARE\Amazon Corretto",
    ];
    for flags in [KEY_READ | KEY_WOW64_64KEY, KEY_READ | KEY_WOW64_32KEY] {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        for root in roots {
            let Ok(key) = hklm.open_subkey_with_flags(root, flags) else { continue };
            collect_reg(&key, 0, &mut out);
        }
    }
    out
}

#[cfg(windows)]
fn collect_reg(key: &winreg::RegKey, depth: u32, out: &mut Vec<PathBuf>) {
    for name in ["JavaHome", "Path", "InstallationPath"] {
        if let Ok(v) = key.get_value::<String, _>(name) {
            out.push(PathBuf::from(v));
        }
    }
    if depth >= 4 {
        return;
    }
    for sub in key.enum_keys().flatten() {
        if let Ok(k) = key.open_subkey(&sub) {
            collect_reg(&k, depth + 1, out);
        }
    }
}

#[cfg(not(windows))]
fn registry_homes() -> Vec<PathBuf> {
    vec![]
}

/// Discover Java installations. `extra_mc_dirs` are additional `.minecraft`-like folders whose
/// `runtime` subfolder should be searched.
pub fn discover(data_runtime: &Path, extra_mc_dirs: &[PathBuf]) -> Vec<JavaInstall> {
    let mut candidates: Vec<(PathBuf, &str)> = vec![];
    for h in mojang_layout_homes(data_runtime) {
        candidates.push((h, "shadow-client"));
    }
    let mut mc_dirs = vec![crate::paths::default_minecraft_dir()];
    mc_dirs.extend(extra_mc_dirs.iter().cloned());
    for d in &mc_dirs {
        for h in mojang_layout_homes(&d.join("runtime")) {
            candidates.push((h, "official-launcher"));
        }
    }
    if let Some(local) = dirs::data_local_dir() {
        let store = local.join("Packages").join("Microsoft.4297127D64EC6_8wekyb3d8bbwe").join("LocalCache").join("Local").join("runtime");
        for h in mojang_layout_homes(&store) {
            candidates.push((h, "official-launcher"));
        }
    }
    for h in registry_homes() {
        candidates.push((h, "registry"));
    }
    for pf in [std::env::var_os("ProgramFiles"), std::env::var_os("ProgramFiles(x86)"), std::env::var_os("ProgramW6432")].into_iter().flatten() {
        let pf = PathBuf::from(pf);
        for vendor in ["Java", "Eclipse Adoptium", "Eclipse Foundation", "Microsoft", "Zulu", "BellSoft", "Amazon Corretto", "Semeru", "OpenJDK"] {
            for h in subdirs(&pf.join(vendor)) {
                candidates.push((h, "installed"));
            }
        }
    }
    if let Some(jh) = std::env::var_os("JAVA_HOME") {
        candidates.push((PathBuf::from(jh), "JAVA_HOME"));
    }
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            if dir.to_string_lossy().to_ascii_lowercase().contains("javapath") {
                continue; // Oracle's redirect shim; the real install is found above
            }
            if dir.join("java.exe").is_file() {
                if let Some(home) = dir.parent() {
                    candidates.push((home.to_path_buf(), "PATH"));
                }
            }
        }
    }
    if let Some(home) = dirs::home_dir() {
        // CurseForge app keeps Mojang runtimes in the official-launcher layout.
        for h in mojang_layout_homes(&home.join("curseforge").join("minecraft").join("Install").join("runtime")) {
            candidates.push((h, "curseforge"));
        }
        for h in subdirs(&home.join(".lunarclient").join("jre")) {
            candidates.push((h.clone(), "lunar-client"));
            for inner in subdirs(&h) {
                candidates.push((inner, "lunar-client"));
            }
        }
    }

    let mut seen = std::collections::HashSet::new();
    let mut out = vec![];
    for (home, source) in candidates {
        let key = crate::paths::canonical_or_self(&home).to_string_lossy().to_ascii_lowercase();
        if !seen.insert(key) {
            continue;
        }
        if let Some(j) = inspect_home(&home, source) {
            out.push(j);
        }
    }
    out.sort_by(|a, b| b.major.cmp(&a.major).then(a.source.cmp(&b.source)));
    out
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum JavaChoice {
    Found { java: JavaInstall, note: Option<String> },
    /// No suitable runtime; the official Mojang runtime `component` can be downloaded.
    NeedsDownload { required_major: u32, component: Option<String>, message: String },
}

/// Can runtime `major` run a game that requires `required`?
pub fn compatible(major: u32, required: u32) -> bool {
    if required <= 8 {
        // Pre-1.17 versions (LaunchWrapper, old LWJGL) break on newer Java.
        major == 8
    } else {
        major >= required
    }
}

/// Choose a Java runtime. `explicit` is a profile/settings override.
pub fn select(installs: &[JavaInstall], required: u32, component: Option<&str>, explicit: Option<&Path>) -> Result<JavaChoice> {
    if let Some(p) = explicit {
        let home = if p.is_file() { p.parent().and_then(|b| b.parent()).map(Path::to_path_buf) } else { Some(p.to_path_buf()) };
        let j = home.as_deref().and_then(|h| inspect_home(h, "custom")).ok_or_else(|| Error::Invalid(format!("{} is not a usable Java installation", p.display())))?;
        if j.major < required {
            return Err(Error::Incompatible(format!(
                "The selected Java {} is too old: this Minecraft version needs Java {required}. Choose 'Automatic' or a newer runtime.",
                j.major
            )));
        }
        let note = (!compatible(j.major, required)).then(|| format!("Java {} was chosen manually; this version expects Java {required} and may not start.", j.major));
        return Ok(JavaChoice::Found { java: j, note });
    }
    let x64_first = |a: &&JavaInstall| if a.arch == "x86" { 1 } else { 0 };
    // 1) exact major, Mojang/our runtime of the right component first
    let mut exact: Vec<&JavaInstall> = installs.iter().filter(|j| j.major == required && j.arch != "x86").collect();
    exact.sort_by_key(|j| {
        let comp_match = component.is_some_and(|c| j.home.to_string_lossy().contains(c));
        let src = match j.source.as_str() {
            "shadow-client" => 0,
            "official-launcher" => 1,
            "installed" | "registry" => 2,
            _ => 3,
        };
        (if comp_match { 0 } else { 1 }, src)
    });
    if let Some(j) = exact.first() {
        return Ok(JavaChoice::Found { java: (*j).clone(), note: None });
    }
    // 2) for Java 17+ versions a newer LTS-or-later runtime normally works; prefer the closest.
    if required > 8 {
        let mut newer: Vec<&JavaInstall> = installs.iter().filter(|j| j.major > required).collect();
        newer.sort_by_key(|j| (x64_first(j), j.major));
        if let Some(j) = newer.first() {
            return Ok(JavaChoice::Found {
                java: (*j).clone(),
                note: Some(format!(
                    "Using Java {} because Java {required} is not installed. Most versions run fine; if mods fail to load, download the official Java {required} runtime.",
                    j.major
                )),
            });
        }
    }
    Ok(JavaChoice::NeedsDownload {
        required_major: required,
        component: component.map(String::from),
        message: format!("Java {required} is required for this Minecraft version and was not found."),
    })
}

#[derive(Deserialize)]
struct RuntimeEntry {
    manifest: RuntimeManifestRef,
    version: RuntimeVersion,
}
#[derive(Deserialize)]
struct RuntimeManifestRef {
    sha1: Option<String>,
    size: Option<u64>,
    url: String,
}
#[derive(Deserialize)]
struct RuntimeVersion {
    name: String,
}
#[derive(Deserialize)]
struct RuntimeFiles {
    files: HashMap<String, RuntimeFile>,
}
#[derive(Deserialize)]
struct RuntimeFile {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    downloads: Option<HashMap<String, RuntimeDownload>>,
}
#[derive(Deserialize)]
struct RuntimeDownload {
    sha1: Option<String>,
    size: Option<u64>,
    url: String,
}

fn platform_key() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "windows-arm64",
        "x86" => "windows-x86",
        _ => "windows-x64",
    }
}

/// Download the official Mojang Java runtime `component` into the data folder.
pub async fn download_runtime(ctx: &Ctx, component: &str) -> Result<JavaInstall> {
    if !component.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(Error::Invalid(format!("bad runtime component {component}")));
    }
    let all: HashMap<String, HashMap<String, Vec<RuntimeEntry>>> =
        ctx.http.get_json(&ctx.api.minecraft.java_runtime_all, Some(Duration::from_secs(3600)), HeaderMap::new()).await?;
    let entry = all
        .get(platform_key())
        .and_then(|p| p.get(component))
        .and_then(|v| v.first())
        .ok_or_else(|| Error::NotFound(format!("Mojang does not publish runtime {component} for this platform")))?;
    let manifest_path = ctx.paths.tmp_dir().join(format!("runtime-{component}.json"));
    crate::download::fetch_to_file(
        &ctx.http,
        &DownloadItem::new(entry.manifest.url.clone(), manifest_path.clone()).sha1(entry.manifest.sha1.clone()).size(entry.manifest.size),
    )
    .await?;
    let files: RuntimeFiles = util::read_json(&manifest_path)?;
    let root = ctx.paths.runtimes_dir().join(component);
    let mut items = vec![];
    for (rel, f) in &files.files {
        let dest = safepath::safe_join(&root, rel)?;
        match f.kind.as_str() {
            "directory" => std::fs::create_dir_all(&dest)?,
            "file" => {
                let raw = f.downloads.as_ref().and_then(|d| d.get("raw")).ok_or_else(|| Error::Invalid(format!("runtime file {rel} has no download")))?;
                items.push(DownloadItem::new(raw.url.clone(), dest).sha1(raw.sha1.clone()).size(raw.size));
            }
            _ => {} // links are not used on Windows
        }
    }
    ctx.downloads.run(&format!("Java runtime {} ({component})", entry.version.name), items).await?;
    std::fs::write(root.join(".pc-runtime-version"), &entry.version.name)?;
    inspect_home(&root, "shadow-client").ok_or_else(|| Error::msg("The downloaded runtime could not be inspected"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_home(root: &Path, name: &str, version: &str, arch: &str) -> PathBuf {
        let h = root.join(name);
        std::fs::create_dir_all(h.join("bin")).unwrap();
        std::fs::write(h.join("bin").join("javaw.exe"), b"").unwrap();
        std::fs::write(h.join("release"), format!("JAVA_VERSION=\"{version}\"\nOS_ARCH=\"{arch}\"\nIMPLEMENTOR=\"Test\"\n")).unwrap();
        h
    }

    #[test]
    fn majors() {
        assert_eq!(parse_major("1.8.0_491"), Some(8));
        assert_eq!(parse_major("17.0.2"), Some(17));
        assert_eq!(parse_major("21"), Some(21));
        assert_eq!(parse_major("26.0.1+8-34"), Some(26));
        assert_eq!(parse_major("garbage"), None);
    }

    #[test]
    fn selection_rules() {
        let dir = tempfile::tempdir().unwrap();
        let j8 = inspect_home(&fake_home(dir.path(), "jre8", "1.8.0_491", "amd64"), "installed").unwrap();
        let j21 = inspect_home(&fake_home(dir.path(), "jdk21", "21.0.5", "amd64"), "installed").unwrap();
        let j21m = inspect_home(&fake_home(dir.path(), "java-runtime-delta", "21.0.7", "amd64"), "official-launcher").unwrap();
        let j26 = inspect_home(&fake_home(dir.path(), "jdk26", "26.0.1", "amd64"), "installed").unwrap();
        let all = vec![j8.clone(), j21.clone(), j21m.clone(), j26.clone()];

        // exact match, Mojang component preferred
        match select(&all, 21, Some("java-runtime-delta"), None).unwrap() {
            JavaChoice::Found { java, note } => {
                assert_eq!(java.home, j21m.home);
                assert!(note.is_none());
            }
            other => panic!("{other:?}"),
        }
        // legacy versions need exactly Java 8
        match select(&all, 8, Some("jre-legacy"), None).unwrap() {
            JavaChoice::Found { java, .. } => assert_eq!(java.major, 8),
            other => panic!("{other:?}"),
        }
        match select(&[j21.clone(), j26.clone()], 8, None, None).unwrap() {
            JavaChoice::NeedsDownload { required_major, .. } => assert_eq!(required_major, 8),
            other => panic!("{other:?}"),
        }
        // Java 25 required, only 26 present ⇒ use it with a note
        match select(&all, 25, Some("java-runtime-epsilon"), None).unwrap() {
            JavaChoice::Found { java, note } => {
                assert_eq!(java.major, 26);
                assert!(note.is_some());
            }
            other => panic!("{other:?}"),
        }
        // explicit too-old runtime is refused
        assert!(select(&all, 21, None, Some(&j8.home)).is_err());
        assert!(compatible(21, 17));
        assert!(!compatible(17, 8));
    }

    #[test]
    fn mojang_layout_is_found() {
        let dir = tempfile::tempdir().unwrap();
        let rt = dir.path().join("runtime");
        fake_home(&rt.join("java-runtime-delta").join("windows-x64"), "java-runtime-delta", "21.0.7", "amd64");
        let homes = mojang_layout_homes(&rt);
        assert!(homes.iter().any(|h| inspect_home(h, "x").is_some_and(|j| j.major == 21)));
    }
}
