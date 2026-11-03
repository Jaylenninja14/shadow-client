//! Mojang version manifest and version JSON parsing, rule evaluation, inheritance merging.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionManifest {
    pub latest: Latest,
    pub versions: Vec<ManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    pub time: String,
    pub release_time: String,
    pub sha1: Option<String>,
    pub compliance_level: Option<u32>,
}

impl VersionManifest {
    pub fn find(&self, id: &str) -> Option<&ManifestEntry> {
        self.versions.iter().find(|v| v.id == id)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionJson {
    pub id: String,
    #[serde(default)]
    pub inherits_from: Option<String>,
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub main_class: Option<String>,
    #[serde(default)]
    pub arguments: Option<Arguments>,
    #[serde(default)]
    pub minecraft_arguments: Option<String>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(default)]
    pub asset_index: Option<AssetIndexRef>,
    #[serde(default)]
    pub assets: Option<String>,
    #[serde(default)]
    pub downloads: Option<HashMap<String, Artifact>>,
    #[serde(default)]
    pub java_version: Option<JavaVersionReq>,
    #[serde(default)]
    pub logging: Option<HashMap<String, LoggingEntry>>,
    #[serde(default)]
    pub release_time: Option<String>,
    #[serde(default)]
    pub time: Option<String>,
    #[serde(default)]
    pub jar: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<Argument>,
    #[serde(default)]
    pub jvm: Vec<Argument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Argument {
    Plain(String),
    Conditional { rules: Vec<Rule>, value: StringOrVec },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StringOrVec {
    One(String),
    Many(Vec<String>),
}

impl StringOrVec {
    pub fn to_vec(&self) -> Vec<String> {
        match self {
            StringOrVec::One(s) => vec![s.clone()],
            StringOrVec::Many(v) => v.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub action: String,
    #[serde(default)]
    pub os: Option<OsRule>,
    #[serde(default)]
    pub features: Option<HashMap<String, bool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsRule {
    pub name: Option<String>,
    pub version: Option<String>,
    pub arch: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Library {
    pub name: String,
    #[serde(default)]
    pub downloads: Option<LibraryDownloads>,
    /// Maven repository base (Fabric/Quilt style libraries)
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub rules: Option<Vec<Rule>>,
    #[serde(default)]
    pub natives: Option<HashMap<String, String>>,
    #[serde(default)]
    pub extract: Option<Extract>,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub sha512: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LibraryDownloads {
    #[serde(default)]
    pub artifact: Option<Artifact>,
    #[serde(default)]
    pub classifiers: Option<HashMap<String, Artifact>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Extract {
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Artifact {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndexRef {
    pub id: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
    pub total_size: Option<u64>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersionReq {
    pub component: Option<String>,
    pub major_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingEntry {
    pub argument: String,
    pub file: LoggingFile,
    #[serde(rename = "type")]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingFile {
    pub id: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndex {
    #[serde(default)]
    pub objects: HashMap<String, AssetObject>,
    #[serde(default)]
    pub map_to_resources: bool,
    #[serde(rename = "virtual", default)]
    pub is_virtual: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

/// Environment the rules are evaluated against.
#[derive(Debug, Clone)]
pub struct RuleEnv {
    pub os_name: &'static str,
    pub os_version: String,
    pub arch: &'static str,
    pub features: HashMap<String, bool>,
}

impl RuleEnv {
    pub fn current() -> Self {
        let arch = match std::env::consts::ARCH {
            "x86_64" => "x86_64",
            "x86" => "x86",
            "aarch64" => "arm64",
            other => if other.contains("64") { "x86_64" } else { "x86" },
        };
        let os_name = match std::env::consts::OS {
            "windows" => "windows",
            "macos" => "osx",
            _ => "linux",
        };
        let os_version = sysinfo::System::kernel_version().unwrap_or_else(|| "10.0".into());
        Self { os_name, os_version, arch, features: HashMap::new() }
    }

    pub fn with_feature(mut self, name: &str, on: bool) -> Self {
        self.features.insert(name.to_string(), on);
        self
    }
}

fn os_rule_matches(os: &OsRule, env: &RuleEnv) -> bool {
    if let Some(name) = &os.name {
        if name != env.os_name {
            return false;
        }
    }
    if let Some(arch) = &os.arch {
        let ok = match arch.as_str() {
            "x86" => env.arch == "x86",
            "x86_64" | "amd64" => env.arch == "x86_64",
            "arm64" | "aarch64" => env.arch == "arm64",
            other => other == env.arch,
        };
        if !ok {
            return false;
        }
    }
    if let Some(ver) = &os.version {
        match regex::Regex::new(ver) {
            Ok(re) => {
                if !re.is_match(&env.os_version) {
                    return false;
                }
            }
            Err(_) => return false,
        }
    }
    true
}

/// Mojang semantics: no rules ⇒ allowed; otherwise the last matching rule decides,
/// starting from "disallowed".
pub fn rules_allow(rules: Option<&[Rule]>, env: &RuleEnv) -> bool {
    let Some(rules) = rules else { return true };
    if rules.is_empty() {
        return true;
    }
    let mut allowed = false;
    for rule in rules {
        let mut matches = true;
        if let Some(os) = &rule.os {
            matches &= os_rule_matches(os, env);
        }
        if let Some(features) = &rule.features {
            for (k, v) in features {
                let actual = env.features.get(k).copied().unwrap_or(false);
                if actual != *v {
                    matches = false;
                }
            }
        }
        if matches {
            allowed = rule.action == "allow";
        }
    }
    allowed
}

/// Maven coordinate parsing: `group:artifact:version[:classifier][@ext]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MavenCoord {
    pub group: String,
    pub artifact: String,
    pub version: String,
    pub classifier: Option<String>,
    pub ext: String,
}

impl MavenCoord {
    pub fn parse(name: &str) -> Result<Self> {
        let (main, ext) = match name.split_once('@') {
            Some((m, e)) => (m, e.to_string()),
            None => (name, "jar".to_string()),
        };
        let parts: Vec<&str> = main.split(':').collect();
        if parts.len() < 3 || parts.iter().any(|p| p.is_empty()) {
            return Err(Error::Invalid(format!("bad library name {name}")));
        }
        Ok(Self {
            group: parts[0].into(),
            artifact: parts[1].into(),
            version: parts[2].into(),
            classifier: parts.get(3).map(|s| s.to_string()),
            ext,
        })
    }

    pub fn path(&self) -> String {
        let file = match &self.classifier {
            Some(c) => format!("{}-{}-{}.{}", self.artifact, self.version, c, self.ext),
            None => format!("{}-{}.{}", self.artifact, self.version, self.ext),
        };
        format!("{}/{}/{}/{}", self.group.replace('.', "/"), self.artifact, self.version, file)
    }

    /// Identity used to deduplicate libraries across inherited version JSONs.
    pub fn dedup_key(&self) -> String {
        format!("{}:{}:{}", self.group, self.artifact, self.classifier.clone().unwrap_or_default())
    }
}

/// Merge a child version JSON (e.g. Fabric profile) onto its parent (vanilla).
pub fn merge(child: &VersionJson, parent: &VersionJson) -> VersionJson {
    let mut out = parent.clone();
    out.id = child.id.clone();
    out.inherits_from = None;
    if child.main_class.is_some() {
        out.main_class = child.main_class.clone();
    }
    if child.minecraft_arguments.is_some() {
        out.minecraft_arguments = child.minecraft_arguments.clone();
    }
    if let Some(ca) = &child.arguments {
        let mut args = out.arguments.clone().unwrap_or_default();
        args.game.extend(ca.game.iter().cloned());
        args.jvm.extend(ca.jvm.iter().cloned());
        out.arguments = Some(args);
    }
    // Child libraries take precedence over parent libraries with the same identity.
    let child_keys: std::collections::HashSet<String> = child
        .libraries
        .iter()
        .filter_map(|l| MavenCoord::parse(&l.name).ok().map(|c| c.dedup_key()))
        .collect();
    let mut libs = child.libraries.clone();
    libs.extend(parent.libraries.iter().filter(|l| match MavenCoord::parse(&l.name) {
        Ok(c) => !child_keys.contains(&c.dedup_key()),
        Err(_) => true,
    }).cloned());
    out.libraries = libs;
    if child.asset_index.is_some() {
        out.asset_index = child.asset_index.clone();
    }
    if child.assets.is_some() {
        out.assets = child.assets.clone();
    }
    if child.downloads.is_some() {
        out.downloads = child.downloads.clone();
    }
    if child.java_version.is_some() {
        out.java_version = child.java_version.clone();
    }
    if child.logging.is_some() {
        out.logging = child.logging.clone();
    }
    if child.kind.is_some() {
        out.kind = child.kind.clone();
    }
    if child.release_time.is_some() {
        out.release_time = child.release_time.clone();
    }
    // Fabric profiles reuse the vanilla jar: keep track of which jar to use.
    if out.jar.is_none() {
        out.jar = Some(parent.jar.clone().unwrap_or_else(|| parent.id.clone()));
    }
    out
}

/// Required Java major version for a version JSON. Falls back by release date for very old
/// JSONs that predate the `javaVersion` field.
pub fn required_java(v: &VersionJson) -> u32 {
    if let Some(j) = &v.java_version {
        return j.major_version;
    }
    8
}

/// Expand an argument list according to rules and substitute `${...}` placeholders.
pub fn expand_args(args: &[Argument], env: &RuleEnv, vars: &HashMap<&str, String>) -> Vec<String> {
    let mut out = Vec::new();
    for a in args {
        match a {
            Argument::Plain(s) => out.push(substitute(s, vars)),
            Argument::Conditional { rules, value } => {
                if rules_allow(Some(rules), env) {
                    for s in value.to_vec() {
                        out.push(substitute(&s, vars));
                    }
                }
            }
        }
    }
    out
}

pub fn substitute(s: &str, vars: &HashMap<&str, String>) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        if let Some(end) = after.find('}') {
            let key = &after[..end];
            match vars.get(key) {
                Some(v) => out.push_str(v),
                None => {
                    out.push_str("${");
                    out.push_str(key);
                    out.push('}');
                }
            }
            rest = &after[end + 1..];
        } else {
            out.push_str(&rest[start..]);
            rest = "";
        }
    }
    out.push_str(rest);
    out
}

/// Detect the mod loader of a (merged) version JSON from its main class and libraries.
pub fn detect_loader(v: &VersionJson) -> (&'static str, Option<String>) {
    let main = v.main_class.as_deref().unwrap_or("");
    let find = |prefix: &str| {
        v.libraries.iter().find_map(|l| {
            let c = MavenCoord::parse(&l.name).ok()?;
            if format!("{}:{}", c.group, c.artifact) == prefix {
                Some(c.version)
            } else {
                None
            }
        })
    };
    if main.contains("fabricmc") || find("net.fabricmc:fabric-loader").is_some() {
        return ("fabric", find("net.fabricmc:fabric-loader"));
    }
    if main.contains("quiltmc") || find("org.quiltmc:quilt-loader").is_some() {
        return ("quilt", find("org.quiltmc:quilt-loader"));
    }
    if v.libraries.iter().any(|l| l.name.starts_with("net.neoforged")) {
        return ("neoforge", find("net.neoforged:neoforge").or_else(|| find("net.neoforged.fancymodloader:loader")));
    }
    if main.contains("cpw.mods") || main.contains("minecraftforge") || v.libraries.iter().any(|l| l.name.starts_with("net.minecraftforge")) {
        return ("forge", find("net.minecraftforge:forge"));
    }
    ("vanilla", None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env() -> RuleEnv {
        RuleEnv { os_name: "windows", os_version: "10.0.26200".into(), arch: "x86_64", features: HashMap::new() }
    }

    #[test]
    fn rule_semantics() {
        let r: Vec<Rule> = serde_json::from_str(r#"[{"action":"allow"},{"action":"disallow","os":{"name":"osx"}}]"#).unwrap();
        assert!(rules_allow(Some(&r), &env()));
        let r: Vec<Rule> = serde_json::from_str(r#"[{"action":"allow","os":{"name":"osx"}}]"#).unwrap();
        assert!(!rules_allow(Some(&r), &env()));
        let r: Vec<Rule> = serde_json::from_str(r#"[{"action":"allow","os":{"arch":"x86"}}]"#).unwrap();
        assert!(!rules_allow(Some(&r), &env()));
        let r: Vec<Rule> = serde_json::from_str(r#"[{"action":"allow","os":{"name":"windows","version":"^10\\."}}]"#).unwrap();
        assert!(rules_allow(Some(&r), &env()));
        let r: Vec<Rule> = serde_json::from_str(r#"[{"action":"allow","features":{"has_custom_resolution":true}}]"#).unwrap();
        assert!(!rules_allow(Some(&r), &env()));
        assert!(rules_allow(Some(&r), &env().with_feature("has_custom_resolution", true)));
    }

    #[test]
    fn maven_paths() {
        let c = MavenCoord::parse("org.lwjgl:lwjgl:3.3.3:natives-windows").unwrap();
        assert_eq!(c.path(), "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-windows.jar");
        let c = MavenCoord::parse("net.fabricmc:intermediary:1.21.11").unwrap();
        assert_eq!(c.path(), "net/fabricmc/intermediary/1.21.11/intermediary-1.21.11.jar");
        let c = MavenCoord::parse("a.b:c:1.0@zip").unwrap();
        assert_eq!(c.path(), "a/b/c/1.0/c-1.0.zip");
        assert!(MavenCoord::parse("bad").is_err());
    }

    #[test]
    fn substitution_and_expansion() {
        let args: Vec<Argument> = serde_json::from_str(
            r#"["--username","${auth_player_name}",{"rules":[{"action":"allow","features":{"has_custom_resolution":true}}],"value":["--width","${resolution_width}"]},{"rules":[{"action":"allow","os":{"name":"osx"}}],"value":"-XstartOnFirstThread"}]"#,
        )
        .unwrap();
        let mut vars = HashMap::new();
        vars.insert("auth_player_name", "Steve".to_string());
        vars.insert("resolution_width", "1280".to_string());
        assert_eq!(expand_args(&args, &env(), &vars), vec!["--username", "Steve"]);
        let e = env().with_feature("has_custom_resolution", true);
        assert_eq!(expand_args(&args, &e, &vars), vec!["--username", "Steve", "--width", "1280"]);
        assert_eq!(substitute("${missing} ok", &vars), "${missing} ok");
    }

    #[test]
    fn merge_child_over_parent() {
        let parent: VersionJson = serde_json::from_str(
            r#"{"id":"1.21.11","mainClass":"net.minecraft.client.main.Main","type":"release",
                "arguments":{"game":["--a"],"jvm":["-cp","${classpath}"]},
                "libraries":[{"name":"org.ow2.asm:asm:9.6"},{"name":"com.mojang:brigadier:1.0"}],
                "javaVersion":{"component":"java-runtime-delta","majorVersion":21},
                "assetIndex":{"id":"27","url":"https://x/27.json"}}"#,
        )
        .unwrap();
        let child: VersionJson = serde_json::from_str(
            r#"{"id":"fabric-loader-0.17.3-1.21.11","inheritsFrom":"1.21.11","mainClass":"net.fabricmc.loader.impl.launch.knot.KnotClient",
                "arguments":{"game":[],"jvm":["-DFabricMcEmu= net.minecraft.client.main.Main "]},
                "libraries":[{"name":"org.ow2.asm:asm:9.8","url":"https://maven.fabricmc.net/"},{"name":"net.fabricmc:fabric-loader:0.17.3","url":"https://maven.fabricmc.net/"}]}"#,
        )
        .unwrap();
        let m = merge(&child, &parent);
        assert_eq!(m.main_class.as_deref(), Some("net.fabricmc.loader.impl.launch.knot.KnotClient"));
        assert_eq!(m.libraries.iter().filter(|l| l.name.starts_with("org.ow2.asm:asm:")).count(), 1);
        assert!(m.libraries.iter().any(|l| l.name == "org.ow2.asm:asm:9.8"));
        assert_eq!(m.arguments.as_ref().unwrap().jvm.len(), 3);
        assert_eq!(required_java(&m), 21);
        assert_eq!(m.jar.as_deref(), Some("1.21.11"));
        assert_eq!(detect_loader(&m), ("fabric", Some("0.17.3".into())));
    }

    #[test]
    fn parses_real_shapes() {
        // Legacy JSON with minecraftArguments and old-style natives.
        let v: VersionJson = serde_json::from_str(
            r#"{"id":"1.8.9","minecraftArguments":"--username ${auth_player_name} --version ${version_name}","mainClass":"net.minecraft.client.main.Main",
                "libraries":[{"name":"org.lwjgl.lwjgl:lwjgl-platform:2.9.4-nightly-20150209","natives":{"windows":"natives-windows"},"extract":{"exclude":["META-INF/"]},
                "downloads":{"classifiers":{"natives-windows":{"path":"x.jar","sha1":"aa","size":1,"url":"https://libraries.minecraft.net/x.jar"}}}}],
                "assets":"1.8"}"#,
        )
        .unwrap();
        assert_eq!(required_java(&v), 8);
        assert!(v.libraries[0].natives.is_some());
    }
}
