//! Security module — path and shell command safety guards

use crate::SecurityResult;

const SECRET_BASENAME_PATTERNS: &[&str] = &[
    ".env",
    ".env.local",
    ".env.production",
    ".env.development",
    ".pem",
    ".key",
    ".p12",
    ".pfx",
    ".asc",
    ".gpg",
    ".keystore",
    ".jks",
    "id_rsa",
    "id_dsa",
    "id_ecdsa",
    "id_ed25519",
    "known_hosts",
    "authorized_keys",
    "htpasswd",
    ".netrc",
    "_netrc",
    "credentials",
    ".pgpass",
    ".npmrc",
    ".pypirc",
];

const PROTECTED_DIRS: &[&str] = &[
    ".ssh",
    ".gnupg",
    ".aws",
    ".azure",
    ".kube",
    ".docker",
    ".git",
    ".terraform.d",
];

pub struct SecurityGuard;

impl SecurityGuard {
    pub fn new() -> Self {
        Self
    }

    pub fn check_readable(&self, path: &str) -> SecurityResult {
        let path_obj = std::path::Path::new(path);

        if let Some(name) = path_obj.file_name().and_then(|n| n.to_str()) {
            for pattern in SECRET_BASENAME_PATTERNS {
                if name.to_lowercase().starts_with(pattern) || name.to_lowercase() == *pattern {
                    return SecurityResult {
                        ok: false,
                        reason: Some(format!(
                            "Refused: '{}' matches sensitive-file pattern '{}'",
                            name, pattern
                        )),
                    };
                }
            }
        }

        let path_lower = path.to_lowercase();
        for dir in PROTECTED_DIRS {
            if path_lower.contains(&format!("/{}/", dir))
                || path_lower.ends_with(&format!("/{}", dir))
            {
                return SecurityResult {
                    ok: false,
                    reason: Some(format!(
                        "Refused: path is inside protected directory '{}'",
                        dir
                    )),
                };
            }
        }

        SecurityResult {
            ok: true,
            reason: None,
        }
    }

    pub fn check_writable(&self, path: &str) -> SecurityResult {
        let read_result = self.check_readable(path);
        if !read_result.ok {
            return read_result;
        }

        let path_lower = path.to_lowercase();
        let write_deny_prefixes = [
            "/etc/",
            "/var/db/",
            "/var/root/",
            "/system/",
            "/usr/bin/",
            "/usr/sbin/",
            "/bin/",
            "/sbin/",
            "/boot/",
        ];

        for prefix in &write_deny_prefixes {
            if path_lower.starts_with(prefix) {
                return SecurityResult {
                    ok: false,
                    reason: Some(format!(
                        "Refused: writes under '{}' are not allowed",
                        prefix
                    )),
                };
            }
        }

        SecurityResult {
            ok: true,
            reason: None,
        }
    }

    pub fn check_shell_command(&self, cmd: &str) -> SecurityResult {
        let cmd_trimmed = cmd.trim();

        if cmd_trimmed.is_empty() {
            return SecurityResult {
                ok: false,
                reason: Some("Refused: empty command".to_string()),
            };
        }

        // GUI-05: Detect rm -rf / with proper tokenization (handles semicolons, pipes, &&)
        let normalized = cmd_trimmed
            .replace(';', " ")
            .replace("&&", " ")
            .replace("||", " ");
        let tokens: Vec<&str> = normalized.split_whitespace().collect();
        for i in 0..tokens.len() {
            if tokens[i] == "rm" && tokens[i + 1..].iter().any(|t| *t == "-rf" || *t == "-fr") {
                // Check if any token after the flags starts with /
                if tokens[i + 1..]
                    .iter()
                    .any(|t| *t == "/" || t.starts_with("/ "))
                {
                    return SecurityResult {
                        ok: false,
                        reason: Some(
                            "Refused: command attempts to recursively delete filesystem root"
                                .to_string(),
                        ),
                    };
                }
            }
        }

        if cmd_trimmed.contains(":(){ :|:& };:") || cmd_trimmed.contains(":(){:|:&};:") {
            return SecurityResult {
                ok: false,
                reason: Some("Refused: fork-bomb pattern detected".to_string()),
            };
        }

        if (cmd_trimmed.contains("curl") || cmd_trimmed.contains("wget"))
            && cmd_trimmed.contains("|")
            && (cmd_trimmed.contains(" sh") || cmd_trimmed.contains(" bash"))
        {
            return SecurityResult {
                ok: false,
                reason: Some("Refused: piping network download directly into shell".to_string()),
            };
        }

        SecurityResult {
            ok: true,
            reason: None,
        }
    }
}
