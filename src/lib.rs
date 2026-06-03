use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};
use zed_extension_api::{
    self as zed,
    settings::LspSettings,
    LanguageServerId, Result, SlashCommand, SlashCommandOutput, SlashCommandOutputSection, Worktree,
};

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
    ///   1. Cached path (still exists on disk as a file)
    ///   2. `worktree.which("tusk-php")` — remote/local PATH
    ///   3. Configured `binary.path` — resolved relative to worktree root when
    ///      not absolute; validated with `fs::metadata`; skipped if missing.
    ///   4. Download the pinned release, verify SHA-256, exec.
    ///   5. Actionable error.
    fn language_server_binary_path(
        &mut self,
        _id: &LanguageServerId,
        worktree: &Worktree,
        configured_path: Option<String>,
    ) -> Result<String> {
        // 1. Cached path.
        if let Some(path) = &self.cached_binary_path {
            if fs::metadata(path).map_or(false, |m| m.is_file()) {
                return Ok(path.clone());
            }
        }

        // 2. Remote/local PATH lookup — wins over any configured path so that
        //    a client-local absolute path is NOT forwarded to an SSH remote.
        if let Some(path) = worktree.which("tusk-php") {
            self.cached_binary_path = Some(path.clone());
            return Ok(path);
        }

        // 3. Configured binary.path — validate existence before trusting it.
        let configured_path_missing = if let Some(raw) = configured_path {
            let resolved: PathBuf = {
                let p = PathBuf::from(&raw);
                if p.is_absolute() {
                    p
                } else {
                    PathBuf::from(worktree.root_path()).join(p)
                }
            };
            if fs::metadata(&resolved).map_or(false, |m| m.is_file()) {
                let s = resolved.to_string_lossy().into_owned();
                self.cached_binary_path = Some(s.clone());
                return Ok(s);
            }
            // Configured but not found on this host — fall through, remember the fact.
            true
        } else {
            false
        };

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
        let ext = if platform == zed::Os::Windows { ".exe" } else { "" };

        let version = pinned_version();
        let platform_arch = format!("{platform_name}-{arch_name}");

        let expected_sha = pinned_sha(&platform_arch).ok_or_else(|| {
            format!("no pinned checksum for {platform_arch} in tusk-lsp.toml")
        })?;

        let binary_path = format!("tusk-php-{version}/tusk-php{ext}");
        let url = format!(
            "https://github.com/Tusk-PHP/lsp/releases/download/{version}/tusk-php-{platform_name}-{arch_name}{ext}"
        );

        // Download if the file is not on disk yet.
        if !fs::metadata(&binary_path).map_or(false, |m| m.is_file()) {
            let _ = fs::create_dir_all(format!("tusk-php-{version}"));
            zed::download_file(&url, &binary_path, zed::DownloadedFileType::Uncompressed)
                .map_err(|e| {
                    if configured_path_missing {
                        format!(
                            "tusk-php language server not found: not on PATH, configured \
                             binary.path does not exist on the target host, and the release \
                             download failed ({e}). Install tusk-php on the remote PATH or \
                             unset/correct lsp.tusk-php.binary.path."
                        )
                    } else {
                        format!(
                            "tusk-php language server not found: not on PATH and the release \
                             download failed ({e}). Install tusk-php manually or ensure network \
                             access to GitHub releases."
                        )
                    }
                })?;
        }

        // Verify SHA-256 (always — even for a pre-existing cached file).
        // If pre-existing file mismatches, re-download once and re-verify.
        let actual_sha = {
            let bytes = fs::read(&binary_path)
                .map_err(|e| format!("failed to read {binary_path}: {e}"))?;
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

    fn slash_output(label: String, text: String) -> SlashCommandOutput {
        SlashCommandOutput {
            text: text.clone(),
            sections: vec![SlashCommandOutputSection {
                range: (0..text.len()).into(),
                label,
            }],
        }
    }

    fn path_argument(args: &[String]) -> core::result::Result<String, String> {
        let path = args.join(" ").trim().to_string();
        if path.is_empty() {
            return Err("missing file path argument".to_string());
        }
        Ok(path)
    }

    fn worktree_relative_path(worktree: &Worktree, path: &str) -> core::result::Result<PathBuf, String> {
        let root = PathBuf::from(worktree.root_path());
        let candidate = PathBuf::from(path);
        let relative = if candidate.is_absolute() {
            candidate
                .strip_prefix(&root)
                .map_err(|_| format!("path must be inside {}", root.display()))?
                .to_path_buf()
        } else {
            candidate
        };

        Ok(Self::normalize_path(relative))
    }

    fn normalize_path(path: PathBuf) -> PathBuf {
        let mut normalized = PathBuf::new();
        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    normalized.pop();
                }
                other => normalized.push(other.as_os_str()),
            }
        }
        normalized
    }

    fn extract_namespace(source: &str) -> Option<String> {
        for line in source.lines() {
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix("namespace ") {
                let end = rest.find([';', '{']).unwrap_or(rest.len());
                let namespace = rest[..end].trim();
                if !namespace.is_empty() {
                    return Some(namespace.to_string());
                }
            }
        }

        None
    }

    fn extract_identifier(token: &str) -> Option<String> {
        let identifier: String = token
            .trim_start_matches('&')
            .chars()
            .take_while(|char| char.is_ascii_alphanumeric() || *char == '_')
            .collect();
        if identifier.is_empty() {
            None
        } else {
            Some(identifier)
        }
    }

    fn extract_primary_type(source: &str) -> Option<String> {
        for line in source.lines() {
            let tokens: Vec<_> = line.split_whitespace().collect();
            for (index, token) in tokens.iter().enumerate() {
                if !matches!(*token, "class" | "interface" | "trait" | "enum") {
                    continue;
                }
                if index > 0 && tokens[index - 1] == "new" {
                    continue;
                }
                if let Some(next) = tokens
                    .get(index + 1)
                    .and_then(|name| Self::extract_identifier(name))
                {
                    return Some(next);
                }
            }
        }

        None
    }

    fn declared_symbol(source: &str) -> Option<String> {
        let namespace = Self::extract_namespace(source).unwrap_or_default();
        let primary_type = Self::extract_primary_type(source);

        match (namespace.is_empty(), primary_type) {
            (_, Some(name)) if namespace.is_empty() => Some(name),
            (_, Some(name)) => Some(format!("{namespace}\\{name}")),
            (false, None) => Some(namespace),
            (true, None) => None,
        }
    }

    fn read_project_psr4(worktree: &Worktree) -> core::result::Result<Vec<(String, PathBuf)>, String> {
        let composer = worktree
            .read_text_file("composer.json")
            .map_err(|err| format!("failed to read composer.json: {err}"))?;
        let value: zed::serde_json::Value =
            zed::serde_json::from_str(&composer).map_err(|err| format!("invalid composer.json: {err}"))?;

        let mut mappings = Vec::new();
        for key in ["autoload", "autoload-dev"] {
            let Some(psr4) = value
                .get(key)
                .and_then(|block| block.get("psr-4"))
                .and_then(|psr4| psr4.as_object())
            else {
                continue;
            };

            for (namespace, paths) in psr4 {
                let namespace = namespace.trim_end_matches('\\').to_string();
                match paths {
                    zed::serde_json::Value::String(path) => {
                        mappings.push((namespace.clone(), Self::normalize_path(PathBuf::from(path))));
                    }
                    zed::serde_json::Value::Array(entries) => {
                        for entry in entries {
                            if let Some(path) = entry.as_str() {
                                mappings.push((namespace.clone(), Self::normalize_path(PathBuf::from(path))));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(mappings)
    }

    fn namespace_suffix(path: &Path) -> String {
        path.components()
            .filter_map(|component| {
                let part = component.as_os_str().to_string_lossy();
                if part == "." || part.is_empty() {
                    None
                } else {
                    Some(part.to_string())
                }
            })
            .collect::<Vec<_>>()
            .join("\\")
    }

    fn expected_namespace_for_path(
        worktree: &Worktree,
        relative_path: &Path,
    ) -> core::result::Result<String, String> {
        let mut best_match: Option<(usize, String)> = None;

        for (namespace, base_path) in Self::read_project_psr4(worktree)? {
            if !relative_path.starts_with(&base_path) {
                continue;
            }

            let remainder = relative_path
                .strip_prefix(&base_path)
                .map_err(|err| err.to_string())?;
            let parent = remainder.parent().unwrap_or_else(|| Path::new(""));
            let suffix = Self::namespace_suffix(parent);
            let candidate = if suffix.is_empty() {
                namespace
            } else {
                format!("{namespace}\\{suffix}")
            };
            let weight = base_path.components().count();

            if best_match
                .as_ref()
                .map_or(true, |(best_weight, _)| weight > *best_weight)
            {
                best_match = Some((weight, candidate));
            }
        }

        best_match
            .map(|(_, namespace)| namespace)
            .ok_or_else(|| format!("no PSR-4 mapping matched {}", relative_path.display()))
    }

    fn run_copy_namespace(
        args: Vec<String>,
        worktree: &Worktree,
    ) -> core::result::Result<SlashCommandOutput, String> {
        let path = Self::path_argument(&args)?;
        let relative_path = Self::worktree_relative_path(worktree, &path)?;
        let source = worktree
            .read_text_file(relative_path.to_string_lossy().as_ref())
            .map_err(|err| format!("failed to read {}: {err}", relative_path.display()))?;
        let symbol = Self::declared_symbol(&source)
            .ok_or_else(|| format!("no namespace or primary type found in {}", relative_path.display()))?;

        Ok(Self::slash_output(
            format!("Namespace: {}", relative_path.display()),
            symbol,
        ))
    }

    fn run_namespace_for_path(
        args: Vec<String>,
        worktree: &Worktree,
    ) -> core::result::Result<SlashCommandOutput, String> {
        let path = Self::path_argument(&args)?;
        let relative_path = Self::worktree_relative_path(worktree, &path)?;
        let namespace = Self::expected_namespace_for_path(worktree, &relative_path)?;

        Ok(Self::slash_output(
            format!("Expected namespace: {}", relative_path.display()),
            namespace,
        ))
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

        // Pass configured_path into the resolver so that PATH lookup wins over
        // a client-local absolute path that may not exist on an SSH remote.
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

    fn run_slash_command(
        &self,
        command: SlashCommand,
        args: Vec<String>,
        worktree: Option<&Worktree>,
    ) -> core::result::Result<SlashCommandOutput, String> {
        let worktree =
            worktree.ok_or_else(|| "slash commands require an open worktree".to_string())?;

        match command.name.as_str() {
            "tusk-copy-namespace" => Self::run_copy_namespace(args, worktree),
            "tusk-namespace-for-path" => Self::run_namespace_for_path(args, worktree),
            name => Err(format!("unknown slash command: \"{name}\"")),
        }
    }
}

zed::register_extension!(PhpLspExtension);
