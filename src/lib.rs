use std::fs;
use zed_extension_api::{self as zed, LanguageServerId, Result};

const EXTENSION_VERSION: &str = env!("CARGO_PKG_VERSION");

struct PhpLspExtension { cached_binary_path: Option<String> }

impl PhpLspExtension {
    fn language_server_binary_path(&mut self, _id: &LanguageServerId, worktree: &zed::Worktree) -> Result<String> {
        if let Some(path) = &self.cached_binary_path {
            if fs::metadata(path).map_or(false, |m| m.is_file()) { return Ok(path.clone()); }
        }
        if let Some(path) = worktree.which("php-lsp") {
            self.cached_binary_path = Some(path.clone());
            return Ok(path);
        }
        let (platform, arch) = zed::current_platform();
        let p = match platform { zed::Os::Mac => "darwin", zed::Os::Linux => "linux", zed::Os::Windows => "windows" };
        let a = match arch { zed::Architecture::Aarch64 => "arm64", zed::Architecture::X8664 => "amd64", _ => return Err("Unsupported arch".into()) };
        let ext = if platform == zed::Os::Windows { ".exe" } else { "" };
        let binary_path = format!("php-lsp-{EXTENSION_VERSION}/php-lsp{ext}");
        if !fs::metadata(&binary_path).map_or(false, |m| m.is_file()) {
            let url = format!(
                "https://github.com/open-southeners/php-lsp/releases/download/v{EXTENSION_VERSION}/php-lsp-{p}-{a}{ext}"
            );
            let _ = fs::create_dir_all(format!("php-lsp-{EXTENSION_VERSION}"));
            zed::download_file(&url, &binary_path, zed::DownloadedFileType::Uncompressed)?;
            zed::make_file_executable(&binary_path)?;
        }
        self.cached_binary_path = Some(binary_path.clone());
        Ok(binary_path)
    }
}

impl zed::Extension for PhpLspExtension {
    fn new() -> Self { Self { cached_binary_path: None } }
    fn language_server_command(&mut self, id: &LanguageServerId, worktree: &zed::Worktree) -> Result<zed::Command> {
        Ok(zed::Command { command: self.language_server_binary_path(id, worktree)?, args: vec!["--transport".into(), "stdio".into()], env: Default::default() })
    }
}

zed::register_extension!(PhpLspExtension);
