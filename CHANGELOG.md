# Changelog

User-facing changes to the **Tusk PHP** Zed extension — the editor integration itself
(slash commands, language-server handling) and which **Tusk PHP language server** version
each release bundles.

For the language server's own changes — diagnostics, completions, hover, and Laravel /
Symfony intelligence — see the
[Tusk PHP LSP changelog](https://github.com/Tusk-PHP/lsp/blob/main/CHANGELOG.md). Each
release below links the server notes for the version it ships.

> Zed has no in-editor changelog view or update prompt, so this file is read on GitHub /
> the repository page rather than inside the editor.

This changelog is written for people installing the extension; it does not follow
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) strictly. The format is based on
[Semantic Versioning](https://semver.org/).

## [Unreleased]

- **Removed** the `/tusk-copy-namespace` and `/tusk-namespace-for-path` slash commands,
  which Zed no longer accepts in published extensions. Copy and move namespace come
  back as code actions with Tusk PHP language server 0.9.0.
- The language server now starts for **PHP** files again; it had been attached to
  `composer.json` only.
- `lsp.tusk-php.binary.path` is now used whenever that file exists on the machine the
  project is on, instead of being silently ignored in favour of the downloaded release
  binary. Over **Remote SSH**, a path that only exists locally is skipped and the
  extension falls back to `tusk-php` on the remote `PATH` or the downloaded release.
- Starting the server over **Remote SSH** no longer tries to run a binary from your local
  machine on the remote host.

## [0.9.0]

Bundles **Tusk PHP language server 0.8.0** —
[what's new in the server](https://github.com/Tusk-PHP/lsp/blob/main/CHANGELOG.md).

- **Namespace slash commands** — `/tusk-copy-namespace` shows a PHP file's declared
  namespace or FQN, and `/tusk-namespace-for-path` shows the namespace a path *should*
  have from your `composer.json` PSR-4 mappings.
- PHP and `composer.json` are routed to the language server for intelligence across both.
- **Zero-setup server** — the matching language-server binary is downloaded on first use
  and verified against the pinned SHA-256 sums before it runs.
