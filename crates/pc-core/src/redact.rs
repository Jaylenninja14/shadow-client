//! Secret redaction for logs, error messages and anything the user might share.

use std::sync::LazyLock;

use regex::Regex;

static PATTERNS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    vec![
        // --accessToken <token> on game command lines
        (Regex::new(r"(?i)(--accessToken\s+)\S+").unwrap(), "${1}[REDACTED]"),
        // key=value / "key": "value" pairs with sensitive names
        (
            Regex::new(r#"(?i)("?(?:access_?token|refresh_?token|id_?token|x-api-key|api_?key|client_?secret|password|auth_?session|session_?id)"?\s*[:=]\s*"?)[^"\s,}&]+"#).unwrap(),
            "${1}[REDACTED]",
        ),
        (Regex::new(r"(?i)(Bearer\s+)[A-Za-z0-9._\-]+").unwrap(), "${1}[REDACTED]"),
        (Regex::new(r"(?i)(XBL3\.0 x=)[^;\s]+;[A-Za-z0-9._\-]+").unwrap(), "${1}[REDACTED]"),
        (Regex::new(r"(?i)(token:)[A-Za-z0-9._\-]+(:[0-9a-f]{32})").unwrap(), "${1}[REDACTED]${2}"),
        // JWTs (three base64url segments starting with eyJ)
        (Regex::new(r"eyJ[A-Za-z0-9_\-]{8,}\.[A-Za-z0-9_\-]{8,}\.[A-Za-z0-9_\-]{8,}").unwrap(), "[REDACTED-JWT]"),
        // Microsoft refresh tokens (M.C5xx_ / M.R3_ prefixes)
        (Regex::new(r"M\.[A-Z][0-9A-Za-z]{1,4}_[A-Za-z0-9!*$\-_.]{20,}").unwrap(), "[REDACTED-MSA]"),
        // CurseForge keys are bcrypt-like strings beginning with $2a$10$
        (Regex::new(r"\$2[aby]\$\d{2}\$[A-Za-z0-9./]{20,}").unwrap(), "[REDACTED-KEY]"),
    ]
});

pub fn redact(input: &str) -> String {
    let mut out = input.to_string();
    for (re, rep) in PATTERNS.iter() {
        out = re.replace_all(&out, *rep).into_owned();
    }
    out
}

/// Additionally replace user-specific values (the Windows user name) for logs that will be shared.
pub fn redact_for_sharing(input: &str) -> String {
    let mut out = redact(input);
    if let Ok(user) = std::env::var("USERNAME") {
        if user.len() >= 3 {
            out = out.replace(&user, "<user>");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_tokens() {
        let s = r#"java -cp x Main --username Steve --accessToken abc.def.ghi --uuid 123"#;
        assert!(!redact(s).contains("abc.def.ghi"));
        assert!(redact("Authorization: Bearer eyJhbGciOi.xxx").contains("Bearer [REDACTED]"));
        assert!(!redact(r#"{"refresh_token":"M.C123_BAY.0.U.-secret-value-that-is-long"}"#).contains("secret-value"));
        assert!(!redact("x-api-key: $2a$10$abcdefghijklmnopqrstuvwxyz0123").contains("abcdefghijklmnop"));
        let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        assert_eq!(redact(jwt), "[REDACTED-JWT]");
        assert_eq!(redact("nothing secret here"), "nothing secret here");
    }
}
