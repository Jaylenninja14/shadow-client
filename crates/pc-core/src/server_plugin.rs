//! ShadowCombat, the companion Paper plugin for your own server (source: server-plugin/).
//! Combat timing is decided by the server, so these rules live there; the launcher only
//! copies the jar into a server folder the player picks.

use std::path::{Path, PathBuf};

use crate::{util, Error, Result};

pub const FILE_NAME: &str = "ShadowCombat-1.0.0.jar";
const JAR: &[u8] = include_bytes!("../data/server/ShadowCombat-1.0.0.jar");

/// Copy the plugin into `<server>/plugins` (or into `dir` itself if it is a plugins folder).
/// Older ShadowCombat jars (and SunnyCombat, its old name) there are renamed to `.jar.old` so the
/// server loads only one.
pub fn install(dir: &Path) -> Result<PathBuf> {
    if !dir.is_dir() {
        return Err(Error::NotFound(dir.display().to_string()));
    }
    let is_plugins = dir.file_name().is_some_and(|n| n.eq_ignore_ascii_case("plugins"));
    let plugins = if is_plugins { dir.to_path_buf() } else { dir.join("plugins") };
    if !is_plugins && !plugins.is_dir() && !dir.join("server.properties").is_file() {
        return Err(Error::Invalid("That folder doesn't look like a Paper server (no plugins folder or server.properties). Pick the folder your server runs in.".into()));
    }
    std::fs::create_dir_all(&plugins)?;
    for entry in std::fs::read_dir(&plugins)? {
        let path = entry?.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        if (name.starts_with("ShadowCombat-") || name.starts_with("SunnyCombat-")) && name.ends_with(".jar") && name != FILE_NAME {
            std::fs::rename(&path, path.with_file_name(format!("{name}.old")))?;
        }
    }
    let dest = plugins.join(FILE_NAME);
    util::atomic_write(&dest, JAR)?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installs_into_server_plugins_folder_only() {
        let d = tempfile::tempdir().unwrap();
        assert!(install(d.path()).is_err(), "not a server folder");
        std::fs::write(d.path().join("server.properties"), "server-port=25565\n").unwrap();
        let p = install(d.path()).unwrap();
        assert_eq!(p, d.path().join("plugins").join(FILE_NAME));
        assert_eq!(std::fs::read(&p).unwrap(), JAR);
        // an older version is set aside, our file replaced in place
        std::fs::write(d.path().join("plugins/ShadowCombat-0.9.0.jar"), b"old").unwrap();
        std::fs::write(d.path().join("plugins/SunnyCombat-1.0.0.jar"), b"old name").unwrap();
        install(&d.path().join("plugins")).unwrap();
        assert!(d.path().join("plugins/ShadowCombat-0.9.0.jar.old").is_file());
        assert!(!d.path().join("plugins/ShadowCombat-0.9.0.jar").exists());
        assert!(d.path().join("plugins/SunnyCombat-1.0.0.jar.old").is_file(), "the old-name plugin is set aside too");
    }

    #[test]
    fn embedded_jar_is_the_plugin() {
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(JAR)).unwrap();
        let mut yml = String::new();
        std::io::Read::read_to_string(&mut zip.by_name("plugin.yml").unwrap(), &mut yml).unwrap();
        assert!(yml.contains("name: ShadowCombat") && yml.contains("version: 1.0.0"));
    }
}
