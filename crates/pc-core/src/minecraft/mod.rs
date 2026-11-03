pub mod detect;
pub mod install;
pub mod launch;
pub mod loaders;
pub mod manifest;
pub mod options;

/// Official endpoints. Kept in one struct so tests can point everything at a mock server.
#[derive(Debug, Clone)]
pub struct Endpoints {
    pub version_manifest: String,
    pub resources: String,
    pub java_runtime_all: String,
    pub fabric_meta: String,
    pub quilt_meta: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self {
            version_manifest: "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json".into(),
            resources: "https://resources.download.minecraft.net".into(),
            java_runtime_all: "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json".into(),
            fabric_meta: "https://meta.fabricmc.net".into(),
            quilt_meta: "https://meta.quiltmc.org".into(),
        }
    }
}

impl Endpoints {
    pub fn mock(base: &str) -> Self {
        Self {
            version_manifest: format!("{base}/mc/game/version_manifest_v2.json"),
            resources: format!("{base}/resources"),
            java_runtime_all: format!("{base}/java-runtime/all.json"),
            fabric_meta: format!("{base}/fabric"),
            quilt_meta: format!("{base}/quilt"),
        }
    }
}
