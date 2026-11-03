//! Compatibility database (bundled JSON) and conflict checks.

use std::collections::{BTreeSet, HashMap};
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatDb {
    pub version: u32,
    pub renderer_mods: RendererMods,
    pub conflicts: Vec<ConflictRule>,
    pub project_to_mod_id: HashMap<String, String>,
    pub performance_mods: Vec<PerformanceMod>,
    pub shader_support: ShaderSupport,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RendererMods {
    pub mod_ids: Vec<String>,
    pub project_ids: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConflictRule {
    pub a: String,
    pub b: String,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceMod {
    pub slug: String,
    pub project_id: String,
    pub name: String,
    pub role: String,
    pub summary: String,
    pub presets: Vec<String>,
    pub loaders: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShaderSupport {
    pub iris_project_id: String,
    pub light_shaders: Vec<LightShader>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LightShader {
    pub slug: String,
    pub project_id: String,
    pub name: String,
    pub note: String,
}

pub static DB: LazyLock<CompatDb> = LazyLock::new(|| serde_json::from_str(include_str!("../../data/compat-db.json")).expect("bundled compat-db.json is valid"));

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Conflict {
    pub a: String,
    pub b: String,
    pub reason: String,
    /// "error" blocks installation; "warning" is shown but allowed
    pub severity: String,
}

pub fn mod_id_for_project(project_id: &str) -> Option<&'static str> {
    DB.project_to_mod_id.get(project_id).map(String::as_str)
}

/// Detect conflicts within a set of mod ids (installed ∪ planned).
pub fn find_conflicts(mod_ids: &BTreeSet<String>) -> Vec<Conflict> {
    let mut out = vec![];
    for rule in &DB.conflicts {
        if mod_ids.contains(&rule.a) && mod_ids.contains(&rule.b) {
            out.push(Conflict { a: rule.a.clone(), b: rule.b.clone(), reason: rule.reason.clone(), severity: "error".into() });
        }
    }
    let renderers: Vec<&String> = DB.renderer_mods.mod_ids.iter().filter(|m| mod_ids.contains(*m)).collect();
    if renderers.len() > 1 {
        let already = out.iter().any(|c| renderers.contains(&&c.a) && renderers.contains(&&c.b));
        if !already {
            out.push(Conflict {
                a: renderers[0].clone(),
                b: renderers[1].clone(),
                reason: DB.renderer_mods.reason.clone(),
                severity: "error".into(),
            });
        }
    }
    out
}

/// Check a mod's own `breaks` declarations against the other mods.
pub fn declared_breaks(mod_id: &str, breaks: &std::collections::BTreeMap<String, Vec<String>>, others: &HashMap<String, String>) -> Vec<Conflict> {
    let mut out = vec![];
    for (dep, preds) in breaks {
        if let Some(ver) = others.get(dep) {
            let hit = super::metadata::matches_any(ver, preds).unwrap_or(true);
            if hit {
                out.push(Conflict {
                    a: mod_id.to_string(),
                    b: dep.clone(),
                    reason: format!("{mod_id} declares it is incompatible with {dep} {}", preds.join(" or ")),
                    severity: "error".into(),
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_loads_and_detects() {
        assert!(DB.performance_mods.iter().any(|m| m.slug == "sodium"));
        assert_eq!(mod_id_for_project("AANobbMI"), Some("sodium"));
        let set: BTreeSet<String> = ["sodium", "optifabric", "lithium"].iter().map(|s| s.to_string()).collect();
        let c = find_conflicts(&set);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].b, "optifabric");
        let set: BTreeSet<String> = ["embeddium", "canvas"].iter().map(|s| s.to_string()).collect();
        assert_eq!(find_conflicts(&set).len(), 1, "two renderer replacements conflict");
        let set: BTreeSet<String> = ["sodium", "lithium", "iris"].iter().map(|s| s.to_string()).collect();
        assert!(find_conflicts(&set).is_empty());
    }

    #[test]
    fn breaks_declarations() {
        let mut breaks = std::collections::BTreeMap::new();
        breaks.insert("sodium".to_string(), vec!["<0.6.0".to_string()]);
        let mut others = HashMap::new();
        others.insert("sodium".to_string(), "0.5.11".to_string());
        assert_eq!(declared_breaks("x", &breaks, &others).len(), 1);
        others.insert("sodium".to_string(), "0.6.13".to_string());
        assert!(declared_breaks("x", &breaks, &others).is_empty());
    }
}
