//! Shadow Client launcher engine.
//!
//! Everything that touches the network, the file system or the game process lives here so it
//! can be tested without the UI. The Tauri crate only translates IPC calls into these APIs.

pub mod auth;
pub mod bedrock;
pub mod bedrock_menu;
pub mod context;
pub mod download;
pub mod error;
pub mod events;
pub mod extras;
pub mod features;
pub mod http;
pub mod hud;
pub mod integration;
pub mod java;
pub mod jvmargs;
pub mod memory;
pub mod menu;
pub mod minecraft;
pub mod modpacks;
pub mod mods;
pub mod paths;
pub mod performance;
pub mod play;
pub mod profiles;
pub mod redact;
pub mod resources;
pub mod safepath;
pub mod secrets;
pub mod server_plugin;
pub mod servers;
pub mod settings;
pub mod skins;
pub mod setup;
pub mod share;
pub mod state;
pub mod util;

pub use error::{Error, Result};

#[cfg(test)]
mod tests_integration;

#[cfg(test)]
pub(crate) mod testutil {
    use std::path::Path;
    use std::sync::Arc;

    use crate::context::{ApiEndpoints, Ctx};
    use crate::events::{RecordingSink, Sink};
    use crate::paths::AppPaths;

    /// A context rooted in a temp dir with every API pointed at `base` (a mock server).
    pub fn ctx(dir: &Path, base: &str) -> (Ctx, Arc<RecordingSink>) {
        let rec = Arc::new(RecordingSink::default());
        let sink: Sink = rec.clone();
        let paths = AppPaths::new(dir.join("data"));
        let ctx = Ctx::new(paths, ApiEndpoints::mock(base), sink, "ShadowClient-test".into()).unwrap();
        (ctx, rec)
    }

    pub fn sha1(b: &[u8]) -> String {
        crate::util::sha1_hex(b)
    }

    pub fn sha512(b: &[u8]) -> String {
        use sha2::Digest;
        hex::encode(sha2::Sha512::digest(b))
    }
}

/// Version string reported in the User-Agent and the UI.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
