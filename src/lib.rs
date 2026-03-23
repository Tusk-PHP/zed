use std::{
    fs,
    path::{Component, Path, PathBuf},
};
use zed_extension_api::{
    self as zed,
    settings::LspSettings,
    LanguageServerId, Result, SlashCommand, SlashCommandOutput, SlashCommandOutputSection, Worktree,
};

const EXTENSION_VERSION: &str = env!("CARGO_PKG_VERSION");
const LANGUAGE_SERVER_ID: &str = "tusk-php";

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

    fn language_server_binary_path(
        &mut self,
        _id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<String> {
        if let Some(path) = &self.cached_binary_path {
            if fs::metadata(path).map_or(false, |metadata| metadata.is_file()) {
                return Ok(path.clone());
            }
        }
        if let Some(path) = worktree.which("php-lsp") {
            self.cached_binary_path = Some(path.clone());
            return Ok(path);
        }

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
        let binary_path = format!("tusk-php-{EXTENSION_VERSION}/php-lsp{ext}");

        if !fs::metadata(&binary_path).map_or(false, |metadata| metadata.is_file()) {
            let url = format!(
                "https://github.com/open-southeners/php-lsp/releases/download/v{EXTENSION_VERSION}/tusk-php-{platform_name}-{arch_name}{ext}"
            );
            let _ = fs::create_dir_all(format!("tusk-php-{EXTENSION_VERSION}"));
            zed::download_file(&url, &binary_path, zed::DownloadedFileType::Uncompressed)?;
            zed::make_file_executable(&binary_path)?;
        }

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

        Ok(zed::Command {
            command: match configured_path {
                Some(path) => path,
                None => self.language_server_binary_path(id, worktree)?,
            },
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
