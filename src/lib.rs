use sha2::{Digest, Sha256};
use std::fs;
use zed_extension_api::{self as zed, settings::LspSettings, LanguageServerId, Result, Worktree};

const LANGUAGE_SERVER_ID: &str = "tusk-php";
const PIN_TOML: &str = include_str!("../tusk-lsp.toml");

// ---------------------------------------------------------------------------
// tusk-lsp.toml pin-file parser (no toml crate — hand-rolled)
// ---------------------------------------------------------------------------

/// Returns the `version` value from the `[lsp]` section of PIN_TOML.
fn pinned_version() -> &'static str {
    let mut in_lsp_section = false;
    for raw in PIN_TOML.lines() {
        let line = raw.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            // "[lsp]" exactly — not "[lsp.sha256]"
            in_lsp_section = line == "[lsp]";
            continue;
        }
        if in_lsp_section {
            if let Some(rest) = line.strip_prefix("version") {
                let rest = rest.trim();
                if let Some(rest) = rest.strip_prefix('=') {
                    let value = rest.trim().trim_matches('"');
                    return value;
                }
            }
        }
    }
    // Fallback: should never happen if tusk-lsp.toml is well-formed.
    "0.0.0"
}

/// Returns the lowercase hex SHA-256 (WITHOUT `sha256:` prefix) for the given
/// `platform-arch` key (e.g. `"darwin-arm64"`), or `None` if not present.
fn pinned_sha(platform_arch: &str) -> Option<String> {
    let mut in_sha_section = false;
    for raw in PIN_TOML.lines() {
        let line = raw.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            in_sha_section = line == "[lsp.sha256]";
            continue;
        }
        if in_sha_section {
            // lines look like: darwin-arm64  = "sha256:aa2260ff..."
            let Some(eq_pos) = line.find('=') else {
                continue;
            };
            let key = line[..eq_pos].trim();
            if key == platform_arch {
                let value = line[eq_pos + 1..].trim().trim_matches('"');
                let hex = value
                    .strip_prefix("sha256:")
                    .unwrap_or(value)
                    .trim()
                    .to_lowercase();
                return Some(hex);
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// SHA-256 helper
// ---------------------------------------------------------------------------

fn sha256_hex(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    digest.iter().map(|b| format!("{:02x}", b)).collect()
}

// ---------------------------------------------------------------------------
// Extension struct
// ---------------------------------------------------------------------------

struct PhpLspExtension {
    cached_binary_path: Option<String>,
}

impl PhpLspExtension {
    fn lsp_settings(worktree: &Worktree) -> LspSettings {
        match zed::settings::LspSettings::for_worktree(LANGUAGE_SERVER_ID, worktree) {
            Ok(settings) => settings,
            Err(_) => LspSettings::default(),
        }
    }

    /// Core resolution logic.
    ///
    /// Precedence:
    ///   1. Configured `binary.path` — validated with `worktree.which`, which
    ///      runs on the worktree's host (SSH-aware) and resolves relative paths
    ///      against the worktree root. `fs::metadata` can't be used here: the
    ///      WASM sandbox only sees this extension's work dir. Skipped if it
    ///      doesn't exist on that host, so a user-level path to a local build
    ///      doesn't break SSH remotes. Result is NOT cached (would cause
    ///      cross-worktree leakage).
    ///   2. `worktree.which("tusk-php")` — authoritative, remote-safe PATH
    ///      lookup on the worktree's host (SSH-aware). Checked before the cache
    ///      so a stale client-local path is never forwarded to a different host.
    ///   3. Cached downloaded binary — a path this extension previously
    ///      downloaded into its own work dir; validated with `fs::metadata`.
    ///   4. Download the pinned release, verify SHA-256, exec.
    ///   5. Actionable error.
    fn language_server_binary_path(
        &mut self,
        _id: &LanguageServerId,
        worktree: &Worktree,
        configured_path: Option<String>,
    ) -> Result<String> {
        // 1. Configured binary.path — wins whenever it exists on this host.
        //    Not cached: caching here would leak a client-local absolute path
        //    to a subsequent SSH-remote worktree in the same session.
        let configured_path_missing = match configured_path {
            Some(raw) => match worktree.which(&raw) {
                Some(path) => return Ok(path),
                // Configured but not found on this host — fall through, remember the fact.
                None => true,
            },
            None => false,
        };

        // 2. Remote/local PATH lookup — authoritative and remote-safe. Runs
        //    before the cache so a client-local binary is never forwarded to
        //    an SSH remote worktree on a different host.
        if let Some(path) = worktree.which("tusk-php") {
            return Ok(path);
        }

        // 3. Cached downloaded binary — only ever holds a path this extension
        //    downloaded into its own work dir (never a client-local path).
        if let Some(path) = &self.cached_binary_path {
            if fs::metadata(path).map_or(false, |m| m.is_file()) {
                return Ok(path.clone());
            }
        }

        // 4. Download pinned release.
        let (platform, arch) = zed::current_platform();
        let platform_name = match platform {
            zed::Os::Mac => "darwin",
            zed::Os::Linux => "linux",
            zed::Os::Windows => "windows",
        };
        let arch_name = match arch {
            zed::Architecture::Aarch64 => "arm64",
            zed::Architecture::X8664 => "amd64",
            _ => return Err("Unsupported arch".into()),
        };
        let ext = if platform == zed::Os::Windows {
            ".exe"
        } else {
            ""
        };

        let version = pinned_version();
        let platform_arch = format!("{platform_name}-{arch_name}");

        let expected_sha = pinned_sha(&platform_arch)
            .ok_or_else(|| format!("no pinned checksum for {platform_arch} in tusk-lsp.toml"))?;

        let binary_path = format!("tusk-php-{version}/tusk-php{ext}");
        let url = format!(
            "https://github.com/Tusk-PHP/lsp/releases/download/{version}/tusk-php-{platform_name}-{arch_name}{ext}"
        );

        // Download if the file is not on disk yet.
        if !fs::metadata(&binary_path).map_or(false, |m| m.is_file()) {
            let _ = fs::create_dir_all(format!("tusk-php-{version}"));
            zed::download_file(&url, &binary_path, zed::DownloadedFileType::Uncompressed).map_err(
                |e| {
                    if configured_path_missing {
                        format!(
                            "tusk-php language server not found: configured binary.path does \
                             not exist on the target host, tusk-php is not on PATH, and the \
                             release download failed ({e}). Install tusk-php on the remote PATH \
                             or unset/correct lsp.tusk-php.binary.path."
                        )
                    } else {
                        format!(
                            "tusk-php language server not found: not on PATH and the release \
                             download failed ({e}). Install tusk-php on PATH, set \
                             lsp.tusk-php.binary.path, or ensure network access to GitHub \
                             releases."
                        )
                    }
                },
            )?;
        }

        // Verify SHA-256 (always — even for a pre-existing cached file).
        // If pre-existing file mismatches, re-download once and re-verify.
        let actual_sha = {
            let bytes =
                fs::read(&binary_path).map_err(|e| format!("failed to read {binary_path}: {e}"))?;
            sha256_hex(&bytes)
        };

        if !actual_sha.eq_ignore_ascii_case(&expected_sha) {
            // Remove the bad file and try one fresh download.
            let _ = fs::remove_file(&binary_path);
            let _ = fs::create_dir_all(format!("tusk-php-{version}"));
            zed::download_file(&url, &binary_path, zed::DownloadedFileType::Uncompressed)
                .map_err(|e| format!("re-download after checksum mismatch failed: {e}"))?;

            let bytes = fs::read(&binary_path)
                .map_err(|e| format!("failed to read {binary_path} after re-download: {e}"))?;
            let retry_sha = sha256_hex(&bytes);

            if !retry_sha.eq_ignore_ascii_case(&expected_sha) {
                let _ = fs::remove_file(&binary_path);
                return Err(format!(
                    "checksum mismatch for downloaded tusk-php {version} ({platform_arch}): \
                     expected {expected_sha}, got {retry_sha}"
                )
                .into());
            }
        }

        zed::make_file_executable(&binary_path)?;
        self.cached_binary_path = Some(binary_path.clone());
        Ok(binary_path)
    }
}

impl zed::Extension for PhpLspExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<zed::Command> {
        let settings = Self::lsp_settings(worktree);
        let (configured_path, configured_args) = match settings.binary {
            Some(binary) => (binary.path, binary.arguments),
            None => (None, None),
        };

        // The resolver checks configured_path on the worktree's host, so a
        // client-local absolute path is skipped on an SSH remote.
        let command = self.language_server_binary_path(id, worktree, configured_path)?;

        Ok(zed::Command {
            command,
            args: configured_args.unwrap_or_else(|| vec!["--transport".into(), "stdio".into()]),
            env: Default::default(),
        })
    }

    fn language_server_initialization_options(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        Ok(Self::lsp_settings(worktree).initialization_options)
    }

    fn language_server_workspace_configuration(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        Ok(Self::lsp_settings(worktree).settings)
    }
}

zed::register_extension!(PhpLspExtension);
