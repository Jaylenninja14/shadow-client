//! JVM arguments: small, explained presets and validation of user-supplied extras.
//!
//! Arguments are only ever accepted from the user (settings/profile), never from mod or pack
//! metadata, and are passed to Java as an argument vector without a shell.

use serde::Serialize;

use crate::{Error, Result};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArgPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub min_java: u32,
}

pub const PRESETS: &[ArgPreset] = &[
    ArgPreset {
        id: "mojang",
        name: "Mojang defaults (G1)",
        description: "The flags Mojang's own launcher uses: G1 collector with a 50 ms pause target. Good everywhere.",
        min_java: 8,
    },
    ArgPreset {
        id: "minimal",
        name: "Minimal",
        description: "Only the heap size. Lets the JVM pick everything else; useful for troubleshooting.",
        min_java: 8,
    },
    ArgPreset {
        id: "zgc",
        name: "Generational ZGC",
        description: "Very short GC pauses at the cost of more memory and CPU. Only worth it with 16 GB+ RAM and 6+ cores; requires Java 21+.",
        min_java: 21,
    },
];

/// Build the JVM flags for a preset (heap flags are added by the caller).
pub fn preset_args(preset: &str, java_major: u32) -> Vec<String> {
    match preset {
        "minimal" => vec![],
        "zgc" if java_major >= 21 => {
            let mut v = vec!["-XX:+UseZGC".to_string()];
            // Generational mode is opt-in on 21-22 and the only mode from 23.
            if java_major < 23 {
                v.push("-XX:+ZGenerational".into());
            }
            v
        }
        _ => {
            if java_major >= 9 || java_major == 8 {
                vec![
                    "-XX:+UnlockExperimentalVMOptions".into(),
                    "-XX:+UseG1GC".into(),
                    "-XX:G1NewSizePercent=20".into(),
                    "-XX:G1ReservePercent=20".into(),
                    "-XX:MaxGCPauseMillis=50".into(),
                    "-XX:G1HeapRegionSize=32M".into(),
                ]
            } else {
                vec![]
            }
        }
    }
}

/// Split a user argument string, honouring double quotes.
pub fn split_args(s: &str) -> Result<Vec<String>> {
    let mut out = vec![];
    let mut cur = String::new();
    let mut in_q = false;
    let mut has = false;
    for c in s.chars() {
        match c {
            '"' => {
                in_q = !in_q;
                has = true;
            }
            c if c.is_whitespace() && !in_q => {
                if has {
                    out.push(std::mem::take(&mut cur));
                    has = false;
                }
            }
            c => {
                cur.push(c);
                has = true;
            }
        }
    }
    if in_q {
        return Err(Error::Invalid("Unclosed quote in JVM arguments".into()));
    }
    if has {
        out.push(cur);
    }
    Ok(out)
}

/// Validate user extras and return the parsed list.
pub fn validate_user_args(s: &str) -> Result<Vec<String>> {
    let args = split_args(s)?;
    let bad = |a: &str, why: &str| Err(Error::Invalid(format!("JVM argument \"{a}\" is not allowed: {why}")));
    for a in &args {
        if a.chars().any(|c| c == '\0' || c == '\n' || c == '\r') {
            return bad(a, "contains control characters");
        }
        let lower = a.to_ascii_lowercase();
        if !a.starts_with('-') {
            return bad(a, "every argument must start with '-'");
        }
        if lower == "-jar" || lower == "-cp" || lower == "-classpath" || lower.starts_with("--class-path") || lower.starts_with("-djava.class.path") {
            return bad(a, "the launcher controls the classpath");
        }
        if lower.starts_with("-javaagent") || lower.starts_with("-agentlib") || lower.starts_with("-agentpath") || lower.starts_with("-xbootclasspath") {
            return bad(a, "loading agents or boot classes is blocked for safety");
        }
        if lower.starts_with("-xmx") || lower.starts_with("-xms") {
            return bad(a, "set memory with the memory sliders instead");
        }
        if lower.starts_with("-djava.library.path") {
            return bad(a, "the launcher controls native library paths");
        }
        let allowed = a.starts_with("-XX:")
            || a.starts_with("-D")
            || a.starts_with("-Xss")
            || a.starts_with("-Xlog")
            || a.starts_with("-Xmn")
            || a.starts_with("-Xshare")
            || a.starts_with("-verbose")
            || a.starts_with("--add-opens=")
            || a.starts_with("--add-exports=")
            || a.starts_with("--add-modules=")
            || a.starts_with("--enable-native-access=")
            || a.starts_with("--enable-preview")
            || a == "-ea"
            || a == "-da"
            || a == "-server"
            || a == "-client";
        if !allowed {
            return bad(a, "only -XX:, -D, -Xss/-Xmn/-Xlog/-Xshare, -verbose, --add-opens/--add-exports/--add-modules/--enable-native-access are accepted");
        }
        if a.starts_with("-XX:") {
            let body = &a[4..];
            let ok = regex::Regex::new(r"^[+-]?[A-Za-z][A-Za-z0-9_]*(=.+)?$").unwrap().is_match(body);
            if !ok {
                return bad(a, "malformed -XX option");
            }
        }
    }
    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_quotes() {
        assert_eq!(split_args(r#"-Dfoo="a b" -XX:+UseG1GC"#).unwrap(), vec!["-Dfoo=a b", "-XX:+UseG1GC"]);
        assert!(split_args(r#"-Dfoo="a"#).is_err());
        assert!(split_args("").unwrap().is_empty());
    }

    #[test]
    fn validation_blocks_dangerous() {
        assert!(validate_user_args("-XX:+UseG1GC -Dfile.encoding=UTF-8 -Xss2M").is_ok());
        for bad in ["-jar x.jar", "-javaagent:evil.jar", "-Xmx8G", "-cp foo", "calc.exe", "-XX:+Bad Option", "-Xbootclasspath/a:x", "& del"] {
            assert!(validate_user_args(bad).is_err(), "should reject {bad}");
        }
    }

    #[test]
    fn presets() {
        assert!(preset_args("mojang", 21).contains(&"-XX:+UseG1GC".to_string()));
        assert_eq!(preset_args("zgc", 21), vec!["-XX:+UseZGC", "-XX:+ZGenerational"]);
        assert_eq!(preset_args("zgc", 25), vec!["-XX:+UseZGC"]);
        // ZGC requested on Java 17 falls back to the G1 defaults
        assert!(preset_args("zgc", 17).contains(&"-XX:+UseG1GC".to_string()));
        assert!(preset_args("minimal", 21).is_empty());
    }
}
